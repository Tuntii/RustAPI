//! HTTP client for replaying recorded requests against a target server.

use rustapi_core::replay::{RecordedResponse, ReplayEntry};
use std::collections::HashMap;

/// Error from replay HTTP client operations.
#[derive(Debug, thiserror::Error)]
pub enum ReplayClientError {
    /// HTTP request error.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Invalid URL.
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}

/// HTTP client for replaying recorded requests against a target server.
///
/// Takes a [`ReplayEntry`] and sends the recorded request to a target URL,
/// capturing the response as a [`RecordedResponse`].
pub struct ReplayClient {
    http: reqwest::Client,
}

impl ReplayClient {
    /// Create a new replay client.
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }

    /// Replay the recorded request against the given target base URL.
    ///
    /// The recorded request path is appended to `target_base_url`.
    /// Returns the target server's response as a [`RecordedResponse`].
    pub async fn replay(
        &self,
        entry: &ReplayEntry,
        target_base_url: &str,
    ) -> Result<RecordedResponse, ReplayClientError> {
        let base = target_base_url.trim_end_matches('/');
        let path = &entry.request.uri;
        let url = format!("{}{}", base, path);

        let method: reqwest::Method = entry
            .request
            .method
            .parse()
            .map_err(|_| ReplayClientError::InvalidUrl(format!("Invalid method: {}", entry.request.method)))?;

        let mut builder = self.http.request(method, &url);

        // Add recorded headers (skip host, content-length as reqwest manages these)
        for (key, value) in &entry.request.headers {
            let key_lower = key.to_lowercase();
            if key_lower == "host" || key_lower == "content-length" {
                continue;
            }
            builder = builder.header(key, value);
        }

        // Add recorded body
        if let Some(ref body) = entry.request.body {
            builder = builder.body(body.clone());
        }

        let response = builder.send().await?;

        let status = response.status().as_u16();
        let mut headers = HashMap::new();
        for (key, value) in response.headers() {
            if let Ok(v) = value.to_str() {
                headers.insert(key.as_str().to_string(), v.to_string());
            }
        }

        let body_bytes = response.bytes().await?;
        let body_size = body_bytes.len();
        let body = String::from_utf8(body_bytes.to_vec()).ok();

        Ok(RecordedResponse {
            status,
            headers,
            body,
            body_size,
            body_truncated: false,
        })
    }
}

impl Default for ReplayClient {
    fn default() -> Self {
        Self::new()
    }
}
