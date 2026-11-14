//! Access to the object detector, loaded lazily so the server starts without model files.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use hands_off_core::{Config, Error, Result};
use hands_off_vision::Detector;
use hands_off_vision::models::{ModelManager, ModelSpec};
use hands_off_vision::rfdetr::RfDetrSeg;
use tokio::sync::OnceCell;

use crate::jobs::JobContext;

/// Supplies a ready-to-use detector, preparing it on first request.
#[async_trait]
pub trait DetectorProvider: Send + Sync {
    /// Return the detector, loading or downloading the model if necessary.
    ///
    /// # Errors
    /// [`Error::Unavailable`] if the model is not installed, other errors on load failure.
    async fn detector(&self, context: &JobContext) -> Result<Arc<dyn Detector>>;
}

/// Loads RF-DETR-Seg from the model directory the first time it is needed.
pub struct LazyRfDetr {
    manager: ModelManager,
    spec: ModelSpec,
    cell: OnceCell<Arc<dyn Detector>>,
}

impl LazyRfDetr {
    /// Detector configured from `config`.
    #[must_use]
    pub fn from_config(config: &Config) -> Self {
        Self {
            manager: ModelManager::new(&config.model_dir),
            spec: ModelSpec::rf_detr_seg_medium(
                config.rfdetr_seg_url.clone(),
                config.rfdetr_seg_sha256.clone(),
            ),
            cell: OnceCell::new(),
        }
    }
}

#[async_trait]
impl DetectorProvider for LazyRfDetr {
    async fn detector(&self, context: &JobContext) -> Result<Arc<dyn Detector>> {
        let detector = self
            .cell
            .get_or_try_init(|| async {
                let last_reported = AtomicU64::new(0);
                let path = self
                    .manager
                    .ensure(&self.spec, |done, total| {
                        let percent = total.map_or(0, |t| done * 100 / t.max(1));
                        if percent >= last_reported.load(Ordering::Relaxed) + 10 {
                            last_reported.store(percent, Ordering::Relaxed);
                            context.log(&format!("Downloading detection model: {percent}%"));
                        }
                    })
                    .await?;
                context.log("Loading detection model");
                let model = tokio::task::spawn_blocking(move || RfDetrSeg::load(&path))
                    .await
                    .map_err(Error::internal)??;
                Ok::<Arc<dyn Detector>, Error>(Arc::new(model))
            })
            .await?;
        Ok(Arc::clone(detector))
    }
}

/// A provider that always returns the same detector. Used in tests and for custom setups.
pub struct StaticDetector(pub Arc<dyn Detector>);

#[async_trait]
impl DetectorProvider for StaticDetector {
    async fn detector(&self, _context: &JobContext) -> Result<Arc<dyn Detector>> {
        Ok(Arc::clone(&self.0))
    }
}
