//! Background retention cleanup job for replay entries.

use rustapi_core::replay::ReplayStore;
use std::sync::Arc;
use std::time::Duration;

/// Background task that periodically deletes expired replay entries.
pub struct RetentionJob;

impl RetentionJob {
    /// Spawn a background task that periodically deletes entries older than TTL.
    ///
    /// # Arguments
    ///
    /// * `store` - The replay store to clean up.
    /// * `ttl_secs` - Time-to-live in seconds. Entries older than this are deleted.
    /// * `check_interval` - How often to check for expired entries.
    pub fn spawn(
        store: Arc<dyn ReplayStore>,
        ttl_secs: u64,
        check_interval: Duration,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(check_interval).await;

                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                let cutoff = now_ms.saturating_sub(ttl_secs * 1000);

                match store.delete_before(cutoff).await {
                    Ok(count) if count > 0 => {
                        tracing::info!(deleted = count, "Replay retention cleanup");
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Replay retention cleanup failed");
                    }
                    _ => {}
                }
            }
        })
    }
}
