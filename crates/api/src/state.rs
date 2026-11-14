//! Shared application state.

use std::sync::Arc;

use hands_off_core::storage::{BlobStore, ProjectStore};
use hands_off_core::{Config, Error, Result};
use tokio::fs;

use crate::detection::{DetectorProvider, LazyRfDetr};
use crate::jobs::JobManager;
use crate::storage::{FsBlobStore, SqliteStore};

/// Everything a request handler needs. Cheap to clone.
#[derive(Clone)]
pub struct AppState {
    /// Runtime configuration.
    pub config: Arc<Config>,
    /// Binary storage for images.
    pub blobs: Arc<dyn BlobStore>,
    /// Persistent records.
    pub store: Arc<dyn ProjectStore>,
    /// Background jobs.
    pub jobs: JobManager,
    /// Source of the object detector.
    pub detectors: Arc<dyn DetectorProvider>,
}

impl AppState {
    /// Assemble state from explicit parts (used by tests).
    #[must_use]
    pub fn new(config: Config, blobs: Arc<dyn BlobStore>, store: Arc<dyn ProjectStore>) -> Self {
        let jobs = JobManager::new(Arc::clone(&store), config.job_timeout);
        let detectors = Arc::new(LazyRfDetr::from_config(&config));
        Self {
            config: Arc::new(config),
            blobs,
            store,
            jobs,
            detectors,
        }
    }

    /// Replace the detector source (used by tests and custom setups).
    #[must_use]
    pub fn with_detectors(mut self, detectors: Arc<dyn DetectorProvider>) -> Self {
        self.detectors = detectors;
        self
    }

    /// Open the storage described by `config`, creating directories as needed.
    ///
    /// # Errors
    /// Returns [`Error::Internal`] if storage cannot be initialised.
    pub async fn from_config(config: Config) -> Result<Self> {
        fs::create_dir_all(&config.data_dir)
            .await
            .map_err(Error::internal)?;
        let blobs = FsBlobStore::new(config.data_dir.join("blobs")).await?;
        let store = SqliteStore::open(&config.data_dir.join("hands-off.db")).await?;
        Ok(Self::new(config, Arc::new(blobs), Arc::new(store)))
    }
}
