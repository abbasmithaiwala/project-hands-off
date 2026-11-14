//! Background job records and the events streamed to clients.

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::ErrorCode;
use crate::ids::JobId;

/// Lifecycle state of a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// Still running.
    Running,
    /// Finished successfully.
    Completed,
    /// Finished with an error.
    Failed,
    /// Stopped by the user.
    Cancelled,
}

impl JobStatus {
    /// Stable lowercase name used for storage.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Parse the storage name produced by [`JobStatus::as_str`].
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    /// Whether the job can no longer change state.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        self != Self::Running
    }
}

/// Persisted summary of a job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, JsonSchema)]
pub struct JobRecord {
    /// Unique identifier.
    pub id: JobId,
    /// What the job does, for example `analyze`.
    pub kind: String,
    /// Current state.
    pub status: JobStatus,
    /// Failure code when `status` is `failed`.
    pub error_code: Option<ErrorCode>,
    /// Failure message when `status` is `failed`.
    pub error_message: Option<String>,
    /// When the job was created.
    pub created_at: DateTime<Utc>,
    /// When the job reached a terminal state.
    pub finished_at: Option<DateTime<Utc>>,
}

/// An event streamed to clients while a job runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JobEvent {
    /// A pipeline stage started or advanced.
    Progress {
        /// Stage name, for example `generating`.
        stage: String,
        /// Human-readable description.
        message: String,
    },
    /// A line of reasoning or diagnostics for the output panel.
    Log {
        /// The log line.
        line: String,
    },
    /// The job finished successfully.
    Completed {
        /// Optional job-specific result payload.
        #[schema(value_type = Option<Object>)]
        result: Option<serde_json::Value>,
    },
    /// The job failed.
    Failed {
        /// Stable error code.
        code: ErrorCode,
        /// Human-readable message.
        message: String,
    },
    /// The job was cancelled.
    Cancelled,
}

impl JobEvent {
    /// Whether no further events follow this one.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed { .. } | Self::Failed { .. } | Self::Cancelled
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_running_is_non_terminal() {
        assert!(!JobStatus::Running.is_terminal());
        for status in [
            JobStatus::Completed,
            JobStatus::Failed,
            JobStatus::Cancelled,
        ] {
            assert!(status.is_terminal());
        }
    }

    #[test]
    fn status_names_round_trip() {
        for status in [
            JobStatus::Running,
            JobStatus::Completed,
            JobStatus::Failed,
            JobStatus::Cancelled,
        ] {
            assert_eq!(JobStatus::from_name(status.as_str()), Some(status));
        }
        assert_eq!(JobStatus::from_name("paused"), None);
    }

    #[test]
    fn terminal_events_are_identified() {
        assert!(!JobEvent::Log { line: "x".into() }.is_terminal());
        assert!(JobEvent::Cancelled.is_terminal());
        assert!(JobEvent::Completed { result: None }.is_terminal());
        assert!(
            JobEvent::Failed {
                code: ErrorCode::Timeout,
                message: "t".into()
            }
            .is_terminal()
        );
    }

    #[test]
    fn events_serialise_with_a_type_tag() {
        let json = serde_json::to_value(JobEvent::Progress {
            stage: "generating".into(),
            message: "working".into(),
        })
        .unwrap();
        assert_eq!(json["type"], "progress");
    }
}
