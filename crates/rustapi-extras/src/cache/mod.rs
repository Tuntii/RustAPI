//! Response Caching Middleware
//!
//! Provides in-memory caching for HTTP responses with:
//! - Configurable TTL and max entries
//! - ETag / If-None-Match support (304 Not Modified)
//! - Cache-Control header awareness (no-cache, no-store)
//! - Path skip lists and vary-by-header support
//! - Cache invalidation via `CacheHandle`
//!
//! Requires `cache` feature.
//!
//! # Example
//!
//! ```rust,ignore
//! use rustapi_extras::cache::{CacheLayer, CacheHandle};
//! use std::time::Duration;
//!
//! let (cache_layer, cache_handle) = CacheLayer::with_handle()
//!     .ttl(Duration::from_secs(300))
//!     .max_entries(1000)
//!     .skip_path("/health")
//!     .skip_path("/metrics")
//!     .build();
//!
//! let app = RustApi::new()
//!     .state(cache_handle.clone())  // share handle for invalidation
//!     .layer(cache_layer);
//!
//! // In a handler:
//! async fn update_user(handle: State<CacheHandle>) {
//!     handle.invalidate("/api/users");  // remove cached entry
//! }
//! ```

use bytes::Bytes;
use dashmap::DashMap;
use http_body_util::BodyExt;
use rustapi_core::{
    middleware::{BoxedNext, MiddlewareLayer},
    Request, Response, ResponseBody,
};
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Cache configuration
#[derive(Clone)]
pub struct CacheConfig {
    /// Time-to-live for cached items
    pub ttl: Duration,
    /// Maximum number of cached entries (0 = unlimited)
    pub max_entries: usize,
    /// Methods to cache (e.g., GET, HEAD)
    pub methods: Vec<String>,
    /// Paths to skip caching
    pub skip_paths: Vec<String>,
    /// Headers to include in cache key (Vary support)
    pub vary_headers: Vec<String>,
    /// Whether to generate ETag headers
    pub etag: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(60),
            max_entries: 10_000,
            methods: vec!["GET".to_string(), "HEAD".to_string()],
            skip_paths: vec!["/health".to_string()],
            vary_headers: Vec::new(),
            etag: true,
        }
    }
}

#[derive(Clone)]
struct CachedResponse {
    status: http::StatusCode,
    headers: http::HeaderMap,
    body: Bytes,
    etag: Option<String>,
    created_at: Instant,
}

/// Shared cache store
#[derive(Clone)]
struct CacheStore {
    entries: Arc<DashMap<String, CachedResponse>>,
    /// Insertion-order queue for LRU eviction
    order: Arc<Mutex<VecDeque<String>>>,
    max_entries: usize,
}

impl CacheStore {
    fn new(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            order: Arc::new(Mutex::new(VecDeque::new())),
            max_entries,
        }
    }

    fn get(&self, key: &str) -> Option<dashmap::mapref::one::Ref<'_, String, CachedResponse>> {
        self.entries.get(key)
    }

    fn insert(&self, key: String, value: CachedResponse) {
        // Evict oldest if at capacity
        if self.max_entries > 0 && self.entries.len() >= self.max_entries {
            if let Ok(mut order) = self.order.lock() {
                while self.entries.len() >= self.max_entries {
                    if let Some(oldest_key) = order.pop_front() {
                        self.entries.remove(&oldest_key);
                    } else {
                        break;
                    }
                }
            }
        }

        self.entries.insert(key.clone(), value);
        if let Ok(mut order) = self.order.lock() {
            order.push_back(key);
        }
    }

    fn remove(&self, key: &str) {
        self.entries.remove(key);
        if let Ok(mut order) = self.order.lock() {
            order.retain(|k| k != key);
        }
    }

    fn clear(&self) {
        self.entries.clear();
        if let Ok(mut order) = self.order.lock() {
            order.clear();
        }
    }

    fn invalidate_prefix(&self, prefix: &str) {
        let keys_to_remove: Vec<String> = self
            .entries
            .iter()
            .filter(|entry| {
                // Cache keys are "METHOD:URI", check the URI part
                entry
                    .key()
                    .split_once(':')
                    .is_some_and(|(_, uri)| uri.starts_with(prefix))
            })
            .map(|entry| entry.key().clone())
            .collect();

        for key in &keys_to_remove {
            self.entries.remove(key.as_str());
        }

        if !keys_to_remove.is_empty() {
            if let Ok(mut order) = self.order.lock() {
                order.retain(|k| !keys_to_remove.contains(k));
            }
        }
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

/// Handle for cache invalidation from handlers
///
/// Store this as application state to invalidate cached entries
/// from within route handlers.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_core::State;
/// use rustapi_extras::cache::CacheHandle;
///
/// async fn update_item(handle: State<CacheHandle>) -> impl IntoResponse {
///     // ... update item in database ...
///     handle.invalidate("/api/items");
///     "ok"
/// }
/// ```
#[derive(Clone)]
pub struct CacheHandle {
    store: CacheStore,
}

impl CacheHandle {
    /// Remove a specific path from cache (all methods)
    pub fn invalidate(&self, path: &str) {
        self.store.invalidate_prefix(path);
    }

    /// Remove a specific method+path from cache
    pub fn invalidate_exact(&self, method: &str, path: &str) {
        let key = format!("{}:{}", method, path);
        self.store.remove(&key);
    }

    /// Clear all cached entries
    pub fn clear(&self) {
        self.store.clear();
    }

    /// Get the number of currently cached items
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.store.len() == 0
    }
}

/// In-memory response cache layer
#[derive(Clone)]
pub struct CacheLayer {
    config: CacheConfig,
    store: CacheStore,
}

impl CacheLayer {
    /// Create a new cache layer with default settings
    pub fn new() -> Self {
        let config = CacheConfig::default();
        let store = CacheStore::new(config.max_entries);
        Self { config, store }
    }

    /// Create a builder that produces both a CacheLayer and a CacheHandle
    ///
    /// The CacheHandle can be stored as state for cache invalidation from handlers.
    pub fn with_handle() -> CacheBuilder {
        CacheBuilder {
            config: CacheConfig::default(),
        }
    }

    /// Get a handle for cache invalidation
    pub fn handle(&self) -> CacheHandle {
        CacheHandle {
            store: self.store.clone(),
        }
    }

    /// Set TTL
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.config.ttl = ttl;
        self
    }

    /// Set maximum number of cached entries.
    ///
    /// **Note:** calling this method replaces the underlying cache store, discarding
    /// any previously cached responses. It should therefore be called before the
    /// layer starts serving requests (i.e. during application setup), not
    /// at runtime.
    pub fn max_entries(mut self, max: usize) -> Self {
        self.config.max_entries = max;
        self.store = CacheStore::new(max);
        self
    }

    /// Add a method to cache
    pub fn add_method(mut self, method: &str) -> Self {
        if !self.config.methods.contains(&method.to_string()) {
            self.config.methods.push(method.to_string());
        }
        self
    }

    /// Add a path to skip
    pub fn skip_path(mut self, path: &str) -> Self {
        self.config.skip_paths.push(path.to_string());
        self
    }

    /// Add a header to vary by (include in cache key)
    pub fn vary_by(mut self, header: &str) -> Self {
        self.config.vary_headers.push(header.to_lowercase());
        self
    }

    /// Enable/disable ETag generation
    pub fn etag(mut self, enabled: bool) -> Self {
        self.config.etag = enabled;
        self
    }
}

impl Default for CacheLayer {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating CacheLayer + CacheHandle pair
pub struct CacheBuilder {
    config: CacheConfig,
}

impl CacheBuilder {
    /// Set TTL
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.config.ttl = ttl;
        self
    }

    /// Set maximum number of cached entries
    pub fn max_entries(mut self, max: usize) -> Self {
        self.config.max_entries = max;
        self
    }

    /// Add a path to skip
    pub fn skip_path(mut self, path: &str) -> Self {
        self.config.skip_paths.push(path.to_string());
        self
    }

    /// Add a header to vary by
    pub fn vary_by(mut self, header: &str) -> Self {
        self.config.vary_headers.push(header.to_lowercase());
        self
    }

    /// Enable/disable ETag generation
    pub fn etag(mut self, enabled: bool) -> Self {
        self.config.etag = enabled;
        self
    }

    /// Build the CacheLayer and CacheHandle pair
    pub fn build(self) -> (CacheLayer, CacheHandle) {
        let store = CacheStore::new(self.config.max_entries);
        let layer = CacheLayer {
            config: self.config,
            store: store.clone(),
        };
        let handle = CacheHandle { store };
        (layer, handle)
    }
}

/// Generate a simple ETag from body bytes (FNV-1a hash)
fn generate_etag(body: &[u8]) -> String {
    // FNV-1a 64-bit hash – fast, no crypto dependency needed
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in body {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("\"{:016x}\"", hash)
}

/// Build a cache key from method, URI, and vary headers.
///
/// Header names and values are percent-encoded so that the delimiter
/// characters (`|`, `=`, `%`) cannot appear unescaped inside them,
/// preventing ambiguous or colliding cache keys.
fn build_cache_key(method: &str, uri: &str, req: &Request, vary_headers: &[String]) -> String {
    if vary_headers.is_empty() {
        return format!("{}:{}", method, uri);
    }

    // Encode characters that are used as delimiters in the key format.
    fn encode_part(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for b in s.bytes() {
            match b {
                b'%' => out.push_str("%25"),
                b'|' => out.push_str("%7C"),
                b'=' => out.push_str("%3D"),
                _ => out.push(b as char),
            }
        }
        out
    }

    let mut key = format!("{}:{}", method, uri);
    for header_name in vary_headers {
        if let Some(value) = req.headers().get(header_name.as_str()) {
            if let Ok(s) = value.to_str() {
                key.push('|');
                key.push_str(&encode_part(header_name));
                key.push('=');
                key.push_str(&encode_part(s));
            }
        }
    }
    key
}

/// Check if the request has Cache-Control directives that prevent caching
fn should_skip_cache(req: &Request) -> bool {
    if let Some(cc) = req.headers().get(http::header::CACHE_CONTROL) {
        if let Ok(s) = cc.to_str() {
            let lower = s.to_lowercase();
            return lower.contains("no-cache") || lower.contains("no-store");
        }
    }
    false
}

impl MiddlewareLayer for CacheLayer {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let config = self.config.clone();
        let store = self.store.clone();

        Box::pin(async move {
            let method = req.method().to_string();
            let uri = req.uri().to_string();

            // Check if cacheable
            if !config.methods.contains(&method)
                || config.skip_paths.iter().any(|p| uri.starts_with(p))
                || should_skip_cache(&req)
            {
                return next(req).await;
            }

            // Build cache key (includes vary headers)
            let key = build_cache_key(&method, &uri, &req, &config.vary_headers);

            // Check for If-None-Match (ETag conditional)
            let if_none_match = req
                .headers()
                .get(http::header::IF_NONE_MATCH)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            // Look up cached entry
            if let Some(entry) = store.get(&key) {
                if entry.created_at.elapsed() < config.ttl {
                    // ETag: return 304 Not Modified if client has the same ETag
                    if let (Some(ref etag), Some(ref client_etag)) = (&entry.etag, &if_none_match) {
                        if etag == client_etag {
                            return http::Response::builder()
                                .status(http::StatusCode::NOT_MODIFIED)
                                .header("ETag", etag.as_str())
                                .header("X-Cache", "HIT")
                                .body(ResponseBody::Full(http_body_util::Full::new(Bytes::new())))
                                .unwrap();
                        }
                    }

                    // Cache hit – rebuild response
                    let mut builder = http::Response::builder().status(entry.status);
                    for (k, v) in &entry.headers {
                        builder = builder.header(k, v);
                    }
                    builder = builder.header("X-Cache", "HIT");
                    if let Some(ref etag) = entry.etag {
                        builder = builder.header("ETag", etag.as_str());
                    }

                    return builder
                        .body(ResponseBody::Full(http_body_util::Full::new(
                            entry.body.clone(),
                        )))
                        .unwrap();
                } else {
                    // Expired
                    drop(entry);
                    store.remove(&key);
                }
            }

            // Cache miss: execute request
            let response = next(req).await;

            // Only cache successful responses
            if response.status().is_success() {
                let (parts, body) = response.into_parts();

                // Buffer the body
                match body.collect().await {
                    Ok(collected) => {
                        let bytes = collected.to_bytes();

                        // Generate ETag if enabled
                        let etag = if config.etag {
                            Some(generate_etag(&bytes))
                        } else {
                            None
                        };

                        let cached = CachedResponse {
                            status: parts.status,
                            headers: parts.headers.clone(),
                            body: bytes.clone(),
                            etag: etag.clone(),
                            created_at: Instant::now(),
                        };

                        store.insert(key, cached);

                        let mut response = http::Response::from_parts(
                            parts,
                            ResponseBody::Full(http_body_util::Full::new(bytes)),
                        );
                        response
                            .headers_mut()
                            .insert("X-Cache", "MISS".parse().unwrap());
                        if let Some(etag) = etag {
                            if let Ok(val) = etag.parse() {
                                response.headers_mut().insert(http::header::ETAG, val);
                            }
                        }
                        return response;
                    }
                    Err(_) => {
                        return http::Response::builder()
                            .status(500)
                            .body(ResponseBody::Full(http_body_util::Full::new(Bytes::from(
                                "Error buffering response for cache",
                            ))))
                            .unwrap();
                    }
                }
            }

            response
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_etag_generation() {
        let body = b"Hello, World!";
        let etag = generate_etag(body);
        assert!(etag.starts_with('"'));
        assert!(etag.ends_with('"'));
        assert_eq!(etag.len(), 18); // 16 hex chars + 2 quotes

        // Same input → same ETag
        assert_eq!(generate_etag(body), generate_etag(body));

        // Different input → different ETag
        assert_ne!(generate_etag(body), generate_etag(b"Different body"));
    }

    #[test]
    fn test_cache_store_eviction() {
        let store = CacheStore::new(2);

        let make_entry = || CachedResponse {
            status: http::StatusCode::OK,
            headers: http::HeaderMap::new(),
            body: Bytes::from("test"),
            etag: None,
            created_at: Instant::now(),
        };

        store.insert("key1".to_string(), make_entry());
        store.insert("key2".to_string(), make_entry());
        assert_eq!(store.len(), 2);

        // Insert third → should evict key1 (oldest)
        store.insert("key3".to_string(), make_entry());
        assert_eq!(store.len(), 2);
        assert!(store.get("key1").is_none());
        assert!(store.get("key3").is_some());
    }

    #[test]
    fn test_cache_handle_invalidation() {
        let store = CacheStore::new(100);

        let make_entry = || CachedResponse {
            status: http::StatusCode::OK,
            headers: http::HeaderMap::new(),
            body: Bytes::from("test"),
            etag: None,
            created_at: Instant::now(),
        };

        store.insert("GET:/api/users".to_string(), make_entry());
        store.insert("GET:/api/users/1".to_string(), make_entry());
        store.insert("GET:/api/posts".to_string(), make_entry());
        assert_eq!(store.len(), 3);

        let handle = CacheHandle {
            store: store.clone(),
        };
        handle.invalidate("/api/users");

        assert_eq!(store.len(), 1);
        assert!(store.get("GET:/api/posts").is_some());
    }

    #[test]
    fn test_cache_handle_clear() {
        let store = CacheStore::new(100);

        let make_entry = || CachedResponse {
            status: http::StatusCode::OK,
            headers: http::HeaderMap::new(),
            body: Bytes::from("test"),
            etag: None,
            created_at: Instant::now(),
        };

        store.insert("key1".to_string(), make_entry());
        store.insert("key2".to_string(), make_entry());

        let handle = CacheHandle {
            store: store.clone(),
        };
        handle.clear();
        assert!(handle.is_empty());
    }

    #[test]
    fn test_cache_config_defaults() {
        let config = CacheConfig::default();
        assert_eq!(config.ttl, Duration::from_secs(60));
        assert_eq!(config.max_entries, 10_000);
        assert!(config.etag);
        assert_eq!(config.methods, vec!["GET", "HEAD"]);
    }

    #[test]
    fn test_builder_produces_handle() {
        let (layer, handle) = CacheLayer::with_handle()
            .ttl(Duration::from_secs(120))
            .max_entries(500)
            .skip_path("/debug")
            .vary_by("accept-language")
            .etag(false)
            .build();

        assert_eq!(layer.config.ttl, Duration::from_secs(120));
        assert_eq!(layer.config.max_entries, 500);
        assert!(layer.config.skip_paths.contains(&"/debug".to_string()));
        assert!(layer
            .config
            .vary_headers
            .contains(&"accept-language".to_string()));
        assert!(!layer.config.etag);
        assert!(handle.is_empty());
    }
}
