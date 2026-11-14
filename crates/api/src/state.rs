//! Shared application state.

use std::sync::Arc;

use hands_off_core::storage::{BlobStore, ProjectStore};
use hands_off_core::{Config, Error, Result};
use tokio::fs;

use hands_off_llm::demo::{DemoGenerator, DemoProvider};
use hands_off_llm::openrouter::OpenRouterProvider;
use hands_off_llm::{ImageGenerator, LlmProvider};

use crate::detection::{DetectorProvider, LazyRfDetr};
use crate::jobs::JobManager;
use crate::settings::Settings;
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
    /// User settings (API key, models).
    pub settings: Settings,
    /// Hosted language and vision models.
    pub llm: Arc<dyn LlmProvider>,
    /// The image model that produces edited images.
    pub generator: Arc<dyn ImageGenerator>,
}

impl AppState {
    /// Assemble state from explicit parts (used by tests).
    #[must_use]
    pub fn new(config: Config, blobs: Arc<dyn BlobStore>, store: Arc<dyn ProjectStore>) -> Self {
        let jobs = JobManager::new(Arc::clone(&store), config.job_timeout);
        let detectors = Arc::new(LazyRfDetr::from_config(&config));
        let settings = Settings::new(config.openrouter_api_key.clone(), Arc::clone(&store));
        let (llm, generator): (Arc<dyn LlmProvider>, Arc<dyn ImageGenerator>) = if config.demo_ai {
            (Arc::new(DemoProvider), Arc::new(DemoGenerator))
        } else {
            let hosted = Arc::new(OpenRouterProvider::new(Arc::new(settings.clone())));
            (hosted.clone(), hosted)
        };
        Self {
            config: Arc::new(config),
            blobs,
            store,
            jobs,
            detectors,
            settings,
            llm,
            generator,
        }
    }

    /// Replace the image generator (used by tests and custom setups).
    #[must_use]
    pub fn with_generator(mut self, generator: Arc<dyn ImageGenerator>) -> Self {
        self.generator = generator;
        self
    }

    /// Replace the model provider (used by tests and custom setups).
    #[must_use]
    pub fn with_llm(mut self, llm: Arc<dyn LlmProvider>) -> Self {
        self.llm = llm;
        self
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
