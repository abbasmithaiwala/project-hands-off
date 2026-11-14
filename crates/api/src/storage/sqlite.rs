//! `SQLite` records for images and jobs.

use std::path::Path;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hands_off_core::job::{JobRecord, JobStatus};
use hands_off_core::model::{ImageFormat, ImageRecord};
use hands_off_core::storage::{JobFailure, ProjectStore};
use hands_off_core::{ContentHash, Error, ErrorCode, ImageId, JobId, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteRow};
use sqlx::{Row, SqlitePool};

/// SQLite-backed records for images and jobs.
#[derive(Debug, Clone)]
pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    /// Open (creating if needed) the database at `path` and apply migrations.
    ///
    /// # Errors
    /// Returns [`Error::Internal`] if the database cannot be opened or migrated.
    pub async fn open(path: &Path) -> Result<Self> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(options)
            .await
            .map_err(Error::internal)?;
        Self::migrated(pool).await
    }

    /// An isolated in-memory database, for tests.
    ///
    /// # Errors
    /// Returns [`Error::Internal`] if the database cannot be created.
    pub async fn in_memory() -> Result<Self> {
        let options = SqliteConnectOptions::new()
            .in_memory(true)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .map_err(Error::internal)?;
        Self::migrated(pool).await
    }

    async fn migrated(pool: SqlitePool) -> Result<Self> {
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(Error::internal)?;
        Ok(Self { pool })
    }
}

fn corrupt(what: &str) -> Error {
    Error::internal(std::io::Error::other(format!("corrupt {what} in database")))
}

fn parse_time(raw: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|_| corrupt("timestamp"))
}

fn image_from_row(row: &SqliteRow) -> Result<ImageRecord> {
    let get = |e: sqlx::Error| Error::internal(e);
    let id: String = row.try_get("id").map_err(get)?;
    let hash: String = row.try_get("hash").map_err(get)?;
    let format: String = row.try_get("format").map_err(get)?;
    let width: i64 = row.try_get("width").map_err(get)?;
    let height: i64 = row.try_get("height").map_err(get)?;
    let size: i64 = row.try_get("size_bytes").map_err(get)?;
    let created: String = row.try_get("created_at").map_err(get)?;
    Ok(ImageRecord {
        id: id.parse().map_err(|_| corrupt("image id"))?,
        hash: ContentHash::new(hash).map_err(|_| corrupt("content hash"))?,
        format: ImageFormat::from_name(&format).ok_or_else(|| corrupt("image format"))?,
        width: u32::try_from(width).map_err(|_| corrupt("width"))?,
        height: u32::try_from(height).map_err(|_| corrupt("height"))?,
        size_bytes: u64::try_from(size).map_err(|_| corrupt("size"))?,
        created_at: parse_time(&created)?,
    })
}

fn job_from_row(row: &SqliteRow) -> Result<JobRecord> {
    let get = |e: sqlx::Error| Error::internal(e);
    let id: String = row.try_get("id").map_err(get)?;
    let status: String = row.try_get("status").map_err(get)?;
    let code: Option<String> = row.try_get("error_code").map_err(get)?;
    let finished: Option<String> = row.try_get("finished_at").map_err(get)?;
    let created: String = row.try_get("created_at").map_err(get)?;
    Ok(JobRecord {
        id: id.parse().map_err(|_| corrupt("job id"))?,
        kind: row.try_get("kind").map_err(get)?,
        status: JobStatus::from_name(&status).ok_or_else(|| corrupt("job status"))?,
        error_code: code
            .map(|c| {
                serde_json::from_value(serde_json::Value::String(c))
                    .map_err(|_| corrupt("error code"))
            })
            .transpose()?,
        error_message: row.try_get("error_message").map_err(get)?,
        created_at: parse_time(&created)?,
        finished_at: finished.as_deref().map(parse_time).transpose()?,
    })
}

fn error_code_name(code: ErrorCode) -> Result<String> {
    match serde_json::to_value(code).map_err(Error::internal)? {
        serde_json::Value::String(name) => Ok(name),
        _ => Err(corrupt("error code")),
    }
}

#[async_trait]
impl ProjectStore for SqliteStore {
    async fn insert_image(&self, record: &ImageRecord) -> Result<()> {
        sqlx::query(
            "INSERT INTO images (id, hash, format, width, height, size_bytes, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(record.id.to_string())
        .bind(record.hash.as_str())
        .bind(record.format.as_str())
        .bind(i64::from(record.width))
        .bind(i64::from(record.height))
        .bind(i64::try_from(record.size_bytes).map_err(Error::internal)?)
        .bind(record.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(Error::internal)?;
        Ok(())
    }

    async fn image_by_id(&self, id: ImageId) -> Result<Option<ImageRecord>> {
        let row = sqlx::query("SELECT * FROM images WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(Error::internal)?;
        row.as_ref().map(image_from_row).transpose()
    }

    async fn image_by_hash(&self, hash: &ContentHash) -> Result<Option<ImageRecord>> {
        let row = sqlx::query("SELECT * FROM images WHERE hash = ?")
            .bind(hash.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(Error::internal)?;
        row.as_ref().map(image_from_row).transpose()
    }

    async fn insert_job(&self, record: &JobRecord) -> Result<()> {
        sqlx::query("INSERT INTO jobs (id, kind, status, created_at) VALUES (?, ?, ?, ?)")
            .bind(record.id.to_string())
            .bind(&record.kind)
            .bind(record.status.as_str())
            .bind(record.created_at.to_rfc3339())
            .execute(&self.pool)
            .await
            .map_err(Error::internal)?;
        Ok(())
    }

    async fn finish_job(
        &self,
        id: JobId,
        status: JobStatus,
        failure: Option<JobFailure>,
    ) -> Result<()> {
        let (code, message) = match failure {
            Some(f) => (Some(error_code_name(f.code)?), Some(f.message)),
            None => (None, None),
        };
        let result = sqlx::query(
            "UPDATE jobs SET status = ?, error_code = ?, error_message = ?, finished_at = ?
             WHERE id = ?",
        )
        .bind(status.as_str())
        .bind(code)
        .bind(message)
        .bind(Utc::now().to_rfc3339())
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(Error::internal)?;
        if result.rows_affected() == 0 {
            return Err(Error::NotFound("job"));
        }
        Ok(())
    }

    async fn job_by_id(&self, id: JobId) -> Result<Option<JobRecord>> {
        let row = sqlx::query("SELECT * FROM jobs WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(Error::internal)?;
        row.as_ref().map(job_from_row).transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image() -> ImageRecord {
        ImageRecord {
            id: ImageId::new(),
            hash: ContentHash::new("ab".repeat(32)).unwrap(),
            format: ImageFormat::Png,
            width: 640,
            height: 480,
            size_bytes: 12_345,
            created_at: Utc::now(),
        }
    }

    fn job() -> JobRecord {
        JobRecord {
            id: JobId::new(),
            kind: "analyze".into(),
            status: JobStatus::Running,
            error_code: None,
            error_message: None,
            created_at: Utc::now(),
            finished_at: None,
        }
    }

    #[tokio::test]
    async fn image_round_trips_by_id_and_by_hash() {
        let store = SqliteStore::in_memory().await.unwrap();
        let record = image();
        store.insert_image(&record).await.unwrap();

        let by_id = store.image_by_id(record.id).await.unwrap().unwrap();
        assert_eq!(by_id.hash, record.hash);
        assert_eq!(
            (by_id.width, by_id.height, by_id.size_bytes),
            (640, 480, 12_345)
        );
        assert_eq!(
            store.image_by_hash(&record.hash).await.unwrap().unwrap().id,
            record.id
        );
    }

    #[tokio::test]
    async fn unknown_image_is_none() {
        let store = SqliteStore::in_memory().await.unwrap();
        assert!(store.image_by_id(ImageId::new()).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn duplicate_hashes_are_rejected() {
        let store = SqliteStore::in_memory().await.unwrap();
        let first = image();
        let mut second = image();
        second.hash = first.hash.clone();
        store.insert_image(&first).await.unwrap();
        assert!(store.insert_image(&second).await.is_err());
    }

    #[tokio::test]
    async fn job_lifecycle_is_persisted() {
        let store = SqliteStore::in_memory().await.unwrap();
        let record = job();
        store.insert_job(&record).await.unwrap();
        assert_eq!(
            store.job_by_id(record.id).await.unwrap().unwrap().status,
            JobStatus::Running
        );

        let failure = JobFailure {
            code: ErrorCode::Timeout,
            message: "too slow".into(),
        };
        store
            .finish_job(record.id, JobStatus::Failed, Some(failure))
            .await
            .unwrap();
        let done = store.job_by_id(record.id).await.unwrap().unwrap();
        assert_eq!(done.status, JobStatus::Failed);
        assert_eq!(done.error_code, Some(ErrorCode::Timeout));
        assert_eq!(done.error_message.as_deref(), Some("too slow"));
        assert!(done.finished_at.is_some());
    }

    #[tokio::test]
    async fn finishing_an_unknown_job_is_not_found() {
        let store = SqliteStore::in_memory().await.unwrap();
        let error = store
            .finish_job(JobId::new(), JobStatus::Completed, None)
            .await
            .unwrap_err();
        assert!(matches!(error, Error::NotFound(_)));
    }

    #[tokio::test]
    async fn opens_a_database_file_and_reopens_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.db");
        let record = image();
        SqliteStore::open(&path)
            .await
            .unwrap()
            .insert_image(&record)
            .await
            .unwrap();
        let reopened = SqliteStore::open(&path).await.unwrap();
        assert!(reopened.image_by_id(record.id).await.unwrap().is_some());
    }
}
