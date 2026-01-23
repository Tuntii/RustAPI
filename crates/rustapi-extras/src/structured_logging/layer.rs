//! Structured logging middleware layer

use super::config::{LogOutputFormat, StructuredLoggingConfig};
use super::formats::{
    DatadogFormatter, JsonFormatter, LogEntry, LogFormatter, LogfmtFormatter, SplunkFormatter,
};
use rustapi_core::{
    middleware::{BoxedNext, MiddlewareLayer},
    Request, Response,
};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Instant, SystemTime};

/// Structured logging middleware layer
#[derive(Clone)]
pub struct StructuredLoggingLayer {
    config: StructuredLoggingConfig,
    formatter: Arc<dyn LogFormatter>,
}

impl StructuredLoggingLayer {
    /// Create a new structured logging layer with the given configuration
    pub fn new(config: StructuredLoggingConfig) -> Self {
        let formatter: Arc<dyn LogFormatter> = match config.format {
            LogOutputFormat::Json => Arc::new(JsonFormatter::new()),
            LogOutputFormat::Datadog => Arc::new(DatadogFormatter::new()),
            LogOutputFormat::Splunk => Arc::new(SplunkFormatter::new()),
            LogOutputFormat::Logfmt => Arc::new(LogfmtFormatter::new()),
            LogOutputFormat::Pretty => Arc::new(JsonFormatter::pretty()),
        };

        Self { config, formatter }
    }

    /// Create a layer with default JSON configuration
    pub fn json() -> Self {
        Self::new(StructuredLoggingConfig::production_json())
    }

    /// Create a layer for development (pretty output)
    pub fn development() -> Self {
        Self::new(StructuredLoggingConfig::development())
    }

    /// Create a layer for Datadog APM
    pub fn datadog() -> Self {
        Self::new(StructuredLoggingConfig::datadog())
    }

    /// Create a layer for Splunk HEC
    pub fn splunk() -> Self {
        Self::new(StructuredLoggingConfig::splunk())
    }

    /// Check if path should be excluded from logging
    fn should_exclude(&self, path: &str) -> bool {
        self.config
            .exclude_paths
            .iter()
            .any(|p| path.starts_with(p))
    }

    /// Extract correlation ID from request
    fn extract_correlation_id(&self, request: &Request) -> Option<String> {
        request
            .headers()
            .get(&self.config.correlation_id_header)
            .and_then(|v| v.to_str().ok())
            .map(String::from)
            .or_else(|| {
                if self.config.generate_correlation_id {
                    Some(generate_correlation_id())
                } else {
                    None
                }
            })
    }

    /// Extract trace context from request extensions
    fn extract_trace_context(&self, request: &Request) -> (Option<String>, Option<String>) {
        // Try to get trace context from extensions (set by OtelLayer)
        if let Some(ctx) = request.extensions().get::<crate::otel::TraceContext>() {
            return (Some(ctx.trace_id.clone()), Some(ctx.span_id.clone()));
        }

        // Fallback to headers
        let trace_id = request
            .headers()
            .get("traceparent")
            .and_then(|v| v.to_str().ok())
            .and_then(|tp| {
                let parts: Vec<&str> = tp.split('-').collect();
                if parts.len() >= 2 {
                    Some(parts[1].to_string())
                } else {
                    None
                }
            });

        let span_id = request
            .headers()
            .get("traceparent")
            .and_then(|v| v.to_str().ok())
            .and_then(|tp| {
                let parts: Vec<&str> = tp.split('-').collect();
                if parts.len() >= 3 {
                    Some(parts[2].to_string())
                } else {
                    None
                }
            });

        (trace_id, span_id)
    }

    /// Extract request headers (with redaction)
    fn extract_headers(&self, headers: &http::HeaderMap) -> HashMap<String, String> {
        let mut result = HashMap::new();
        for (name, value) in headers {
            let name_str = name.as_str().to_lowercase();
            let value_str = if self.config.redact_headers.contains(&name_str) {
                "[REDACTED]".to_string()
            } else {
                value.to_str().unwrap_or("[non-utf8]").to_string()
            };
            result.insert(name_str, value_str);
        }
        result
    }

    /// Extract client IP from request
    fn extract_client_ip(&self, request: &Request) -> Option<String> {
        // Try various headers used for client IP
        request
            .headers()
            .get("x-forwarded-for")
            .or_else(|| request.headers().get("x-real-ip"))
            .or_else(|| request.headers().get("cf-connecting-ip"))
            .and_then(|v| v.to_str().ok())
            .map(|s| {
                // X-Forwarded-For can have multiple IPs, take the first
                s.split(',').next().unwrap_or(s).trim().to_string()
            })
    }

    /// Build a log entry from request and response data
    #[allow(dead_code)]
    #[allow(clippy::too_many_arguments)]
    fn build_entry(
        &self,
        message: &str,
        method: &str,
        uri: &str,
        status: Option<u16>,
        duration_ms: Option<u64>,
        correlation_id: Option<String>,
        trace_id: Option<String>,
        span_id: Option<String>,
        client_ip: Option<String>,
        user_agent: Option<String>,
        request_headers: Option<HashMap<String, String>>,
        response_headers: Option<HashMap<String, String>>,
    ) -> LogEntry {
        let mut entry = LogEntry {
            timestamp: SystemTime::now(),
            level: if status.unwrap_or(200) >= 500 {
                "error".to_string()
            } else if status.unwrap_or(200) >= 400 {
                "warn".to_string()
            } else {
                "info".to_string()
            },
            message: message.to_string(),
            method: Some(method.to_string()),
            uri: Some(uri.to_string()),
            status,
            duration_ms,
            correlation_id,
            trace_id,
            span_id,
            service_name: Some(self.config.service_name.clone()),
            service_version: self.config.service_version.clone(),
            environment: self.config.environment.clone(),
            request_headers,
            response_headers,
            request_body: None,
            response_body: None,
            client_ip,
            user_agent,
            custom_fields: HashMap::new(),
            error: None,
        };

        // Add static fields
        for (key, value) in &self.config.static_fields {
            entry.custom_fields.insert(key.clone(), value.clone());
        }

        entry
    }
}

impl MiddlewareLayer for StructuredLoggingLayer {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let config = self.config.clone();
        let formatter = Arc::clone(&self.formatter);

        let method = req.method().to_string();
        let uri = req.uri().to_string();
        let path = req.uri().path().to_string();

        // Check if this path should be excluded
        if self.should_exclude(&path) {
            return Box::pin(async move { next(req).await });
        }

        // Extract data from request
        let correlation_id = self.extract_correlation_id(&req);
        let (trace_id, span_id) = self.extract_trace_context(&req);
        let client_ip = self.extract_client_ip(&req);
        let user_agent = req
            .headers()
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let request_headers = if config.include_request_headers {
            Some(self.extract_headers(req.headers()))
        } else {
            None
        };

        let service_name = config.service_name.clone();
        let service_version = config.service_version.clone();
        let environment = config.environment.clone();
        let static_fields = config.static_fields.clone();
        let include_response_headers = config.include_response_headers;
        let log_request_start = config.log_request_start;
        let log_request_end = config.log_request_end;
        let include_timing = config.include_timing;
        let redact_headers = config.redact_headers.clone();

        Box::pin(async move {
            let start = Instant::now();

            // Log request start if configured
            if log_request_start {
                let mut entry = LogEntry::new("request started").method(&method).uri(&uri);

                if let Some(ref cid) = correlation_id {
                    entry = entry.correlation_id(cid);
                }
                if let Some(ref tid) = trace_id {
                    entry = entry.trace_id(tid);
                }
                if let Some(ref sid) = span_id {
                    entry = entry.span_id(sid);
                }
                if let Some(sn) = Some(&service_name) {
                    entry = entry.service_name(sn);
                }

                entry.request_headers = request_headers.clone();

                for (key, value) in &static_fields {
                    entry.custom_fields.insert(key.clone(), value.clone());
                }

                tracing::info!(target: "structured", "{}", formatter.format(&entry));
            }

            // Call next middleware/handler
            let response = next(req).await;

            // Log request end if configured
            if log_request_end {
                let duration_ms = if include_timing {
                    Some(start.elapsed().as_millis() as u64)
                } else {
                    None
                };

                let status = response.status().as_u16();

                let response_headers = if include_response_headers {
                    let mut headers = HashMap::new();
                    for (name, value) in response.headers() {
                        let name_str = name.as_str().to_lowercase();
                        let value_str = if redact_headers.contains(&name_str) {
                            "[REDACTED]".to_string()
                        } else {
                            value.to_str().unwrap_or("[non-utf8]").to_string()
                        };
                        headers.insert(name_str, value_str);
                    }
                    Some(headers)
                } else {
                    None
                };

                let level = if status >= 500 {
                    "error"
                } else if status >= 400 {
                    "warn"
                } else {
                    "info"
                };

                let mut entry = LogEntry {
                    timestamp: SystemTime::now(),
                    level: level.to_string(),
                    message: "request completed".to_string(),
                    method: Some(method),
                    uri: Some(uri),
                    status: Some(status),
                    duration_ms,
                    correlation_id,
                    trace_id,
                    span_id,
                    service_name: Some(service_name),
                    service_version,
                    environment,
                    request_headers,
                    response_headers,
                    request_body: None,
                    response_body: None,
                    client_ip,
                    user_agent,
                    custom_fields: HashMap::new(),
                    error: None,
                };

                for (key, value) in &static_fields {
                    entry.custom_fields.insert(key.clone(), value.clone());
                }

                let formatted = formatter.format(&entry);

                match level {
                    "error" => tracing::error!(target: "structured", "{}", formatted),
                    "warn" => tracing::warn!(target: "structured", "{}", formatted),
                    _ => tracing::info!(target: "structured", "{}", formatted),
                }
            }

            response
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}

/// Generate a correlation ID
fn generate_correlation_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let random: u64 = {
        use std::cell::Cell;
        thread_local! {
            static STATE: Cell<u64> = Cell::new(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64
            );
        }
        STATE.with(|state| {
            let mut x = state.get();
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            state.set(x);
            x
        })
    };

    format!("{:x}-{:x}", timestamp, random)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use rustapi_core::ResponseBody;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_structured_logging_layer() {
        let config = StructuredLoggingConfig::builder()
            .service_name("test-service")
            .format(LogOutputFormat::Json)
            .build();

        let layer = StructuredLoggingLayer::new(config);

        let next: BoxedNext = Arc::new(|_req: Request| {
            Box::pin(async {
                http::Response::builder()
                    .status(200)
                    .body(ResponseBody::new(Bytes::from("OK")))
                    .unwrap()
            }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        });

        let req = http::Request::builder()
            .method("GET")
            .uri("/api/test")
            .header("x-correlation-id", "test-123")
            .body(())
            .unwrap();
        let req = Request::from_http_request(req, Bytes::new());

        let response = layer.call(req, next).await;
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn test_excludes_health_check() {
        let config = StructuredLoggingConfig::builder()
            .exclude_path("/health")
            .build();

        let layer = StructuredLoggingLayer::new(config);
        assert!(layer.should_exclude("/health"));
        assert!(layer.should_exclude("/health/ready"));
        assert!(!layer.should_exclude("/api/health"));
    }

    #[test]
    fn test_generate_correlation_id() {
        let id1 = generate_correlation_id();
        let id2 = generate_correlation_id();

        assert!(!id1.is_empty());
        assert_ne!(id1, id2);
        assert!(id1.contains('-'));
    }

    #[tokio::test]
    async fn test_json_factory() {
        let layer = StructuredLoggingLayer::json();
        assert!(matches!(layer.config.format, LogOutputFormat::Json));
    }

    #[tokio::test]
    async fn test_datadog_factory() {
        let layer = StructuredLoggingLayer::datadog();
        assert!(matches!(layer.config.format, LogOutputFormat::Datadog));
    }
}
