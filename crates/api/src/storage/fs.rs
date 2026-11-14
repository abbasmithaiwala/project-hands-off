//! Local filesystem blob storage.

use std::io::ErrorKind;
use std::path::PathBuf;

use async_trait::async_trait;
use hands_off_core::storage::{BlobKey, BlobStore};
use hands_off_core::{Error, Result};
use tokio::fs;
use uuid::Uuid;

/// Blob storage backed by a directory on the local filesystem.
#[derive(Debug, Clone)]
pub struct FsBlobStore {
    root: PathBuf,
}

impl FsBlobStore {
    /// Store blobs beneath `root`, creating it if needed.
    ///
    /// # Errors
    /// Returns [`Error::Internal`] if the directory cannot be created.
    pub async fn new(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root).await.map_err(Error::internal)?;
        Ok(Self { root })
    }

    fn path(&self, key: &BlobKey) -> PathBuf {
        self.root.join(key.as_str())
    }
}

#[async_trait]
impl BlobStore for FsBlobStore {
    async fn put(&self, key: &BlobKey, bytes: Vec<u8>) -> Result<()> {
        let path = self.path(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(Error::internal)?;
        }
        let temp = path.with_extension(format!("{}.tmp", Uuid::new_v4().simple()));
        fs::write(&temp, bytes).await.map_err(Error::internal)?;
        fs::rename(&temp, &path).await.map_err(Error::internal)
    }

    async fn get(&self, key: &BlobKey) -> Result<Vec<u8>> {
        fs::read(self.path(key)).await.map_err(|e| match e.kind() {
            ErrorKind::NotFound => Error::NotFound("blob"),
            _ => Error::internal(e),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn store() -> (tempfile::TempDir, FsBlobStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = FsBlobStore::new(dir.path()).await.unwrap();
        (dir, store)
    }

    #[tokio::test]
    async fn stores_and_reads_back_bytes() {
        let (_dir, store) = store().await;
        let key = BlobKey::new("images/abc/original").unwrap();
        store.put(&key, b"hello".to_vec()).await.unwrap();
        assert_eq!(store.get(&key).await.unwrap(), b"hello");
    }

    #[tokio::test]
    async fn overwrites_existing_blobs() {
        let (_dir, store) = store().await;
        let key = BlobKey::new("a/b").unwrap();
        store.put(&key, b"one".to_vec()).await.unwrap();
        store.put(&key, b"two".to_vec()).await.unwrap();
        assert_eq!(store.get(&key).await.unwrap(), b"two");
    }

    #[tokio::test]
    async fn missing_blob_is_not_found() {
        let (_dir, store) = store().await;
        let key = BlobKey::new("nope").unwrap();
        assert!(matches!(
            store.get(&key).await.unwrap_err(),
            Error::NotFound(_)
        ));
    }

    #[tokio::test]
    async fn leaves_no_temp_files_behind() {
        let (dir, store) = store().await;
        let key = BlobKey::new("x/y").unwrap();
        store.put(&key, vec![1, 2, 3]).await.unwrap();
        let names: Vec<_> = std::fs::read_dir(dir.path().join("x"))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, ["y"]);
    }
}
