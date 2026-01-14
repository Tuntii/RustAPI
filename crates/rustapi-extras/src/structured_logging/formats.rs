//! Log formatters for different output formats

use serde_json::json;
use std::collections::HashMap;
use std::time::SystemTime;

/// Log entry data structure
#[derive(Clone, Debug)]
pub struct LogEntry {
    /// Timestamp of the log entry
    pub timestamp: SystemTime,
    /// Log level (info, warn, error, debug)
    pub level: String,
    /// Log message
    pub message: String,
    /// HTTP method
    pub method: Option<String>,
    /// Request URI/path
    pub uri: Option<String>,
    /// HTTP status code
    pub status: Option<u16>,
    /// Request duration in milliseconds
    pub duration_ms: Option<u64>,
    /// Correlation ID
    pub correlation_id: Option<String>,
    /// Trace ID (for distributed tracing)
    pub trace_id: Option<String>,
    /// Span ID
    pub span_id: Option<String>,
    /// Service name
    pub service_name: Option<String>,
    /// Service version
    pub service_version: Option<String>,
    /// Environment name
    pub environment: Option<String>,
    /// Request headers (key-value pairs)
    pub request_headers: Option<HashMap<String, String>>,
    /// Response headers (key-value pairs)
    pub response_headers: Option<HashMap<String, String>>,
    /// Request body (truncated)
    pub request_body: Option<String>,
    /// Response body (truncated)
    pub response_body: Option<String>,
    /// Client IP address
    pub client_ip: Option<String>,
    /// User agent string
    pub user_agent: Option<String>,
    /// Additional custom fields
    pub custom_fields: HashMap<String, String>,
    /// Error message if any
    pub error: Option<String>,
}

impl Default for LogEntry {
    fn default() -> Self {
        Self {
            timestamp: SystemTime::now(),
            level: "info".to_string(),
            message: String::new(),
            method: None,
            uri: None,
            status: None,
            duration_ms: None,
            correlation_id: None,
            trace_id: None,
            span_id: None,
            service_name: None,
            service_version: None,
            environment: None,
            request_headers: None,
            response_headers: None,
            request_body: None,
            response_body: None,
            client_ip: None,
            user_agent: None,
            custom_fields: HashMap::new(),
            error: None,
        }
    }
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            ..Default::default()
        }
    }

    /// Set the log level
    pub fn level(mut self, level: impl Into<String>) -> Self {
        self.level = level.into();
        self
    }

    /// Set HTTP method
    pub fn method(mut self, method: impl Into<String>) -> Self {
        self.method = Some(method.into());
        self
    }

    /// Set URI
    pub fn uri(mut self, uri: impl Into<String>) -> Self {
        self.uri = Some(uri.into());
        self
    }

    /// Set status code
    pub fn status(mut self, status: u16) -> Self {
        self.status = Some(status);
        self
    }

    /// Set duration in milliseconds
    pub fn duration_ms(mut self, duration: u64) -> Self {
        self.duration_ms = Some(duration);
        self
    }

    /// Set correlation ID
    pub fn correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Set trace ID
    pub fn trace_id(mut self, id: impl Into<String>) -> Self {
        self.trace_id = Some(id.into());
        self
    }

    /// Set span ID
    pub fn span_id(mut self, id: impl Into<String>) -> Self {
        self.span_id = Some(id.into());
        self
    }

    /// Set service name
    pub fn service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = Some(name.into());
        self
    }

    /// Add a custom field
    pub fn field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_fields.insert(key.into(), value.into());
        self
    }

    /// Set error message
    pub fn error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self.level = "error".to_string();
        self
    }
}

/// Trait for log formatters
pub trait LogFormatter: Send + Sync {
    /// Format a log entry to a string
    fn format(&self, entry: &LogEntry) -> String;
}

/// JSON log formatter
#[derive(Clone, Debug, Default)]
pub struct JsonFormatter {
    /// Whether to pretty print JSON
    pub pretty: bool,
}

impl JsonFormatter {
    /// Create a new JSON formatter
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a pretty-printing JSON formatter
    pub fn pretty() -> Self {
        Self { pretty: true }
    }
}

impl LogFormatter for JsonFormatter {
    fn format(&self, entry: &LogEntry) -> String {
        let timestamp = entry
            .timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        let mut obj = json!({
            "timestamp": timestamp,
            "level": entry.level,
            "message": entry.message,
        });

        // Add optional fields
        if let Some(ref method) = entry.method {
            obj["http.method"] = json!(method);
        }
        if let Some(ref uri) = entry.uri {
            obj["http.url"] = json!(uri);
        }
        if let Some(status) = entry.status {
            obj["http.status_code"] = json!(status);
        }
        if let Some(duration) = entry.duration_ms {
            obj["duration_ms"] = json!(duration);
        }
        if let Some(ref correlation_id) = entry.correlation_id {
            obj["correlation_id"] = json!(correlation_id);
        }
        if let Some(ref trace_id) = entry.trace_id {
            obj["trace.id"] = json!(trace_id);
        }
        if let Some(ref span_id) = entry.span_id {
            obj["span.id"] = json!(span_id);
        }
        if let Some(ref service_name) = entry.service_name {
            obj["service.name"] = json!(service_name);
        }
        if let Some(ref service_version) = entry.service_version {
            obj["service.version"] = json!(service_version);
        }
        if let Some(ref env) = entry.environment {
            obj["environment"] = json!(env);
        }
        if let Some(ref client_ip) = entry.client_ip {
            obj["client.ip"] = json!(client_ip);
        }
        if let Some(ref user_agent) = entry.user_agent {
            obj["user_agent"] = json!(user_agent);
        }
        if let Some(ref error) = entry.error {
            obj["error.message"] = json!(error);
        }
        if let Some(ref headers) = entry.request_headers {
            obj["http.request.headers"] = json!(headers);
        }
        if let Some(ref headers) = entry.response_headers {
            obj["http.response.headers"] = json!(headers);
        }

        // Add custom fields
        for (key, value) in &entry.custom_fields {
            obj[key] = json!(value);
        }

        if self.pretty {
            serde_json::to_string_pretty(&obj).unwrap_or_default()
        } else {
            serde_json::to_string(&obj).unwrap_or_default()
        }
    }
}

/// Datadog APM log formatter
#[derive(Clone, Debug, Default)]
pub struct DatadogFormatter;

impl DatadogFormatter {
    /// Create a new Datadog formatter
    pub fn new() -> Self {
        Self
    }
}

impl LogFormatter for DatadogFormatter {
    fn format(&self, entry: &LogEntry) -> String {
        let timestamp = entry
            .timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        let mut obj = json!({
            "timestamp": timestamp,
            "status": entry.level,
            "message": entry.message,
        });

        // Datadog specific fields
        if let Some(ref trace_id) = entry.trace_id {
            obj["dd.trace_id"] = json!(trace_id);
        }
        if let Some(ref span_id) = entry.span_id {
            obj["dd.span_id"] = json!(span_id);
        }
        if let Some(ref service_name) = entry.service_name {
            obj["service"] = json!(service_name);
        }
        if let Some(ref env) = entry.environment {
            obj["env"] = json!(env);
        }
        if let Some(ref service_version) = entry.service_version {
            obj["version"] = json!(service_version);
        }

        // HTTP fields in Datadog format
        if let Some(ref method) = entry.method {
            obj["http.method"] = json!(method);
        }
        if let Some(ref uri) = entry.uri {
            obj["http.url"] = json!(uri);
        }
        if let Some(status) = entry.status {
            obj["http.status_code"] = json!(status);
        }
        if let Some(duration) = entry.duration_ms {
            obj["duration"] = json!(duration * 1_000_000); // Datadog uses nanoseconds
        }
        if let Some(ref client_ip) = entry.client_ip {
            obj["network.client.ip"] = json!(client_ip);
        }
        if let Some(ref user_agent) = entry.user_agent {
            obj["http.useragent"] = json!(user_agent);
        }

        // Error handling
        if let Some(ref error) = entry.error {
            obj["error.message"] = json!(error);
            obj["error.stack"] = json!(error);
        }

        // Custom fields
        for (key, value) in &entry.custom_fields {
            obj[key] = json!(value);
        }

        serde_json::to_string(&obj).unwrap_or_default()
    }
}

/// Splunk HEC log formatter
#[derive(Clone, Debug, Default)]
pub struct SplunkFormatter {
    /// Splunk source
    pub source: Option<String>,
    /// Splunk sourcetype
    pub sourcetype: Option<String>,
    /// Splunk index
    pub index: Option<String>,
}

impl SplunkFormatter {
    /// Create a new Splunk formatter
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the source
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set the sourcetype
    pub fn sourcetype(mut self, sourcetype: impl Into<String>) -> Self {
        self.sourcetype = Some(sourcetype.into());
        self
    }

    /// Set the index
    pub fn index(mut self, index: impl Into<String>) -> Self {
        self.index = Some(index.into());
        self
    }
}

impl LogFormatter for SplunkFormatter {
    fn format(&self, entry: &LogEntry) -> String {
        let timestamp = entry
            .timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();

        let mut event = json!({
            "level": entry.level,
            "message": entry.message,
        });

        // Add all fields to event
        if let Some(ref method) = entry.method {
            event["http_method"] = json!(method);
        }
        if let Some(ref uri) = entry.uri {
            event["http_url"] = json!(uri);
        }
        if let Some(status) = entry.status {
            event["http_status"] = json!(status);
        }
        if let Some(duration) = entry.duration_ms {
            event["duration_ms"] = json!(duration);
        }
        if let Some(ref correlation_id) = entry.correlation_id {
            event["correlation_id"] = json!(correlation_id);
        }
        if let Some(ref trace_id) = entry.trace_id {
            event["trace_id"] = json!(trace_id);
        }
        if let Some(ref service_name) = entry.service_name {
            event["service"] = json!(service_name);
        }
        if let Some(ref env) = entry.environment {
            event["environment"] = json!(env);
        }
        if let Some(ref error) = entry.error {
            event["error"] = json!(error);
        }

        for (key, value) in &entry.custom_fields {
            event[key] = json!(value);
        }

        // Build Splunk HEC format
        let mut obj = json!({
            "time": timestamp,
            "event": event,
        });

        if let Some(ref source) = self.source {
            obj["source"] = json!(source);
        }
        if let Some(ref sourcetype) = self.sourcetype {
            obj["sourcetype"] = json!(sourcetype);
        }
        if let Some(ref index) = self.index {
            obj["index"] = json!(index);
        }
        if let Some(ref service_name) = entry.service_name {
            obj["host"] = json!(service_name);
        }

        serde_json::to_string(&obj).unwrap_or_default()
    }
}

/// Logfmt log formatter (key=value pairs)
#[derive(Clone, Debug, Default)]
pub struct LogfmtFormatter;

impl LogfmtFormatter {
    /// Create a new Logfmt formatter
    pub fn new() -> Self {
        Self
    }
}

impl LogFormatter for LogfmtFormatter {
    fn format(&self, entry: &LogEntry) -> String {
        let mut parts = Vec::new();

        let timestamp = entry
            .timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        parts.push(format!("ts={}", timestamp));
        parts.push(format!("level={}", entry.level));
        parts.push(format!("msg=\"{}\"", escape_logfmt(&entry.message)));

        if let Some(ref method) = entry.method {
            parts.push(format!("method={}", method));
        }
        if let Some(ref uri) = entry.uri {
            parts.push(format!("uri=\"{}\"", escape_logfmt(uri)));
        }
        if let Some(status) = entry.status {
            parts.push(format!("status={}", status));
        }
        if let Some(duration) = entry.duration_ms {
            parts.push(format!("duration_ms={}", duration));
        }
        if let Some(ref correlation_id) = entry.correlation_id {
            parts.push(format!("correlation_id={}", correlation_id));
        }
        if let Some(ref trace_id) = entry.trace_id {
            parts.push(format!("trace_id={}", trace_id));
        }
        if let Some(ref span_id) = entry.span_id {
            parts.push(format!("span_id={}", span_id));
        }
        if let Some(ref service_name) = entry.service_name {
            parts.push(format!("service={}", service_name));
        }
        if let Some(ref env) = entry.environment {
            parts.push(format!("env={}", env));
        }
        if let Some(ref error) = entry.error {
            parts.push(format!("error=\"{}\"", escape_logfmt(error)));
        }

        for (key, value) in &entry.custom_fields {
            parts.push(format!("{}=\"{}\"", key, escape_logfmt(value)));
        }

        parts.join(" ")
    }
}

/// Escape special characters for logfmt
fn escape_logfmt(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn test_json_formatter() {
        let entry = LogEntry::new("test message")
            .method("GET")
            .uri("/api/test")
            .status(200)
            .duration_ms(42)
            .correlation_id("abc-123");

        let formatter = JsonFormatter::new();
        let output = formatter.format(&entry);

        let parsed: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["message"], "test message");
        assert_eq!(parsed["http.method"], "GET");
        assert_eq!(parsed["http.status_code"], 200);
    }

    #[test]
    fn test_datadog_formatter() {
        let entry = LogEntry::new("test message")
            .method("POST")
            .status(201)
            .trace_id("trace-123")
            .span_id("span-456")
            .service_name("my-service");

        let formatter = DatadogFormatter::new();
        let output = formatter.format(&entry);

        let parsed: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["dd.trace_id"], "trace-123");
        assert_eq!(parsed["dd.span_id"], "span-456");
        assert_eq!(parsed["service"], "my-service");
    }

    #[test]
    fn test_splunk_formatter() {
        let entry = LogEntry::new("test message")
            .method("GET")
            .status(200)
            .service_name("my-service");

        let formatter = SplunkFormatter::new()
            .source("rustapi")
            .sourcetype("json")
            .index("main");
        let output = formatter.format(&entry);

        let parsed: Value = serde_json::from_str(&output).unwrap();
        assert!(parsed["time"].is_number());
        assert_eq!(parsed["source"], "rustapi");
        assert_eq!(parsed["sourcetype"], "json");
    }

    #[test]
    fn test_logfmt_formatter() {
        let entry = LogEntry::new("test message")
            .method("GET")
            .uri("/api/test")
            .status(200)
            .correlation_id("abc-123");

        let formatter = LogfmtFormatter::new();
        let output = formatter.format(&entry);

        assert!(output.contains("level=info"));
        assert!(output.contains("method=GET"));
        assert!(output.contains("status=200"));
        assert!(output.contains("correlation_id=abc-123"));
    }

    #[test]
    fn test_log_entry_builder() {
        let entry = LogEntry::new("test")
            .level("warn")
            .method("DELETE")
            .uri("/api/item/1")
            .status(204)
            .field("custom", "value");

        assert_eq!(entry.level, "warn");
        assert_eq!(entry.method, Some("DELETE".to_string()));
        assert_eq!(
            entry.custom_fields.get("custom"),
            Some(&"value".to_string())
        );
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use serde_json::Value;

    /// **Feature: v1-features-roadmap, Property 14: Structured log format**
    /// **Validates: Requirements 7.4**
    ///
    /// For any log output:
    /// - Format SHALL be valid JSON for JSON/Datadog/Splunk formatters
    /// - All required fields SHALL be present (timestamp, level, message)
    /// - Round-trip SHALL preserve all data
    /// - Special characters SHALL be properly escaped
    /// - Logfmt format SHALL follow key=value specification

    /// Strategy for generating log levels
    fn log_level_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("debug".to_string()),
            Just("info".to_string()),
            Just("warn".to_string()),
            Just("error".to_string()),
        ]
    }

    /// Strategy for generating HTTP methods
    fn http_method_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("GET".to_string()),
            Just("POST".to_string()),
            Just("PUT".to_string()),
            Just("DELETE".to_string()),
            Just("PATCH".to_string()),
            Just("HEAD".to_string()),
            Just("OPTIONS".to_string()),
        ]
    }

    /// Strategy for generating HTTP status codes
    fn status_code_strategy() -> impl Strategy<Value = u16> {
        prop_oneof![
            200u16..=299, // 2xx success
            300u16..=399, // 3xx redirect
            400u16..=499, // 4xx client error
            500u16..=599, // 5xx server error
        ]
    }

    /// Strategy for generating URIs
    fn uri_strategy() -> impl Strategy<Value = String> {
        prop::string::string_regex("/api/[a-z]{3,10}(/[0-9]{1,5})?").unwrap()
    }

    /// Strategy for generating messages with special characters
    fn message_strategy() -> impl Strategy<Value = String> {
        prop::string::string_regex(r#"[a-zA-Z0-9 \n\r\t"'\\]{5,50}"#).unwrap()
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 14: JSON formatter produces valid JSON
        #[test]
        fn prop_json_format_valid(
            message in message_strategy(),
            level in log_level_strategy(),
            method in prop::option::of(http_method_strategy()),
            uri in prop::option::of(uri_strategy()),
            status in prop::option::of(status_code_strategy()),
        ) {
            let mut entry = LogEntry::new(message.clone()).level(level.clone());
            if let Some(m) = method {
                entry = entry.method(m);
            }
            if let Some(u) = uri {
                entry = entry.uri(u);
            }
            if let Some(s) = status {
                entry = entry.status(s);
            }

            let formatter = JsonFormatter::new();
            let output = formatter.format(&entry);

            // MUST be valid JSON
            let parsed: Value = serde_json::from_str(&output)
                .expect("JSON formatter must produce valid JSON");

            // Required fields MUST be present
            prop_assert!(parsed.get("timestamp").is_some());
            prop_assert!(parsed.get("level").is_some());
            prop_assert!(parsed.get("message").is_some());

            // Values MUST match
            prop_assert_eq!(parsed["level"].as_str().unwrap(), level);
            prop_assert_eq!(parsed["message"].as_str().unwrap(), message);
        }

        /// Property 14: JSON formatter preserves all fields
        #[test]
        fn prop_json_format_preserves_fields(
            message in "[a-zA-Z0-9 ]{5,30}",
            method in http_method_strategy(),
            uri in uri_strategy(),
            status in status_code_strategy(),
            duration in 1u64..10000,
            correlation_id in "[a-z0-9-]{10,20}",
            trace_id in "[a-f0-9]{32}",
        ) {
            let entry = LogEntry::new(message.clone())
                .method(method.clone())
                .uri(uri.clone())
                .status(status)
                .duration_ms(duration)
                .correlation_id(correlation_id.clone())
                .trace_id(trace_id.clone());

            let formatter = JsonFormatter::new();
            let output = formatter.format(&entry);
            let parsed: Value = serde_json::from_str(&output).unwrap();

            // All fields MUST be preserved
            prop_assert_eq!(parsed["message"], message);
            prop_assert_eq!(parsed["http.method"], method);
            prop_assert_eq!(parsed["http.url"], uri);
            prop_assert_eq!(parsed["http.status_code"], status);
            prop_assert_eq!(parsed["duration_ms"], duration);
            prop_assert_eq!(parsed["correlation_id"], correlation_id);
            prop_assert_eq!(parsed["trace.id"], trace_id);
        }

        /// Property 14: Datadog formatter produces valid JSON
        #[test]
        fn prop_datadog_format_valid(
            message in message_strategy(),
            trace_id in "[a-f0-9]{32}",
            span_id in "[a-f0-9]{16}",
            service in "[a-z-]{5,15}",
        ) {
            let entry = LogEntry::new(message.clone())
                .trace_id(trace_id.clone())
                .span_id(span_id.clone())
                .service_name(service.clone());

            let formatter = DatadogFormatter::new();
            let output = formatter.format(&entry);

            // MUST be valid JSON
            let parsed: Value = serde_json::from_str(&output)
                .expect("Datadog formatter must produce valid JSON");

            // Datadog-specific fields MUST be present
            prop_assert_eq!(parsed["dd.trace_id"], trace_id);
            prop_assert_eq!(parsed["dd.span_id"], span_id);
            prop_assert_eq!(parsed["service"], service);
            prop_assert_eq!(parsed["status"], "info");
        }

        /// Property 14: Datadog converts duration to nanoseconds
        #[test]
        fn prop_datadog_duration_nanoseconds(duration_ms in 1u64..10000) {
            let entry = LogEntry::new("test").duration_ms(duration_ms);
            let formatter = DatadogFormatter::new();
            let output = formatter.format(&entry);
            let parsed: Value = serde_json::from_str(&output).unwrap();

            // Datadog duration MUST be in nanoseconds (ms * 1_000_000)
            let expected_ns = duration_ms * 1_000_000;
            prop_assert_eq!(parsed["duration"], expected_ns);
        }

        /// Property 14: Splunk formatter produces valid JSON
        #[test]
        fn prop_splunk_format_valid(
            message in message_strategy(),
            source in "[a-z]{5,10}",
            sourcetype in "[a-z]{5,10}",
            index in "[a-z]{4,8}",
        ) {
            let entry = LogEntry::new(message.clone());
            let formatter = SplunkFormatter::new()
                .source(source.clone())
                .sourcetype(sourcetype.clone())
                .index(index.clone());
            let output = formatter.format(&entry);

            // MUST be valid JSON
            let parsed: Value = serde_json::from_str(&output)
                .expect("Splunk formatter must produce valid JSON");

            // Splunk HEC structure MUST be correct
            prop_assert!(parsed.get("time").is_some());
            prop_assert!(parsed.get("event").is_some());
            prop_assert_eq!(parsed["source"], source);
            prop_assert_eq!(parsed["sourcetype"], sourcetype);
            prop_assert_eq!(parsed["index"], index);

            // Event MUST contain message
            let event = &parsed["event"];
            prop_assert_eq!(event["message"], message);
        }

        /// Property 14: Splunk timestamp is Unix epoch
        #[test]
        fn prop_splunk_timestamp_format(_seed in 0u32..100) {
            let entry = LogEntry::new("test");
            let formatter = SplunkFormatter::new();
            let output = formatter.format(&entry);
            let parsed: Value = serde_json::from_str(&output).unwrap();

            // Time MUST be a number (Unix timestamp)
            prop_assert!(parsed["time"].is_f64() || parsed["time"].is_u64());

            let time = parsed["time"].as_f64().unwrap();
            // Reasonable timestamp check (after 2020-01-01, before 2100-01-01)
            prop_assert!(time > 1577836800.0 && time < 4102444800.0);
        }

        /// Property 14: Logfmt format follows specification
        #[test]
        fn prop_logfmt_format_valid(
            message in "[a-zA-Z0-9 ]{5,30}",
            method in http_method_strategy(),
            status in status_code_strategy(),
        ) {
            let entry = LogEntry::new(message.clone())
                .method(method.clone())
                .status(status);

            let formatter = LogfmtFormatter::new();
            let output = formatter.format(&entry);

            // MUST contain required fields
            prop_assert!(output.contains("ts="));
            prop_assert!(output.contains("level="));
            prop_assert!(output.contains("msg="));

            // MUST contain optional fields
            prop_assert!(output.contains(&format!("method={}", method)));
            prop_assert!(output.contains(&format!("status={}", status)));

            // MUST use key=value format
            prop_assert!(output.split_whitespace().all(|part| part.contains('=')));
        }

        /// Property 14: Logfmt escapes special characters
        #[test]
        fn prop_logfmt_escapes_special_chars(
            message in prop::string::string_regex(r#"[a-zA-Z0-9 \n\r"\\]{5,30}"#).unwrap()
        ) {
            let entry = LogEntry::new(message.clone());
            let formatter = LogfmtFormatter::new();
            let output = formatter.format(&entry);

            // Quotes MUST be escaped
            if message.contains('"') {
                prop_assert!(output.contains(r#"\""#));
            }

            // Newlines MUST be escaped
            if message.contains('\n') {
                prop_assert!(output.contains(r"\n"));
            }

            // Carriage returns MUST be escaped
            if message.contains('\r') {
                prop_assert!(output.contains(r"\r"));
            }

            // Backslashes MUST be escaped
            if message.contains('\\') {
                prop_assert!(output.contains(r"\\"));
            }
        }

        /// Property 14: Custom fields are preserved in all formats
        #[test]
        fn prop_custom_fields_preserved(
            message in "[a-zA-Z0-9 ]{5,20}",
            key in "[a-z_]{5,10}",
            value in "[a-zA-Z0-9]{5,15}",
        ) {
            let entry = LogEntry::new(message).field(key.clone(), value.clone());

            // JSON formatter
            let json_formatter = JsonFormatter::new();
            let json_output = json_formatter.format(&entry);
            let json_parsed: Value = serde_json::from_str(&json_output).unwrap();
            prop_assert_eq!(json_parsed[&key], value);

            // Datadog formatter
            let dd_formatter = DatadogFormatter::new();
            let dd_output = dd_formatter.format(&entry);
            let dd_parsed: Value = serde_json::from_str(&dd_output).unwrap();
            prop_assert_eq!(dd_parsed[&key], value);

            // Splunk formatter
            let splunk_formatter = SplunkFormatter::new();
            let splunk_output = splunk_formatter.format(&entry);
            let splunk_parsed: Value = serde_json::from_str(&splunk_output).unwrap();
            prop_assert_eq!(splunk_parsed["event"][&key], value);

            // Logfmt formatter
            let logfmt_formatter = LogfmtFormatter::new();
            let logfmt_output = logfmt_formatter.format(&entry);
            prop_assert!(logfmt_output.contains(&format!("{}=\"{}\"", key, value)));
        }

        /// Property 14: Error messages are properly formatted
        #[test]
        fn prop_error_formatting(
            error_msg in message_strategy(),
        ) {
            let entry = LogEntry::new("error occurred").error(error_msg.clone());

            // Level MUST be set to error
            prop_assert_eq!(entry.level, "error");

            // JSON formatter
            let json_formatter = JsonFormatter::new();
            let json_output = json_formatter.format(&entry);
            let json_parsed: Value = serde_json::from_str(&json_output).unwrap();
            prop_assert_eq!(json_parsed["level"], "error");
            prop_assert_eq!(json_parsed["error.message"], error_msg);

            // Datadog formatter
            let dd_formatter = DatadogFormatter::new();
            let dd_output = dd_formatter.format(&entry);
            let dd_parsed: Value = serde_json::from_str(&dd_output).unwrap();
            prop_assert_eq!(dd_parsed["status"], "error");
            prop_assert_eq!(dd_parsed["error.message"], error_msg);
        }

        /// Property 14: JSON pretty printing is valid
        #[test]
        fn prop_json_pretty_valid(message in "[a-zA-Z0-9 ]{5,20}") {
            let entry = LogEntry::new(message.clone());
            let formatter = JsonFormatter::pretty();
            let output = formatter.format(&entry);

            // MUST be valid JSON despite formatting
            let parsed: Value = serde_json::from_str(&output).unwrap();
            prop_assert_eq!(parsed["message"], message);

            // MUST contain newlines (pretty formatted)
            prop_assert!(output.contains('\n'));
        }

        /// Property 14: Timestamps are monotonic and reasonable
        #[test]
        fn prop_timestamps_reasonable(_seed in 0u32..100) {
            let entry1 = LogEntry::new("test1");
            std::thread::sleep(std::time::Duration::from_millis(1));
            let entry2 = LogEntry::new("test2");

            let formatter = JsonFormatter::new();

            let output1 = formatter.format(&entry1);
            let output2 = formatter.format(&entry2);

            let parsed1: Value = serde_json::from_str(&output1).unwrap();
            let parsed2: Value = serde_json::from_str(&output2).unwrap();

            let ts1 = parsed1["timestamp"].as_u64().unwrap();
            let ts2 = parsed2["timestamp"].as_u64().unwrap();

            // Later entry MUST have later or equal timestamp
            prop_assert!(ts2 >= ts1);

            // Timestamps MUST be reasonable (after 2020, before 2100)
            prop_assert!(ts1 > 1577836800000); // 2020-01-01 in milliseconds
            prop_assert!(ts1 < 4102444800000); // 2100-01-01 in milliseconds
        }

        /// Property 14: All formatters handle empty optional fields
        #[test]
        fn prop_empty_fields_handled(message in "[a-zA-Z0-9 ]{5,20}") {
            let entry = LogEntry::new(message.clone());

            // JSON
            let json_output = JsonFormatter::new().format(&entry);
            prop_assert!(serde_json::from_str::<Value>(&json_output).is_ok());

            // Datadog
            let dd_output = DatadogFormatter::new().format(&entry);
            prop_assert!(serde_json::from_str::<Value>(&dd_output).is_ok());

            // Splunk
            let splunk_output = SplunkFormatter::new().format(&entry);
            prop_assert!(serde_json::from_str::<Value>(&splunk_output).is_ok());

            // Logfmt
            let logfmt_output = LogfmtFormatter::new().format(&entry);
            prop_assert!(logfmt_output.contains("msg="));
        }
    }
}
