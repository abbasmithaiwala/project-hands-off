//! Storage abstractions. Implementations live in the API crate.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::ErrorCode;
use crate::error::{Error, Result};
use crate::ids::{ContentHash, ImageId, JobId, ObjectId};
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

    /// Key for an image's cached analysis (the list of detected objects).
    #[must_use]
    pub fn analysis(id: ImageId) -> Self {
        Self(format!("images/{id}/analysis.json"))
    }

    /// Key for one detected object's mask PNG.
    #[must_use]
    pub fn mask(id: ImageId, object: ObjectId) -> Self {
        Self(format!("images/{id}/masks/{}.png", object.0))
    }

    /// Key for a cached AI result (property suggestions, widget plans).
    ///
    /// # Errors
    /// [`Error::Validation`] if `name` is not a valid key segment.
    pub fn ai_cache(id: ImageId, object: ObjectId, name: &str) -> Result<Self> {
        Self::new(format!("images/{id}/objects/{}/{name}.json", object.0))
    }

    /// Key for the image produced by an edit job.
    #[must_use]
    pub fn job_result(job: JobId) -> Self {
        Self(format!("jobs/{job}/result.png"))
    }

    /// Prefix holding everything stored for an image.
    #[must_use]
    pub fn image_dir(id: ImageId) -> Self {
        Self(format!("images/{id}"))
    }

    /// Prefix holding everything stored for a job.
    #[must_use]
    pub fn job_dir(job: JobId) -> Self {
        Self(format!("jobs/{job}"))
    }

    /// Key for the metadata describing an edit job's result.
    #[must_use]
    pub fn job_meta(job: JobId) -> Self {
        Self(format!("jobs/{job}/result.json"))
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

    /// Remove `prefix` and everything beneath it. Removing nothing is not an error.
    async fn remove_tree(&self, prefix: &BlobKey) -> Result<()>;
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

    /// Latest versions (images nothing was edited from), most recent first.
    async fn list_tips(&self, limit: u32) -> Result<Vec<ImageRecord>>;

    /// The version chain ending at `id`, original first.
    async fn lineage(&self, id: ImageId) -> Result<Vec<ImageRecord>>;

    /// Rename every version in the chain ending at `id`.
    ///
    /// # Errors
    /// [`Error::NotFound`] when the image does not exist.
    async fn rename_lineage(&self, id: ImageId, name: &str) -> Result<()>;

    /// Delete `id` and every version edited from it, returning the removed ids.
    async fn delete_image_tree(&self, id: ImageId) -> Result<Vec<ImageId>>;

    /// Delete job records finished before `cutoff`, returning their ids.
    async fn expire_jobs(&self, cutoff: DateTime<Utc>) -> Result<Vec<JobId>>;

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

    /// Read a stored setting.
    async fn setting(&self, key: &str) -> Result<Option<String>>;

    /// Create or replace a setting.
    async fn set_setting(&self, key: &str, value: &str) -> Result<()>;

    /// Remove a setting. Removing a missing setting is not an error.
    async fn delete_setting(&self, key: &str) -> Result<()>;
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
        assert!(BlobKey::new(BlobKey::analysis(id).as_str()).is_ok());
        assert!(BlobKey::new(BlobKey::mask(id, ObjectId(3)).as_str()).is_ok());
        let job = JobId::new();
        assert!(BlobKey::new(BlobKey::job_result(job).as_str()).is_ok());
        assert!(BlobKey::new(BlobKey::job_meta(job).as_str()).is_ok());
    }
}
