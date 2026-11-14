//! Storage abstractions. Implementations live in the API crate.

use async_trait::async_trait;

use crate::ErrorCode;
use crate::error::{Error, Result};
use crate::ids::{ContentHash, ImageId, JobId};
use crate::job::{JobRecord, JobStatus};
use crate::model::ImageRecord;

/// A validated, relative key addressing one blob.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlobKey(String);

impl BlobKey {
    /// Validate a key made of lowercase letters, digits, `-`, `_`, `.` and `/` separators.
    ///
    /// # Errors
    /// Returns [`Error::Validation`] for empty keys, absolute paths, empty or dot-only
    /// segments, or characters outside the allowed set.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        let allowed =
            |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '.');
        let valid = !value.is_empty()
            && value.split('/').all(|segment| {
                !segment.is_empty() && !segment.starts_with('.') && segment.chars().all(allowed)
            });
        if valid {
            Ok(Self(value))
        } else {
            Err(Error::Validation(format!("invalid blob key: {value}")))
        }
    }

    /// Key for an image's original bytes.
    #[must_use]
    pub fn original(id: ImageId) -> Self {
        Self(format!("images/{id}/original"))
    }

    /// Key for an image's thumbnail.
    #[must_use]
    pub fn thumbnail(id: ImageId) -> Self {
        Self(format!("images/{id}/thumbnail.jpg"))
    }

    /// The key as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Binary object storage (filesystem now, S3-compatible later).
#[async_trait]
pub trait BlobStore: Send + Sync {
    /// Store `bytes` under `key`, replacing any existing blob.
    async fn put(&self, key: &BlobKey, bytes: Vec<u8>) -> Result<()>;

    /// Read the blob at `key`.
    ///
    /// # Errors
    /// [`Error::NotFound`] when the key does not exist.
    async fn get(&self, key: &BlobKey) -> Result<Vec<u8>>;
}

/// Failure details recorded when a job fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobFailure {
    /// Stable error code.
    pub code: ErrorCode,
    /// Human-readable message.
    pub message: String,
}

/// Persistent records for images and jobs.
#[async_trait]
pub trait ProjectStore: Send + Sync {
    /// Insert a new image record.
    async fn insert_image(&self, record: &ImageRecord) -> Result<()>;

    /// Find an image by id.
    async fn image_by_id(&self, id: ImageId) -> Result<Option<ImageRecord>>;

    /// Find an image by content hash.
    async fn image_by_hash(&self, hash: &ContentHash) -> Result<Option<ImageRecord>>;

    /// Insert a new job record.
    async fn insert_job(&self, record: &JobRecord) -> Result<()>;

    /// Move a job to a terminal state.
    async fn finish_job(
        &self,
        id: JobId,
        status: JobStatus,
        failure: Option<JobFailure>,
    ) -> Result<()>;

    /// Find a job by id.
    async fn job_by_id(&self, id: JobId) -> Result<Option<JobRecord>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_well_formed_keys() {
        for key in ["images/abc/original", "a", "images/x-1_2/thumbnail.jpg"] {
            assert!(BlobKey::new(key).is_ok(), "{key}");
        }
    }

    #[test]
    fn rejects_traversal_absolute_and_malformed_keys() {
        for key in [
            "",
            "/etc/passwd",
            "../x",
            "a/../b",
            "a//b",
            "a/.hidden",
            "UPPER",
            "a b",
            "a\\b",
        ] {
            assert!(BlobKey::new(key).is_err(), "{key}");
        }
    }

    #[test]
    fn derived_keys_are_valid() {
        let id = ImageId::new();
        assert!(BlobKey::new(BlobKey::original(id).as_str()).is_ok());
        assert!(BlobKey::new(BlobKey::thumbnail(id).as_str()).is_ok());
    }
}
