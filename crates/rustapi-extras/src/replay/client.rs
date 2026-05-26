//! HTTP client for replaying recorded requests against a target server.

use rustapi_core::replay::{RecordedResponse, ReplayEntry};
use std::collections::HashMap;
use std::time::Duration;

/// Error from replay HTTP client operations.
#[derive(Debug)]
pub enum ReplayClientError {
    /// HTTP request error.
    Http(reqwest::Error),
    /// Invalid URL.
    InvalidUrl(String),
}

impl std::fmt::Display for ReplayClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(e) => write!(f, "HTTP error: {}", e),
            Self::InvalidUrl(url) => write!(f, "Invalid URL: {}", url),
        }
    }
}

impl std::error::Error for ReplayClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Http(e) => Some(e),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for ReplayClientError {
    fn from(e: reqwest::Error) -> Self {
        Self::Http(e)
    }
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
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10))
            .build()
            .expect("failed to build replay HTTP client");

        Self { http }
    }

    /// Create a replay client from an existing reqwest client.
    pub fn with_client(http: reqwest::Client) -> Self {
        Self { http }
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
        self.replay_with_limit(entry, target_base_url, None).await
    }

    /// Replay a request and cap the captured replayed response body.
    pub async fn replay_with_limit(
        &self,
        entry: &ReplayEntry,
        target_base_url: &str,
        max_response_body: Option<usize>,
    ) -> Result<RecordedResponse, ReplayClientError> {
        let url = replay_url(target_base_url, &entry.request.uri)?;
        let method: reqwest::Method = entry.request.method.parse().map_err(|_| {
            ReplayClientError::InvalidUrl(format!("Invalid method: {}", entry.request.method))
        })?;

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
        let (body, body_size, body_truncated) =
            response_body_from_bytes(&body_bytes, max_response_body);

        Ok(RecordedResponse {
            status,
            headers,
            body,
            body_size,
            body_truncated,
        })
    }
}

fn replay_url(target_base_url: &str, recorded_uri: &str) -> Result<String, ReplayClientError> {
    let trimmed = target_base_url.trim();
    let parsed = reqwest::Url::parse(trimmed)
        .map_err(|_| ReplayClientError::InvalidUrl(target_base_url.to_string()))?;

    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(ReplayClientError::InvalidUrl(target_base_url.to_string()));
    }

    if parsed.query().is_some() || parsed.fragment().is_some() {
        return Err(ReplayClientError::InvalidUrl(
            "target URL must not include query or fragment".to_string(),
        ));
    }

    let base = trimmed.trim_end_matches('/');
    let path = recorded_path_and_query(recorded_uri);
    Ok(format!("{base}{path}"))
}

fn recorded_path_and_query(recorded_uri: &str) -> String {
    if let Ok(uri) = recorded_uri.parse::<http::Uri>() {
        if let Some(path_and_query) = uri.path_and_query() {
            let value = path_and_query.as_str();
            return if value.starts_with('/') {
                value.to_string()
            } else {
                format!("/{value}")
            };
        }
    }

    if recorded_uri.starts_with('/') {
        recorded_uri.to_string()
    } else {
        format!("/{}", recorded_uri.trim_start_matches('/'))
    }
}

fn response_body_from_bytes(
    body_bytes: &[u8],
    max_response_body: Option<usize>,
) -> (Option<String>, usize, bool) {
    let body_size = body_bytes.len();
    if let Some(limit) = max_response_body {
        if body_size > limit {
            return (
                Some(String::from_utf8_lossy(&body_bytes[..limit]).into_owned()),
                body_size,
                true,
            );
        }
    }

    (
        String::from_utf8(body_bytes.to_vec()).ok(),
        body_size,
        false,
    )
}

impl Default for ReplayClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_url_accepts_absolute_http_targets() {
        assert_eq!(
            replay_url("https://example.com", "/api/users?active=true").unwrap(),
            "https://example.com/api/users?active=true"
        );
        assert_eq!(
            replay_url("http://127.0.0.1:3000/base/", "api/users").unwrap(),
            "http://127.0.0.1:3000/base/api/users"
        );
    }

    #[test]
    fn replay_url_rejects_relative_empty_and_non_http_targets() {
        for target in [
            "",
            "/relative",
            "example.com",
            "ftp://example.com",
            "file:///tmp/x",
        ] {
            assert!(matches!(
                replay_url(target, "/api"),
                Err(ReplayClientError::InvalidUrl(_))
            ));
        }
    }

    #[test]
    fn replay_url_rejects_target_query_and_fragment() {
        for target in [
            "https://example.com?token=secret",
            "https://example.com#frag",
        ] {
            assert!(matches!(
                replay_url(target, "/api"),
                Err(ReplayClientError::InvalidUrl(_))
            ));
        }
    }

    #[test]
    fn response_body_from_bytes_applies_limit() {
        let (body, size, truncated) = response_body_from_bytes(b"abcdef", Some(3));

        assert_eq!(body.as_deref(), Some("abc"));
        assert_eq!(size, 6);
        assert!(truncated);
    }

    #[test]
    fn response_body_from_bytes_keeps_unlimited_body() {
        let (body, size, truncated) = response_body_from_bytes(b"abcdef", None);

        assert_eq!(body.as_deref(), Some("abcdef"));
        assert_eq!(size, 6);
        assert!(!truncated);
    }
}
