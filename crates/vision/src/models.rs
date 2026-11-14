//! Locating, verifying and downloading model weights.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use futures::StreamExt;
use hands_off_core::{Error, Result};
use sha2::{Digest, Sha256};
use tokio::fs;
use tokio::io::AsyncWriteExt;

const MANIFEST_FILE: &str = "manifest.json";

/// A model file the application needs.
#[derive(Debug, Clone)]
pub struct ModelSpec {
    /// File name inside the model directory.
    pub file_name: String,
    /// Where to download it from if it is missing.
    pub url: Option<String>,
    /// Expected SHA-256, checked after a download.
    pub sha256: Option<String>,
    /// Shown in the "not installed" message.
    pub install_hint: String,
}

impl ModelSpec {
    /// The default segmentation model.
    #[must_use]
    pub fn rf_detr_seg_medium(url: Option<String>, sha256: Option<String>) -> Self {
        Self {
            file_name: "rfdetr-seg-medium.onnx".into(),
            url,
            sha256: sha256.map(|s| s.to_ascii_lowercase()),
            install_hint:
                "run `just models` to export it, or set HANDS_OFF_RFDETR_SEG_URL to a hosted copy"
                    .into(),
        }
    }
}

/// Manages model files in one directory, recording their checksums in `manifest.json`.
#[derive(Debug, Clone)]
pub struct ModelManager {
    dir: PathBuf,
    client: reqwest::Client,
}

impl ModelManager {
    /// Manage models stored in `dir`.
    #[must_use]
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Return the path of a verified model, downloading it first if needed.
    ///
    /// `progress` receives `(bytes_downloaded, total_bytes)` while downloading.
    ///
    /// # Errors
    /// - [`Error::Unavailable`] if the file is missing and no download URL is configured.
    /// - [`Error::Validation`] if a file fails its checksum.
    /// - [`Error::Internal`] on I/O or network failures.
    pub async fn ensure(
        &self,
        spec: &ModelSpec,
        progress: impl Fn(u64, Option<u64>),
    ) -> Result<PathBuf> {
        let path = self.dir.join(&spec.file_name);
        if fs::try_exists(&path).await.map_err(Error::internal)? {
            self.verify_existing(spec, &path).await?;
            return Ok(path);
        }
        let Some(url) = &spec.url else {
            return Err(Error::Unavailable(format!(
                "model {} is not installed: {}",
                spec.file_name, spec.install_hint
            )));
        };
        fs::create_dir_all(&self.dir)
            .await
            .map_err(Error::internal)?;
        let digest = self.download(url, &path, progress).await?;
        if spec
            .sha256
            .as_ref()
            .is_some_and(|expected| *expected != digest)
        {
            let _ = fs::remove_file(&path).await;
            return Err(Error::Validation(format!(
                "downloaded {} does not match the expected checksum",
                spec.file_name
            )));
        }
        self.record(&spec.file_name, &digest).await?;
        Ok(path)
    }

    async fn verify_existing(&self, spec: &ModelSpec, path: &Path) -> Result<()> {
        let digest = sha256_of_file(path).await?;
        let manifest = self.read_manifest().await?;
        let expected = spec
            .sha256
            .clone()
            .or_else(|| manifest.get(&spec.file_name).cloned());
        match expected {
            Some(expected) if expected != digest => Err(Error::Validation(format!(
                "{} is corrupted or was modified (checksum mismatch); delete it and reinstall",
                spec.file_name
            ))),
            Some(_) => Ok(()),
            None => self.record(&spec.file_name, &digest).await,
        }
    }

    async fn download(
        &self,
        url: &str,
        path: &Path,
        progress: impl Fn(u64, Option<u64>),
    ) -> Result<String> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(Error::internal)?;
        let total = response.content_length();
        let temp = path.with_extension("download");
        let mut file = fs::File::create(&temp).await.map_err(Error::internal)?;
        let mut hasher = Sha256::new();
        let mut downloaded = 0_u64;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(Error::internal)?;
            hasher.update(&chunk);
            file.write_all(&chunk).await.map_err(Error::internal)?;
            downloaded += chunk.len() as u64;
            progress(downloaded, total);
        }
        file.flush().await.map_err(Error::internal)?;
        drop(file);
        fs::rename(&temp, path).await.map_err(Error::internal)?;
        Ok(hex::encode(hasher.finalize()))
    }

    async fn read_manifest(&self) -> Result<BTreeMap<String, String>> {
        match fs::read(self.dir.join(MANIFEST_FILE)).await {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|_| Error::Validation("models/manifest.json is malformed".into())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
            Err(e) => Err(Error::internal(e)),
        }
    }

    async fn record(&self, file_name: &str, digest: &str) -> Result<()> {
        let mut manifest = self.read_manifest().await?;
        manifest.insert(file_name.to_owned(), digest.to_owned());
        let json = serde_json::to_vec_pretty(&manifest).map_err(Error::internal)?;
        fs::write(self.dir.join(MANIFEST_FILE), json)
            .await
            .map_err(Error::internal)
    }
}

async fn sha256_of_file(path: &Path) -> Result<String> {
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || {
        use std::io::Read;
        let mut file = std::fs::File::open(path).map_err(Error::internal)?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0_u8; 1 << 20];
        loop {
            let read = file.read(&mut buffer).map_err(Error::internal)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        Ok(hex::encode(hasher.finalize()))
    })
    .await
    .map_err(Error::internal)?
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use axum::Router;
    use axum::routing::get;
    use tokio::net::TcpListener;

    use super::*;

    const PAYLOAD: &[u8] = b"pretend this is an onnx model";

    fn payload_sha() -> String {
        hex::encode(Sha256::digest(PAYLOAD))
    }

    async fn serve() -> String {
        let app = Router::new().route("/model.onnx", get(|| async { PAYLOAD }));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}/model.onnx")
    }

    fn spec(url: Option<String>, sha: Option<String>) -> ModelSpec {
        ModelSpec {
            file_name: "m.onnx".into(),
            url,
            sha256: sha,
            install_hint: "install it".into(),
        }
    }

    #[tokio::test]
    async fn downloads_verifies_and_records_a_missing_model() {
        let dir = tempfile::tempdir().unwrap();
        let manager = ModelManager::new(dir.path());
        let reported = AtomicU64::new(0);

        let path = manager
            .ensure(
                &spec(Some(serve().await), Some(payload_sha())),
                |done, _| {
                    reported.store(done, Ordering::SeqCst);
                },
            )
            .await
            .unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), PAYLOAD);
        assert_eq!(reported.load(Ordering::SeqCst), PAYLOAD.len() as u64);
        let manifest = std::fs::read_to_string(dir.path().join(MANIFEST_FILE)).unwrap();
        assert!(manifest.contains(&payload_sha()));
        assert!(!dir.path().join("m.download").exists());
    }

    #[tokio::test]
    async fn rejects_and_removes_a_download_with_the_wrong_checksum() {
        let dir = tempfile::tempdir().unwrap();
        let manager = ModelManager::new(dir.path());
        let wrong = Some("0".repeat(64));

        let error = manager
            .ensure(&spec(Some(serve().await), wrong), |_, _| {})
            .await
            .unwrap_err();

        assert!(matches!(error, Error::Validation(_)));
        assert!(!dir.path().join("m.onnx").exists());
    }

    #[tokio::test]
    async fn missing_model_without_a_url_explains_how_to_install_it() {
        let dir = tempfile::tempdir().unwrap();
        let error = ModelManager::new(dir.path())
            .ensure(&spec(None, None), |_, _| {})
            .await
            .unwrap_err();
        assert!(
            matches!(&error, Error::Unavailable(m) if m.contains("install it") && m.contains("m.onnx"))
        );
    }

    #[tokio::test]
    async fn existing_models_are_trusted_on_first_use_then_verified() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("m.onnx"), PAYLOAD).unwrap();
        let manager = ModelManager::new(dir.path());

        manager.ensure(&spec(None, None), |_, _| {}).await.unwrap();
        assert!(
            std::fs::read_to_string(dir.path().join(MANIFEST_FILE))
                .unwrap()
                .contains(&payload_sha())
        );

        std::fs::write(dir.path().join("m.onnx"), b"tampered").unwrap();
        let error = manager
            .ensure(&spec(None, None), |_, _| {})
            .await
            .unwrap_err();
        assert!(matches!(error, Error::Validation(_)));
    }

    #[tokio::test]
    async fn a_pinned_checksum_overrides_the_manifest() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("m.onnx"), PAYLOAD).unwrap();
        let manager = ModelManager::new(dir.path());
        assert!(
            manager
                .ensure(&spec(None, Some(payload_sha())), |_, _| {})
                .await
                .is_ok()
        );
        assert!(
            manager
                .ensure(&spec(None, Some("1".repeat(64))), |_, _| {})
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn http_errors_surface_as_internal_failures() {
        let dir = tempfile::tempdir().unwrap();
        let url = serve().await.replace("model.onnx", "missing.onnx");
        let error = ModelManager::new(dir.path())
            .ensure(&spec(Some(url), None), |_, _| {})
            .await
            .unwrap_err();
        assert!(matches!(error, Error::Internal(_)));
        assert!(!dir.path().join("m.onnx").exists());
    }

    #[tokio::test]
    async fn a_malformed_manifest_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("m.onnx"), PAYLOAD).unwrap();
        std::fs::write(dir.path().join(MANIFEST_FILE), "not json").unwrap();
        let error = ModelManager::new(dir.path())
            .ensure(&spec(None, None), |_, _| {})
            .await
            .unwrap_err();
        assert!(matches!(error, Error::Validation(_)));
    }
}
