//! `SQLite` records for images and jobs.

use std::path::Path;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hands_off_core::job::{JobRecord, JobStatus};
use hands_off_core::model::{EditResult, ImageFormat, ImageRecord};
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
    let parent: Option<String> = row.try_get("parent_id").map_err(get)?;
    let edit: Option<String> = row.try_get("edit_json").map_err(get)?;
    Ok(ImageRecord {
        id: id.parse().map_err(|_| corrupt("image id"))?,
        hash: ContentHash::new(hash).map_err(|_| corrupt("content hash"))?,
        format: ImageFormat::from_name(&format).ok_or_else(|| corrupt("image format"))?,
        width: u32::try_from(width).map_err(|_| corrupt("width"))?,
        height: u32::try_from(height).map_err(|_| corrupt("height"))?,
        size_bytes: u64::try_from(size).map_err(|_| corrupt("size"))?,
        created_at: parse_time(&created)?,
        name: row.try_get("name").map_err(get)?,
        parent_id: parent
            .map(|p| p.parse().map_err(|_| corrupt("parent id")))
            .transpose()?,
        edit: edit
            .map(|e| serde_json::from_str::<EditResult>(&e).map_err(|_| corrupt("edit")))
            .transpose()?,
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
            "INSERT INTO images
                 (id, hash, format, width, height, size_bytes, created_at, name, parent_id, edit_json)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(record.id.to_string())
        .bind(record.hash.as_str())
        .bind(record.format.as_str())
        .bind(i64::from(record.width))
        .bind(i64::from(record.height))
        .bind(i64::try_from(record.size_bytes).map_err(Error::internal)?)
        .bind(record.created_at.to_rfc3339())
        .bind(record.name.as_deref())
        .bind(record.parent_id.map(|p| p.to_string()))
        .bind(
            record
                .edit
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(Error::internal)?,
        )
        .execute(&self.pool)
        .await
        .map_err(Error::internal)?;
        Ok(())
    }

    async fn list_tips(&self, limit: u32) -> Result<Vec<ImageRecord>> {
        let rows = sqlx::query(
            "SELECT * FROM images
             WHERE id NOT IN (SELECT parent_id FROM images WHERE parent_id IS NOT NULL)
             ORDER BY created_at DESC LIMIT ?",
        )
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(Error::internal)?;
        rows.iter().map(image_from_row).collect()
    }

    async fn lineage(&self, id: ImageId) -> Result<Vec<ImageRecord>> {
        let rows = sqlx::query(
            "WITH RECURSIVE chain(id, depth) AS (
                 SELECT id, 0 FROM images WHERE id = ?1
                 UNION ALL
                 SELECT i.parent_id, chain.depth + 1
                 FROM images i JOIN chain ON i.id = chain.id
                 WHERE i.parent_id IS NOT NULL
             )
             SELECT images.* FROM images JOIN chain ON images.id = chain.id
             ORDER BY chain.depth DESC",
        )
        .bind(id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(Error::internal)?;
        rows.iter().map(image_from_row).collect()
    }

    async fn rename_lineage(&self, id: ImageId, name: &str) -> Result<()> {
        let result = sqlx::query(
            "WITH RECURSIVE chain(id) AS (
                 SELECT id FROM images WHERE id = ?1
                 UNION ALL
                 SELECT i.parent_id FROM images i JOIN chain ON i.id = chain.id
                 WHERE i.parent_id IS NOT NULL
             )
             UPDATE images SET name = ?2 WHERE id IN (SELECT id FROM chain)",
        )
        .bind(id.to_string())
        .bind(name)
        .execute(&self.pool)
        .await
        .map_err(Error::internal)?;
        if result.rows_affected() == 0 {
            return Err(Error::NotFound("image"));
        }
        Ok(())
    }

    async fn delete_image_tree(&self, id: ImageId) -> Result<Vec<ImageId>> {
        let mut tx = self.pool.begin().await.map_err(Error::internal)?;
        let rows = sqlx::query(
            "WITH RECURSIVE tree(id) AS (
                 SELECT id FROM images WHERE id = ?1
                 UNION ALL
                 SELECT i.id FROM images i JOIN tree ON i.parent_id = tree.id
             )
             SELECT id FROM tree",
        )
        .bind(id.to_string())
        .fetch_all(&mut *tx)
        .await
        .map_err(Error::internal)?;
        // Descendants are removed by the `ON DELETE CASCADE` on `parent_id`.
        sqlx::query("DELETE FROM images WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(Error::internal)?;
        tx.commit().await.map_err(Error::internal)?;
        rows.iter()
            .map(|row| {
                let raw: String = row.try_get("id").map_err(Error::internal)?;
                raw.parse().map_err(|_| corrupt("image id"))
            })
            .collect()
    }

    async fn expire_jobs(&self, cutoff: DateTime<Utc>) -> Result<Vec<JobId>> {
        let rows = sqlx::query(
            "DELETE FROM jobs WHERE finished_at IS NOT NULL AND finished_at < ? RETURNING id",
        )
        .bind(cutoff.to_rfc3339())
        .fetch_all(&self.pool)
        .await
        .map_err(Error::internal)?;
        rows.iter()
            .map(|row| {
                let raw: String = row.try_get("id").map_err(Error::internal)?;
                raw.parse().map_err(|_| corrupt("job id"))
            })
            .collect()
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

    async fn setting(&self, key: &str) -> Result<Option<String>> {
        let row = sqlx::query("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(Error::internal)?;
        row.map(|r| r.try_get("value").map_err(Error::internal))
            .transpose()
    }

    async fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await
        .map_err(Error::internal)?;
        Ok(())
    }

    async fn delete_setting(&self, key: &str) -> Result<()> {
        sqlx::query("DELETE FROM settings WHERE key = ?")
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(Error::internal)?;
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
            name: None,
            parent_id: None,
            edit: None,
        }
    }

    fn version_of(parent: &ImageRecord, tag: &str) -> ImageRecord {
        ImageRecord {
            hash: ContentHash::new(tag.repeat(32)).unwrap(),
            parent_id: Some(parent.id),
            edit: Some(EditResult {
                summary: format!("edit {tag}"),
                changes: Vec::new(),
                instruction: "i".into(),
                model: "m".into(),
                cost_usd: Some(0.1),
                generation_ms: 5,
            }),
            created_at: parent.created_at + chrono::Duration::seconds(1),
            ..image()
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
    async fn settings_can_be_set_replaced_and_deleted() {
        let store = SqliteStore::in_memory().await.unwrap();
        assert_eq!(store.setting("k").await.unwrap(), None);
        store.set_setting("k", "one").await.unwrap();
        store.set_setting("k", "two").await.unwrap();
        assert_eq!(store.setting("k").await.unwrap().as_deref(), Some("two"));
        store.delete_setting("k").await.unwrap();
        store.delete_setting("k").await.unwrap();
        assert_eq!(store.setting("k").await.unwrap(), None);
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

    #[tokio::test]
    async fn lineage_lists_versions_original_first() {
        let store = SqliteStore::in_memory().await.unwrap();
        let root = image();
        let mid = version_of(&root, "cd");
        let tip = version_of(&mid, "ef");
        for record in [&root, &mid, &tip] {
            store.insert_image(record).await.unwrap();
        }
        let ids: Vec<_> = store
            .lineage(tip.id)
            .await
            .unwrap()
            .iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(ids, [root.id, mid.id, tip.id]);
        let chain = store.lineage(mid.id).await.unwrap();
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[1].edit.as_ref().unwrap().summary, "edit cd");
    }

    #[tokio::test]
    async fn tips_exclude_versions_that_were_edited_further() {
        let store = SqliteStore::in_memory().await.unwrap();
        let root = image();
        let tip = version_of(&root, "cd");
        let mut other = image();
        other.hash = ContentHash::new("12".repeat(32)).unwrap();
        for record in [&root, &tip, &other] {
            store.insert_image(record).await.unwrap();
        }
        let tips: Vec<_> = store
            .list_tips(10)
            .await
            .unwrap()
            .iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(tips.len(), 2);
        assert!(tips.contains(&tip.id) && tips.contains(&other.id));
        assert_eq!(store.list_tips(1).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn renaming_covers_the_whole_chain_and_rejects_unknown_images() {
        let store = SqliteStore::in_memory().await.unwrap();
        let root = image();
        let tip = version_of(&root, "cd");
        store.insert_image(&root).await.unwrap();
        store.insert_image(&tip).await.unwrap();
        store.rename_lineage(tip.id, "Living room").await.unwrap();
        for id in [root.id, tip.id] {
            let name = store.image_by_id(id).await.unwrap().unwrap().name;
            assert_eq!(name.as_deref(), Some("Living room"));
        }
        let missing = store.rename_lineage(ImageId::new(), "x").await;
        assert!(matches!(missing.unwrap_err(), Error::NotFound(_)));
    }

    #[tokio::test]
    async fn deleting_an_image_removes_its_descendants_only() {
        let store = SqliteStore::in_memory().await.unwrap();
        let root = image();
        let mid = version_of(&root, "cd");
        let tip = version_of(&mid, "ef");
        for record in [&root, &mid, &tip] {
            store.insert_image(record).await.unwrap();
        }
        let mut removed = store.delete_image_tree(mid.id).await.unwrap();
        removed.sort_by_key(ToString::to_string);
        let mut expected = vec![mid.id, tip.id];
        expected.sort_by_key(ToString::to_string);
        assert_eq!(removed, expected);
        assert!(store.image_by_id(root.id).await.unwrap().is_some());
        assert!(store.image_by_id(tip.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn only_old_finished_jobs_expire() {
        let store = SqliteStore::in_memory().await.unwrap();
        let (done, running) = (job(), job());
        store.insert_job(&done).await.unwrap();
        store.insert_job(&running).await.unwrap();
        store
            .finish_job(done.id, JobStatus::Completed, None)
            .await
            .unwrap();
        let future = Utc::now() + chrono::Duration::hours(1);
        assert!(
            store
                .expire_jobs(Utc::now() - chrono::Duration::hours(1))
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(store.expire_jobs(future).await.unwrap(), [done.id]);
        assert!(store.job_by_id(running.id).await.unwrap().is_some());
    }
}
