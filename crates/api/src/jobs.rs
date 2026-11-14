//! Background jobs with replayable event streams, cancellation and timeouts.

use std::collections::HashMap;
use std::future::{Future, ready};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use chrono::Utc;
use futures::stream::{self, BoxStream, StreamExt};
use hands_off_core::job::{JobEvent, JobRecord, JobStatus};
use hands_off_core::storage::{JobFailure, ProjectStore};
use hands_off_core::{Error, ErrorCode, JobId, Result};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_util::sync::CancellationToken;

const CHANNEL_CAPACITY: usize = 256;
const HISTORY_LIMIT: usize = 1024;
const DEFAULT_RETENTION: Duration = Duration::from_secs(600);

struct LiveJob {
    history: Mutex<Vec<JobEvent>>,
    sender: broadcast::Sender<JobEvent>,
    cancel: CancellationToken,
}

impl LiveJob {
    fn new() -> Self {
        Self {
            history: Mutex::new(Vec::new()),
            sender: broadcast::channel(CHANNEL_CAPACITY).0,
            cancel: CancellationToken::new(),
        }
    }

    fn history(&self) -> std::sync::MutexGuard<'_, Vec<JobEvent>> {
        self.history.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Record and broadcast an event. The lock is held across the send so that a concurrent
    /// subscriber sees every event exactly once, either in the replay or on the channel.
    fn emit(&self, event: JobEvent) {
        let mut history = self.history();
        if history.len() < HISTORY_LIMIT || event.is_terminal() {
            history.push(event.clone());
        }
        let _ = self.sender.send(event);
    }

    fn subscribe(&self) -> (Vec<JobEvent>, broadcast::Receiver<JobEvent>) {
        let history = self.history();
        (history.clone(), self.sender.subscribe())
    }
}

/// Handle given to a running job for reporting progress and observing cancellation.
#[derive(Clone)]
pub struct JobContext {
    job: Arc<LiveJob>,
}

impl JobContext {
    /// Announce that a pipeline stage started or advanced.
    pub fn progress(&self, stage: &str, message: &str) {
        self.job.emit(JobEvent::Progress {
            stage: stage.to_owned(),
            message: message.to_owned(),
        });
    }

    /// Append a line to the job's output log.
    pub fn log(&self, line: &str) {
        self.job.emit(JobEvent::Log {
            line: line.to_owned(),
        });
    }

    /// Publish an arbitrary event to the job's subscribers.
    pub fn event(&self, event: JobEvent) {
        self.job.emit(event);
    }

    /// Token that is cancelled when the user cancels the job.
    #[must_use]
    pub fn cancellation(&self) -> &CancellationToken {
        &self.job.cancel
    }
}

struct Inner {
    live: Mutex<HashMap<JobId, Arc<LiveJob>>>,
    store: Arc<dyn ProjectStore>,
    timeout: Duration,
    retention: Duration,
}

/// Creates, tracks and cancels background jobs.
#[derive(Clone)]
pub struct JobManager {
    inner: Arc<Inner>,
}

impl JobManager {
    /// A manager persisting job records to `store`, failing jobs that exceed `timeout`.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore>, timeout: Duration) -> Self {
        Self {
            inner: Arc::new(Inner {
                live: Mutex::new(HashMap::new()),
                store,
                timeout,
                retention: DEFAULT_RETENTION,
            }),
        }
    }

    /// Keep finished jobs' event history in memory for `retention` before dropping it.
    #[must_use]
    pub fn with_retention(self, retention: Duration) -> Self {
        let inner = &self.inner;
        Self {
            inner: Arc::new(Inner {
                live: Mutex::new(HashMap::new()),
                store: Arc::clone(&inner.store),
                timeout: inner.timeout,
                retention,
            }),
        }
    }

    /// Start `work` in the background and return its id immediately.
    ///
    /// # Errors
    /// Returns an error if the job record cannot be persisted.
    pub async fn spawn<F, Fut>(&self, kind: &str, work: F) -> Result<JobId>
    where
        F: FnOnce(JobContext) -> Fut + Send + 'static,
        Fut: Future<Output = Result<Option<serde_json::Value>>> + Send + 'static,
    {
        let id = JobId::new();
        let record = JobRecord {
            id,
            kind: kind.to_owned(),
            status: JobStatus::Running,
            error_code: None,
            error_message: None,
            created_at: Utc::now(),
            finished_at: None,
        };
        self.inner.store.insert_job(&record).await?;

        let job = Arc::new(LiveJob::new());
        self.live().insert(id, Arc::clone(&job));

        let manager = self.clone();
        tokio::spawn(async move {
            let context = JobContext {
                job: Arc::clone(&job),
            };
            let outcome = tokio::select! {
                biased;
                () = job.cancel.cancelled() => Err(Error::Cancelled),
                result = tokio::time::timeout(manager.inner.timeout, work(context)) => {
                    result.unwrap_or(Err(Error::Timeout))
                }
            };
            manager.finish(id, &job, outcome).await;
        });
        Ok(id)
    }

    /// Request cancellation. Succeeds silently if the job has already finished.
    ///
    /// # Errors
    /// [`Error::NotFound`] if the job is unknown.
    pub async fn cancel(&self, id: JobId) -> Result<()> {
        if let Some(job) = self.live().get(&id) {
            job.cancel.cancel();
            return Ok(());
        }
        self.status(id).await.map(|_| ())
    }

    /// The persisted record of a job.
    ///
    /// # Errors
    /// [`Error::NotFound`] if the job is unknown.
    pub async fn status(&self, id: JobId) -> Result<JobRecord> {
        self.inner
            .store
            .job_by_id(id)
            .await?
            .ok_or(Error::NotFound("job"))
    }

    /// A stream of the job's events: everything emitted so far, then live events, ending
    /// after the terminal event. Finished jobs whose history was dropped yield one
    /// synthesised terminal event.
    ///
    /// # Errors
    /// [`Error::NotFound`] if the job is unknown.
    pub async fn events(&self, id: JobId) -> Result<BoxStream<'static, JobEvent>> {
        let live = self.live().get(&id).cloned();
        let Some(job) = live else {
            let record = self.status(id).await?;
            return Ok(stream::once(ready(terminal_event(&record))).boxed());
        };

        let (history, receiver) = job.subscribe();
        let finished = history.iter().any(JobEvent::is_terminal);
        let live_events = if finished {
            stream::empty().boxed()
        } else {
            BroadcastStream::new(receiver)
                .filter_map(|item| ready(item.ok()))
                .boxed()
        };
        let merged = stream::iter(history).chain(live_events).boxed();
        Ok(
            stream::unfold((merged, false), |(mut merged, done)| async move {
                if done {
                    return None;
                }
                let event = merged.next().await?;
                let terminal = event.is_terminal();
                Some((event, (merged, terminal)))
            })
            .boxed(),
        )
    }

    fn live(&self) -> std::sync::MutexGuard<'_, HashMap<JobId, Arc<LiveJob>>> {
        self.inner
            .live
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    async fn finish(&self, id: JobId, job: &LiveJob, outcome: Result<Option<serde_json::Value>>) {
        let (status, event, failure) = match outcome {
            Ok(result) => (JobStatus::Completed, JobEvent::Completed { result }, None),
            Err(Error::Cancelled) => (JobStatus::Cancelled, JobEvent::Cancelled, None),
            Err(error) => {
                tracing::error!(job = %id, error = ?error, "job failed");
                let code = error.code();
                let message = error.to_string();
                (
                    JobStatus::Failed,
                    JobEvent::Failed {
                        code,
                        message: message.clone(),
                    },
                    Some(JobFailure { code, message }),
                )
            }
        };
        if let Err(error) = self.inner.store.finish_job(id, status, failure).await {
            tracing::error!(job = %id, error = ?error, "could not persist job result");
        }
        job.emit(event);

        let manager = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(manager.inner.retention).await;
            manager.live().remove(&id);
        });
    }
}

fn terminal_event(record: &JobRecord) -> JobEvent {
    match record.status {
        JobStatus::Completed => JobEvent::Completed { result: None },
        JobStatus::Cancelled => JobEvent::Cancelled,
        JobStatus::Failed => JobEvent::Failed {
            code: record.error_code.unwrap_or(ErrorCode::Internal),
            message: record
                .error_message
                .clone()
                .unwrap_or_else(|| "job failed".into()),
        },
        JobStatus::Running => JobEvent::Failed {
            code: ErrorCode::Internal,
            message: "job was interrupted".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::storage::SqliteStore;

    async fn manager(timeout: Duration) -> JobManager {
        let store: Arc<dyn ProjectStore> = Arc::new(SqliteStore::in_memory().await.unwrap());
        JobManager::new(store, timeout)
    }

    async fn collect(manager: &JobManager, id: JobId) -> Vec<JobEvent> {
        manager.events(id).await.unwrap().collect().await
    }

    #[tokio::test]
    async fn successful_job_streams_events_in_order_and_persists_completion() {
        let manager = manager(Duration::from_secs(5)).await;
        let id = manager
            .spawn("demo", |ctx| async move {
                ctx.progress("analysing", "looking");
                ctx.log("found 3 objects");
                Ok(Some(json!({"objects": 3})))
            })
            .await
            .unwrap();

        let events = collect(&manager, id).await;
        assert_eq!(events.len(), 3);
        assert!(matches!(&events[0], JobEvent::Progress { stage, .. } if stage == "analysing"));
        assert!(matches!(&events[1], JobEvent::Log { line } if line == "found 3 objects"));
        assert_eq!(
            events[2],
            JobEvent::Completed {
                result: Some(json!({"objects": 3}))
            }
        );
        assert_eq!(
            manager.status(id).await.unwrap().status,
            JobStatus::Completed
        );
    }

    #[tokio::test]
    async fn failing_job_reports_a_stable_code_and_hides_internal_detail() {
        let manager = manager(Duration::from_secs(5)).await;
        let id = manager
            .spawn("demo", |_| async {
                Err(Error::internal(std::io::Error::other("secret path")))
            })
            .await
            .unwrap();

        let events = collect(&manager, id).await;
        assert_eq!(
            events.last(),
            Some(&JobEvent::Failed {
                code: ErrorCode::Internal,
                message: "internal error".into()
            })
        );
        let record = manager.status(id).await.unwrap();
        assert_eq!(record.status, JobStatus::Failed);
        assert_eq!(record.error_code, Some(ErrorCode::Internal));
    }

    #[tokio::test]
    async fn cancellation_stops_a_running_job() {
        let manager = manager(Duration::from_secs(30)).await;
        let id = manager
            .spawn("demo", |ctx| async move {
                ctx.cancellation().cancelled().await;
                Ok(None)
            })
            .await
            .unwrap();
        manager.cancel(id).await.unwrap();

        let events = collect(&manager, id).await;
        assert_eq!(events.last(), Some(&JobEvent::Cancelled));
        assert_eq!(
            manager.status(id).await.unwrap().status,
            JobStatus::Cancelled
        );
    }

    #[tokio::test]
    async fn jobs_exceeding_the_timeout_fail_with_timeout() {
        let manager = manager(Duration::from_millis(50)).await;
        let id = manager
            .spawn("demo", |_| async {
                tokio::time::sleep(Duration::from_secs(30)).await;
                Ok(None)
            })
            .await
            .unwrap();

        let events = collect(&manager, id).await;
        assert!(matches!(
            events.last(),
            Some(JobEvent::Failed {
                code: ErrorCode::Timeout,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn late_subscribers_get_the_full_history_and_the_stream_ends() {
        let manager = manager(Duration::from_secs(5)).await;
        let id = manager
            .spawn("demo", |ctx| async move {
                ctx.log("one");
                Ok(None)
            })
            .await
            .unwrap();
        let first = collect(&manager, id).await;
        let second = collect(&manager, id).await;
        assert_eq!(first, second);
        assert!(first.last().unwrap().is_terminal());
    }

    #[tokio::test]
    async fn expired_jobs_yield_a_synthesised_terminal_event() {
        let manager = manager(Duration::from_secs(5))
            .await
            .with_retention(Duration::from_millis(10));
        let id = manager.spawn("demo", |_| async { Ok(None) }).await.unwrap();
        collect(&manager, id).await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        let events = collect(&manager, id).await;
        assert_eq!(events, [JobEvent::Completed { result: None }]);
    }

    #[tokio::test]
    async fn unknown_jobs_are_not_found() {
        let manager = manager(Duration::from_secs(5)).await;
        let id = JobId::new();
        assert!(matches!(
            manager.cancel(id).await.unwrap_err(),
            Error::NotFound(_)
        ));
        assert!(matches!(
            manager.status(id).await.unwrap_err(),
            Error::NotFound(_)
        ));
        assert!(matches!(
            manager.events(id).await.err().unwrap(),
            Error::NotFound(_)
        ));
    }

    #[tokio::test]
    async fn cancelling_a_finished_job_is_a_no_op() {
        let manager = manager(Duration::from_secs(5)).await;
        let id = manager.spawn("demo", |_| async { Ok(None) }).await.unwrap();
        collect(&manager, id).await;
        manager.cancel(id).await.unwrap();
        assert_eq!(
            manager.status(id).await.unwrap().status,
            JobStatus::Completed
        );
    }
}
