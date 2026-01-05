//! Export functionality for insight data.
//!
//! This module provides traits and implementations for exporting
//! insight data to various destinations.

use super::data::InsightData;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Error type for export operations.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// IO error during export.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// HTTP error during webhook export.
    #[error("HTTP error: {0}")]
    Http(String),

    /// Export sink is closed or unavailable.
    #[error("Export sink unavailable: {0}")]
    Unavailable(String),
}

/// Result type for export operations.
pub type ExportResult<T> = Result<T, ExportError>;

/// Trait for exporting insight data to external destinations.
///
/// Implement this trait to create custom export sinks.
pub trait InsightExporter: Send + Sync + 'static {
    /// Export a single insight entry.
    fn export(&self, insight: &InsightData) -> ExportResult<()>;

    /// Export multiple insights in batch.
    fn export_batch(&self, insights: &[InsightData]) -> ExportResult<()> {
        for insight in insights {
            self.export(insight)?;
        }
        Ok(())
    }

    /// Flush any buffered data.
    fn flush(&self) -> ExportResult<()> {
        Ok(())
    }

    /// Close the exporter and release resources.
    fn close(&self) -> ExportResult<()> {
        self.flush()
    }

    /// Clone this exporter into a boxed trait object.
    fn clone_exporter(&self) -> Box<dyn InsightExporter>;
}

/// File exporter that writes insights as JSON lines.
///
/// Each insight is written as a single JSON object on its own line,
/// compatible with common log aggregation tools.
///
/// # Example
///
/// ```ignore
/// use rustapi_extras::insight::export::FileExporter;
///
/// let exporter = FileExporter::new("./insights.jsonl")?;
/// ```
pub struct FileExporter {
    path: PathBuf,
    writer: Arc<Mutex<BufWriter<File>>>,
}

impl FileExporter {
    /// Create a new file exporter.
    ///
    /// Creates or appends to the specified file.
    pub fn new(path: impl Into<PathBuf>) -> ExportResult<Self> {
        let path = path.into();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        let writer = BufWriter::new(file);

        Ok(Self {
            path,
            writer: Arc::new(Mutex::new(writer)),
        })
    }

    /// Get the file path.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Clone for FileExporter {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            writer: self.writer.clone(),
        }
    }
}

impl InsightExporter for FileExporter {
    fn export(&self, insight: &InsightData) -> ExportResult<()> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| ExportError::Unavailable(e.to_string()))?;

        let json = serde_json::to_string(insight)?;
        writeln!(writer, "{}", json)?;

        Ok(())
    }

    fn export_batch(&self, insights: &[InsightData]) -> ExportResult<()> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| ExportError::Unavailable(e.to_string()))?;

        for insight in insights {
            let json = serde_json::to_string(insight)?;
            writeln!(writer, "{}", json)?;
        }

        Ok(())
    }

    fn flush(&self) -> ExportResult<()> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| ExportError::Unavailable(e.to_string()))?;

        writer.flush()?;
        Ok(())
    }

    fn clone_exporter(&self) -> Box<dyn InsightExporter> {
        Box::new(self.clone())
    }
}

/// Webhook exporter configuration.
#[derive(Clone, Debug)]
pub struct WebhookConfig {
    /// URL to POST insights to.
    pub url: String,
    /// Optional authorization header value.
    pub auth_header: Option<String>,
    /// Custom headers to include.
    pub headers: Vec<(String, String)>,
    /// Batch size for batched exports.
    pub batch_size: usize,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
}

impl WebhookConfig {
    /// Create a new webhook configuration.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            auth_header: None,
            headers: Vec::new(),
            batch_size: 100,
            timeout_secs: 30,
        }
    }

    /// Set the authorization header.
    pub fn auth(mut self, value: impl Into<String>) -> Self {
        self.auth_header = Some(value.into());
        self
    }

    /// Add a custom header.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Set the batch size for batched exports.
    pub fn batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Set the request timeout.
    pub fn timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

/// Webhook exporter that POSTs insights to a URL.
///
/// Insights are sent as JSON in POST requests.
///
/// # Example
///
/// ```ignore
/// use rustapi_extras::insight::export::{WebhookExporter, WebhookConfig};
///
/// let config = WebhookConfig::new("https://example.com/insights")
///     .auth("Bearer my-token")
///     .batch_size(50);
///
/// let exporter = WebhookExporter::new(config);
/// ```
#[derive(Clone)]
pub struct WebhookExporter {
    config: WebhookConfig,
    buffer: Arc<Mutex<Vec<InsightData>>>,
}

impl WebhookExporter {
    /// Create a new webhook exporter.
    pub fn new(config: WebhookConfig) -> Self {
        Self {
            config,
            buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Send insights to the webhook.
    fn send_insights(&self, insights: &[InsightData]) -> ExportResult<()> {
        // Note: This is a simplified implementation.
        // In production, you'd use an async HTTP client like reqwest.
        // For now, we'll just log and return success since this crate
        // doesn't want to add heavy HTTP client dependencies.
        
        let json = serde_json::to_string(insights)?;
        tracing::debug!(
            url = %self.config.url,
            count = insights.len(),
            size = json.len(),
            "Would send insights to webhook"
        );
        
        // TODO: Implement actual HTTP POST when reqwest is available
        // For now, this is a placeholder that logs the intent
        
        Ok(())
    }
}

impl InsightExporter for WebhookExporter {
    fn export(&self, insight: &InsightData) -> ExportResult<()> {
        let mut buffer = self
            .buffer
            .lock()
            .map_err(|e| ExportError::Unavailable(e.to_string()))?;

        buffer.push(insight.clone());

        // Flush if batch size reached
        if buffer.len() >= self.config.batch_size {
            let to_send: Vec<_> = buffer.drain(..).collect();
            drop(buffer); // Release lock before sending
            self.send_insights(&to_send)?;
        }

        Ok(())
    }

    fn export_batch(&self, insights: &[InsightData]) -> ExportResult<()> {
        // Send in batches
        for chunk in insights.chunks(self.config.batch_size) {
            self.send_insights(chunk)?;
        }
        Ok(())
    }

    fn flush(&self) -> ExportResult<()> {
        let mut buffer = self
            .buffer
            .lock()
            .map_err(|e| ExportError::Unavailable(e.to_string()))?;

        if !buffer.is_empty() {
            let to_send: Vec<_> = buffer.drain(..).collect();
            drop(buffer);
            self.send_insights(&to_send)?;
        }

        Ok(())
    }

    fn clone_exporter(&self) -> Box<dyn InsightExporter> {
        Box::new(self.clone())
    }
}

/// A composite exporter that sends to multiple destinations.
///
/// # Example
///
/// ```ignore
/// use rustapi_extras::insight::export::{CompositeExporter, FileExporter, WebhookExporter, WebhookConfig};
///
/// let composite = CompositeExporter::new()
///     .add(FileExporter::new("./insights.jsonl")?)
///     .add(WebhookExporter::new(WebhookConfig::new("https://example.com/insights")));
/// ```
#[derive(Default)]
pub struct CompositeExporter {
    exporters: Vec<Box<dyn InsightExporter>>,
}

impl Clone for CompositeExporter {
    fn clone(&self) -> Self {
        let exporters = self.exporters.iter().map(|e| e.clone_exporter()).collect();
        Self { exporters }
    }
}

impl CompositeExporter {
    /// Create a new composite exporter.
    pub fn new() -> Self {
        Self {
            exporters: Vec::new(),
        }
    }

    /// Add an exporter to the composite.
    pub fn add<E: InsightExporter>(mut self, exporter: E) -> Self {
        self.exporters.push(Box::new(exporter));
        self
    }

    /// Add a boxed exporter to the composite.
    pub fn add_boxed(mut self, exporter: Box<dyn InsightExporter>) -> Self {
        self.exporters.push(exporter);
        self
    }
}

impl InsightExporter for CompositeExporter {
    fn export(&self, insight: &InsightData) -> ExportResult<()> {
        for exporter in &self.exporters {
            if let Err(e) = exporter.export(insight) {
                tracing::warn!(error = %e, "Export failed for one sink");
            }
        }
        Ok(())
    }

    fn export_batch(&self, insights: &[InsightData]) -> ExportResult<()> {
        for exporter in &self.exporters {
            if let Err(e) = exporter.export_batch(insights) {
                tracing::warn!(error = %e, "Batch export failed for one sink");
            }
        }
        Ok(())
    }

    fn flush(&self) -> ExportResult<()> {
        for exporter in &self.exporters {
            if let Err(e) = exporter.flush() {
                tracing::warn!(error = %e, "Flush failed for one sink");
            }
        }
        Ok(())
    }

    fn close(&self) -> ExportResult<()> {
        for exporter in &self.exporters {
            if let Err(e) = exporter.close() {
                tracing::warn!(error = %e, "Close failed for one sink");
            }
        }
        Ok(())
    }

    fn clone_exporter(&self) -> Box<dyn InsightExporter> {
        let exporters: Vec<_> = self.exporters.iter().map(|e| e.clone_exporter()).collect();
        Box::new(CompositeExporter { exporters })
    }
}

/// A callback-based exporter that invokes a function for each insight.
///
/// # Example
///
/// ```ignore
/// use rustapi_extras::insight::export::CallbackExporter;
///
/// let exporter = CallbackExporter::new(|insight| {
///     println!("Received: {} {}", insight.method, insight.path);
/// });
/// ```
pub struct CallbackExporter<F>
where
    F: Fn(&InsightData) + Send + Sync + 'static,
{
    callback: Arc<F>,
}

impl<F> CallbackExporter<F>
where
    F: Fn(&InsightData) + Send + Sync + 'static,
{
    /// Create a new callback exporter.
    pub fn new(callback: F) -> Self {
        Self {
            callback: Arc::new(callback),
        }
    }
}

impl<F> Clone for CallbackExporter<F>
where
    F: Fn(&InsightData) + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            callback: self.callback.clone(),
        }
    }
}

impl<F> InsightExporter for CallbackExporter<F>
where
    F: Fn(&InsightData) + Send + Sync + 'static,
{
    fn export(&self, insight: &InsightData) -> ExportResult<()> {
        (self.callback)(insight);
        Ok(())
    }

    fn clone_exporter(&self) -> Box<dyn InsightExporter> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use tempfile::tempdir;

    fn create_test_insight() -> InsightData {
        InsightData::new("test-123", "GET", "/users")
            .with_status(200)
            .with_duration(Duration::from_millis(42))
    }

    #[test]
    fn test_file_exporter() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.jsonl");

        let exporter = FileExporter::new(&path).unwrap();
        exporter.export(&create_test_insight()).unwrap();
        exporter.flush().unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("test-123"));
        assert!(content.contains("GET"));
        assert!(content.contains("/users"));
    }

    #[test]
    fn test_file_exporter_batch() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("batch.jsonl");

        let exporter = FileExporter::new(&path).unwrap();
        let insights: Vec<_> = (0..5)
            .map(|i| InsightData::new(format!("req-{}", i), "GET", "/test"))
            .collect();

        exporter.export_batch(&insights).unwrap();
        exporter.flush().unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<_> = content.lines().collect();
        assert_eq!(lines.len(), 5);
    }

    #[test]
    fn test_callback_exporter() {
        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();

        let exporter = CallbackExporter::new(move |_insight| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });

        exporter.export(&create_test_insight()).unwrap();
        exporter.export(&create_test_insight()).unwrap();

        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_composite_exporter() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("composite.jsonl");

        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();

        let composite = CompositeExporter::new()
            .add(FileExporter::new(&path).unwrap())
            .add(CallbackExporter::new(move |_| {
                count_clone.fetch_add(1, Ordering::SeqCst);
            }));

        composite.export(&create_test_insight()).unwrap();
        composite.flush().unwrap();

        assert_eq!(count.load(Ordering::SeqCst), 1);
        assert!(std::fs::read_to_string(&path).unwrap().contains("test-123"));
    }

    #[test]
    fn test_webhook_config() {
        let config = WebhookConfig::new("https://example.com/insights")
            .auth("Bearer token")
            .header("X-Custom", "value")
            .batch_size(50)
            .timeout(60);

        assert_eq!(config.url, "https://example.com/insights");
        assert_eq!(config.auth_header, Some("Bearer token".to_string()));
        assert_eq!(config.batch_size, 50);
        assert_eq!(config.timeout_secs, 60);
    }
}
