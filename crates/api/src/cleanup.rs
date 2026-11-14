//! Removing edit results nobody accepted.

use std::time::Duration;

use chrono::Utc;
use hands_off_core::Result;
use hands_off_core::storage::BlobKey;

use crate::state::AppState;

/// How long a finished job's result is kept for review before it is swept.
pub const JOB_RETENTION: Duration = Duration::from_secs(24 * 60 * 60);
const SWEEP_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// Delete the records and stored results of jobs finished longer ago than `retention`.
///
/// Returns how many jobs were removed.
///
/// # Errors
/// [`hands_off_core::Error::Internal`] if the store or blob storage fails.
pub async fn sweep_jobs(state: &AppState, retention: Duration) -> Result<usize> {
    let age = chrono::Duration::from_std(retention).unwrap_or(chrono::Duration::MAX);
    let cutoff = Utc::now().checked_sub_signed(age).unwrap_or(Utc::now());
    let expired = state.store.expire_jobs(cutoff).await?;
    for job in &expired {
        state.blobs.remove_tree(&BlobKey::job_dir(*job)).await?;
    }
    Ok(expired.len())
}

/// Sweep expired jobs once now and then hourly, for as long as the process runs.
pub fn spawn_sweeper(state: AppState) {
    tokio::spawn(async move {
        loop {
            match sweep_jobs(&state, JOB_RETENTION).await {
                Ok(0) => {}
                Ok(count) => tracing::info!(count, "removed expired edit results"),
                Err(error) => tracing::warn!(%error, "sweeping expired edit results failed"),
            }
            tokio::time::sleep(SWEEP_INTERVAL).await;
        }
    });
}
