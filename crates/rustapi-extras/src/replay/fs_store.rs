//! Filesystem-based replay store using JSON Lines format.
//!
//! Follows the [`FileAuditStore`](crate::audit::FileAuditStore) pattern.

use async_trait::async_trait;
use rustapi_core::replay::{
    ReplayEntry, ReplayQuery, ReplayStore, ReplayStoreError, ReplayStoreResult,
};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Mutex;

/// Configuration for the filesystem replay store.
#[derive(Debug, Clone)]
pub struct FsReplayStoreConfig {
    /// Directory to store replay files.
    pub directory: PathBuf,
    /// Maximum file size before rotation (bytes). None = no rotation.
    pub max_file_size: Option<u64>,
    /// Create directory if it doesn't exist.
    pub create_if_missing: bool,
}

impl FsReplayStoreConfig {
    /// Create a new config with the given directory.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            directory: dir.into(),
            max_file_size: None,
            create_if_missing: true,
        }
    }

    /// Set the maximum file size before rotation.
    pub fn max_file_size(mut self, bytes: u64) -> Self {
        self.max_file_size = Some(bytes);
        self
    }
}

/// Filesystem-based replay store.
///
/// Stores entries in JSON Lines format (one entry per line).
/// Supports file rotation by size.
pub struct FsReplayStore {
    config: FsReplayStoreConfig,
    writer: Mutex<Option<File>>,
}

impl FsReplayStore {
    /// Create a new filesystem store.
    pub fn new(config: FsReplayStoreConfig) -> ReplayStoreResult<Self> {
        if config.create_if_missing {
            fs::create_dir_all(&config.directory).map_err(|e| {
                ReplayStoreError::Io(format!(
                    "Failed to create directory {:?}: {}",
                    config.directory, e
                ))
            })?;
        }
        Ok(Self {
            config,
            writer: Mutex::new(None),
        })
    }

    /// Open a store at the given directory with defaults.
    pub fn open(dir: impl Into<PathBuf>) -> ReplayStoreResult<Self> {
        Self::new(FsReplayStoreConfig::new(dir))
    }

    fn data_file(&self) -> PathBuf {
        self.config.directory.join("replays.jsonl")
    }

    fn ensure_writer(&self) -> ReplayStoreResult<()> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| ReplayStoreError::Other(format!("Lock poisoned: {}", e)))?;

        if writer.is_none() {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.data_file())
                .map_err(|e| ReplayStoreError::Io(e.to_string()))?;
            *writer = Some(file);
        }

        Ok(())
    }

    fn read_all_entries(&self) -> ReplayStoreResult<Vec<ReplayEntry>> {
        let path = self.data_file();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&path).map_err(|e| ReplayStoreError::Io(e.to_string()))?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for line in reader.lines() {
            let line = line.map_err(|e| ReplayStoreError::Io(e.to_string()))?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<ReplayEntry>(&line) {
                Ok(entry) => entries.push(entry),
                Err(_) => continue, // skip malformed lines
            }
        }

        Ok(entries)
    }

    fn write_all_entries(&self, entries: &[ReplayEntry]) -> ReplayStoreResult<()> {
        let path = self.data_file();
        let mut file = File::create(&path).map_err(|e| ReplayStoreError::Io(e.to_string()))?;

        for entry in entries {
            let line = serde_json::to_string(entry)
                .map_err(|e| ReplayStoreError::Serialization(e.to_string()))?;
            writeln!(file, "{}", line).map_err(|e| ReplayStoreError::Io(e.to_string()))?;
        }

        // Reset writer since we overwrote the file
        if let Ok(mut writer) = self.writer.lock() {
            *writer = None;
        }

        Ok(())
    }

    fn check_rotation(&self) -> ReplayStoreResult<()> {
        if let Some(max_size) = self.config.max_file_size {
            let path = self.data_file();
            if path.exists() {
                let metadata =
                    fs::metadata(&path).map_err(|e| ReplayStoreError::Io(e.to_string()))?;
                if metadata.len() >= max_size {
                    // Rotate: rename current file
                    let rotated = self.config.directory.join(format!(
                        "replays.{}.jsonl",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()
                    ));
                    fs::rename(&path, &rotated)
                        .map_err(|e| ReplayStoreError::Io(e.to_string()))?;

                    // Reset writer
                    if let Ok(mut writer) = self.writer.lock() {
                        *writer = None;
                    }
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl ReplayStore for FsReplayStore {
    async fn store(&self, entry: ReplayEntry) -> ReplayStoreResult<()> {
        self.check_rotation()?;
        self.ensure_writer()?;

        let line = serde_json::to_string(&entry)
            .map_err(|e| ReplayStoreError::Serialization(e.to_string()))?;

        let mut writer = self
            .writer
            .lock()
            .map_err(|e| ReplayStoreError::Other(format!("Lock poisoned: {}", e)))?;

        if let Some(ref mut file) = *writer {
            writeln!(file, "{}", line).map_err(|e| ReplayStoreError::Io(e.to_string()))?;
            file.flush().map_err(|e| ReplayStoreError::Io(e.to_string()))?;
        }

        Ok(())
    }

    async fn get(&self, id: &str) -> ReplayStoreResult<Option<ReplayEntry>> {
        let entries = self.read_all_entries()?;
        Ok(entries.into_iter().find(|e| e.id == id))
    }

    async fn list(&self, query: &ReplayQuery) -> ReplayStoreResult<Vec<ReplayEntry>> {
        let entries = self.read_all_entries()?;
        let mut filtered: Vec<ReplayEntry> =
            entries.into_iter().filter(|e| query.matches(e)).collect();

        if query.newest_first {
            filtered.sort_by(|a, b| b.recorded_at.cmp(&a.recorded_at));
        } else {
            filtered.sort_by(|a, b| a.recorded_at.cmp(&b.recorded_at));
        }

        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);

        Ok(filtered.into_iter().skip(offset).take(limit).collect())
    }

    async fn delete(&self, id: &str) -> ReplayStoreResult<bool> {
        let entries = self.read_all_entries()?;
        let before = entries.len();
        let filtered: Vec<ReplayEntry> = entries.into_iter().filter(|e| e.id != id).collect();
        let deleted = filtered.len() < before;

        if deleted {
            self.write_all_entries(&filtered)?;
        }

        Ok(deleted)
    }

    async fn count(&self) -> ReplayStoreResult<usize> {
        Ok(self.read_all_entries()?.len())
    }

    async fn clear(&self) -> ReplayStoreResult<()> {
        self.write_all_entries(&[])?;
        Ok(())
    }

    async fn delete_before(&self, timestamp_ms: u64) -> ReplayStoreResult<usize> {
        let entries = self.read_all_entries()?;
        let before = entries.len();
        let filtered: Vec<ReplayEntry> = entries
            .into_iter()
            .filter(|e| e.recorded_at >= timestamp_ms)
            .collect();
        let deleted = before - filtered.len();

        if deleted > 0 {
            self.write_all_entries(&filtered)?;
        }

        Ok(deleted)
    }

    fn clone_store(&self) -> Box<dyn ReplayStore> {
        Box::new(Self {
            config: self.config.clone(),
            writer: Mutex::new(None),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustapi_core::replay::{RecordedRequest, RecordedResponse, ReplayMeta};
    use tempfile::TempDir;

    fn make_entry(method: &str, path: &str, status: u16) -> ReplayEntry {
        ReplayEntry::new(
            RecordedRequest::new(method, path, path),
            RecordedResponse::new(status),
            ReplayMeta::new(),
        )
    }

    #[tokio::test]
    async fn test_fs_store_basic() {
        let tmp = TempDir::new().unwrap();
        let store = FsReplayStore::open(tmp.path()).unwrap();

        let entry = make_entry("GET", "/users", 200);
        let id = entry.id.clone();

        store.store(entry).await.unwrap();

        let retrieved = store.get(&id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().request.method, "GET");
    }

    #[tokio::test]
    async fn test_fs_store_list() {
        let tmp = TempDir::new().unwrap();
        let store = FsReplayStore::open(tmp.path()).unwrap();

        store.store(make_entry("GET", "/a", 200)).await.unwrap();
        store.store(make_entry("POST", "/b", 201)).await.unwrap();

        let all = store.list(&ReplayQuery::new()).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_fs_store_delete() {
        let tmp = TempDir::new().unwrap();
        let store = FsReplayStore::open(tmp.path()).unwrap();

        let entry = make_entry("GET", "/users", 200);
        let id = entry.id.clone();
        store.store(entry).await.unwrap();

        assert!(store.delete(&id).await.unwrap());
        assert_eq!(store.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_fs_store_clear() {
        let tmp = TempDir::new().unwrap();
        let store = FsReplayStore::open(tmp.path()).unwrap();

        store.store(make_entry("GET", "/a", 200)).await.unwrap();
        store.clear().await.unwrap();
        assert_eq!(store.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_fs_store_delete_before() {
        let tmp = TempDir::new().unwrap();
        let store = FsReplayStore::open(tmp.path()).unwrap();

        let mut e1 = make_entry("GET", "/a", 200);
        e1.recorded_at = 1000;
        let mut e2 = make_entry("GET", "/b", 200);
        e2.recorded_at = 3000;

        store.store(e1).await.unwrap();
        store.store(e2).await.unwrap();

        let deleted = store.delete_before(2000).await.unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(store.count().await.unwrap(), 1);
    }
}
