//! File-based audit store implementation

use super::event::AuditEvent;
use super::query::AuditQuery;
use super::store::{AuditError, AuditResult, AuditStore};
use std::fs::{File, OpenOptions};
use std::future::Future;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

/// Configuration for file-based audit store.
#[derive(Debug, Clone)]
pub struct FileAuditStoreConfig {
    /// Path to the audit log file.
    pub file_path: PathBuf,
    /// Maximum file size in bytes before rotation.
    pub max_file_size: Option<u64>,
    /// Whether to create the file if it doesn't exist.
    pub create_if_missing: bool,
    /// Whether to append to existing file.
    pub append: bool,
}

impl FileAuditStoreConfig {
    /// Create a new configuration for the given file path.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            file_path: path.into(),
            max_file_size: Some(100 * 1024 * 1024), // 100MB default
            create_if_missing: true,
            append: true,
        }
    }

    /// Set maximum file size before rotation.
    pub fn max_size(mut self, bytes: u64) -> Self {
        self.max_file_size = Some(bytes);
        self
    }

    /// Disable file size limit.
    pub fn no_size_limit(mut self) -> Self {
        self.max_file_size = None;
        self
    }
}

/// Internal state for FileAuditStore
#[derive(Debug)]
struct FileAuditStoreInner {
    config: FileAuditStoreConfig,
    writer: Mutex<Option<File>>,
}

/// File-based audit store (JSON Lines format).
#[derive(Clone, Debug)]
pub struct FileAuditStore {
    inner: Arc<FileAuditStoreInner>,
}

impl FileAuditStore {
    /// Create a new file-based audit store.
    pub fn new(config: FileAuditStoreConfig) -> AuditResult<Self> {
        let inner = FileAuditStoreInner {
            config,
            writer: Mutex::new(None),
        };
        let store = Self {
            inner: Arc::new(inner),
        };
        store.open_writer()?;
        Ok(store)
    }

    /// Create a store for the given file path with default configuration.
    pub fn open(path: impl Into<PathBuf>) -> AuditResult<Self> {
        Self::new(FileAuditStoreConfig::new(path))
    }

    /// Open or create the file writer.
    fn open_writer(&self) -> AuditResult<()> {
        let mut writer = self
            .inner
            .writer
            .lock()
            .map_err(|e| AuditError::WriteError(format!("Failed to acquire lock: {}", e)))?;

        // Create parent directories if they don't exist
        if let Some(parent) = self.inner.config.file_path.parent() {
            if !parent.exists() && self.inner.config.create_if_missing {
                std::fs::create_dir_all(parent).map_err(|e| {
                    AuditError::IoError(format!("Failed to create directories: {}", e))
                })?;
            }
        }

        let file = OpenOptions::new()
            .create(self.inner.config.create_if_missing)
            .append(self.inner.config.append)
            .write(true)
            .open(&self.inner.config.file_path)
            .map_err(|e| AuditError::IoError(format!("Failed to open file: {}", e)))?;

        *writer = Some(file);
        Ok(())
    }

    /// Check if rotation is needed and perform it.
    fn check_rotation(&self) -> AuditResult<()> {
        if let Some(max_size) = self.inner.config.max_file_size {
            if let Ok(metadata) = std::fs::metadata(&self.inner.config.file_path) {
                if metadata.len() >= max_size {
                    self.rotate()?;
                }
            }
        }
        Ok(())
    }

    /// Rotate the log file.
    fn rotate(&self) -> AuditResult<()> {
        let mut writer = self
            .inner
            .writer
            .lock()
            .map_err(|e| AuditError::WriteError(format!("Failed to acquire lock: {}", e)))?;

        // Close current file
        *writer = None;

        // Generate rotated filename with timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let rotated_path = self
            .inner
            .config
            .file_path
            .with_extension(format!("{}.log", timestamp));

        // Rename current file
        std::fs::rename(&self.inner.config.file_path, &rotated_path)
            .map_err(|e| AuditError::IoError(format!("Failed to rotate file: {}", e)))?;

        // Open new file
        drop(writer);
        self.open_writer()?;

        Ok(())
    }

    /// Read all events from the file.
    fn read_all_events(&self) -> AuditResult<Vec<AuditEvent>> {
        let path = &self.inner.config.file_path;

        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(path)
            .map_err(|e| AuditError::IoError(format!("Failed to open file for reading: {}", e)))?;

        let reader = BufReader::new(file);
        let mut events = Vec::new();

        for line in reader.lines() {
            let line =
                line.map_err(|e| AuditError::IoError(format!("Failed to read line: {}", e)))?;

            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<AuditEvent>(&line) {
                Ok(event) => events.push(event),
                Err(e) => {
                    // Log warning but continue (corrupted line)
                    tracing::warn!("Failed to parse audit event: {}", e);
                }
            }
        }

        Ok(events)
    }
}

impl AuditStore for FileAuditStore {
    fn log(&self, event: AuditEvent) -> AuditResult<()> {
        self.check_rotation()?;

        let mut writer = self
            .inner
            .writer
            .lock()
            .map_err(|e| AuditError::WriteError(format!("Failed to acquire lock: {}", e)))?;

        let file = writer
            .as_mut()
            .ok_or_else(|| AuditError::WriteError("File not open".to_string()))?;

        let json = serde_json::to_string(&event)
            .map_err(|e| AuditError::SerializationError(e.to_string()))?;

        writeln!(file, "{}", json)
            .map_err(|e| AuditError::IoError(format!("Failed to write: {}", e)))?;

        Ok(())
    }

    fn log_async(
        &self,
        event: AuditEvent,
    ) -> Pin<Box<dyn Future<Output = AuditResult<()>> + Send + '_>> {
        let store = self.clone();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || store.log(event))
                .await
                .map_err(|e| AuditError::IoError(format!("Task join error: {}", e)))?
        })
    }

    fn get(&self, id: &str) -> AuditResult<Option<AuditEvent>> {
        let events = self.read_all_events()?;
        Ok(events.into_iter().find(|e| e.id == id))
    }

    fn execute_query(&self, query: &AuditQuery) -> AuditResult<Vec<AuditEvent>> {
        let events = self.read_all_events()?;

        let mut results: Vec<AuditEvent> =
            events.into_iter().filter(|e| query.matches(e)).collect();

        // Sort by timestamp
        if query.newest_first {
            results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        } else {
            results.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        }

        // Apply offset and limit
        let offset = query.offset.unwrap_or(0);
        let results: Vec<AuditEvent> = results.into_iter().skip(offset).collect();

        let results = if let Some(limit) = query.limit {
            results.into_iter().take(limit).collect()
        } else {
            results
        };

        Ok(results)
    }

    fn count(&self, query: &AuditQuery) -> AuditResult<usize> {
        let events = self.read_all_events()?;
        Ok(events.iter().filter(|e| query.matches(e)).count())
    }

    fn total_count(&self) -> AuditResult<usize> {
        let events = self.read_all_events()?;
        Ok(events.len())
    }

    fn clear(&self) -> AuditResult<()> {
        let mut writer = self
            .inner
            .writer
            .lock()
            .map_err(|e| AuditError::WriteError(format!("Failed to acquire lock: {}", e)))?;

        *writer = None;

        // Truncate the file
        File::create(&self.inner.config.file_path)
            .map_err(|e| AuditError::IoError(format!("Failed to clear file: {}", e)))?;

        // Reopen
        drop(writer);
        self.open_writer()?;

        Ok(())
    }

    fn flush(&self) -> AuditResult<()> {
        let mut writer = self
            .inner
            .writer
            .lock()
            .map_err(|e| AuditError::WriteError(format!("Failed to acquire lock: {}", e)))?;

        if let Some(ref mut file) = *writer {
            file.flush()
                .map_err(|e| AuditError::IoError(format!("Failed to flush: {}", e)))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditAction;
    use tempfile::TempDir;

    fn temp_store() -> (FileAuditStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("audit.log");
        let store = FileAuditStore::open(&path).unwrap();
        (store, dir)
    }

    #[test]
    fn test_file_store_log_and_get() {
        let (store, _dir) = temp_store();

        let event = AuditEvent::new(AuditAction::Create)
            .resource("users", "user-123")
            .actor("admin");

        let id = event.id.clone();
        store.log(event).unwrap();
        store.flush().unwrap();

        let retrieved = store.get(&id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().actor_id, Some("admin".to_string()));
    }

    #[test]
    fn test_file_store_query() {
        let (store, _dir) = temp_store();

        store
            .log(AuditEvent::new(AuditAction::Create).actor("alice"))
            .unwrap();
        store
            .log(AuditEvent::new(AuditAction::Read).actor("bob"))
            .unwrap();
        store
            .log(AuditEvent::new(AuditAction::Create).actor("alice"))
            .unwrap();
        store.flush().unwrap();

        let results = store.query().actor("alice").execute().unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_file_store_persistence() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("audit.log");

        // Write events
        {
            let store = FileAuditStore::open(&path).unwrap();
            store
                .log(AuditEvent::new(AuditAction::Create).actor("alice"))
                .unwrap();
            store
                .log(AuditEvent::new(AuditAction::Read).actor("bob"))
                .unwrap();
            store.flush().unwrap();
        }

        // Read back
        {
            let store = FileAuditStore::open(&path).unwrap();
            assert_eq!(store.total_count().unwrap(), 2);
        }
    }

    #[test]
    fn test_file_store_clear() {
        let (store, _dir) = temp_store();

        store.log(AuditEvent::new(AuditAction::Create)).unwrap();
        store.log(AuditEvent::new(AuditAction::Read)).unwrap();
        store.flush().unwrap();

        assert_eq!(store.total_count().unwrap(), 2);

        store.clear().unwrap();

        assert_eq!(store.total_count().unwrap(), 0);
    }
}
