//! W3C Trace Context propagation utilities
//!
//! This module implements trace context propagation according to the
//! W3C Trace Context specification for distributed tracing.

use rustapi_core::Request;
use std::fmt;

/// W3C Trace Context header name for traceparent
pub const TRACEPARENT_HEADER: &str = "traceparent";

/// W3C Trace Context header name for tracestate
pub const TRACESTATE_HEADER: &str = "tracestate";

/// Correlation ID header name
pub const CORRELATION_ID_HEADER: &str = "x-correlation-id";

/// Request ID header name
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Trace context information
#[derive(Clone, Debug, Default)]
pub struct TraceContext {
    /// Trace ID (128-bit, hex encoded)
    pub trace_id: String,
    /// Span ID (64-bit, hex encoded)
    pub span_id: String,
    /// Parent span ID (64-bit, hex encoded) - if this is a child span
    pub parent_span_id: Option<String>,
    /// Trace flags (8 bits)
    pub trace_flags: u8,
    /// Trace state (vendor-specific data)
    pub trace_state: Option<String>,
    /// Correlation ID for request tracking
    pub correlation_id: Option<String>,
}

impl TraceContext {
    /// Create a new trace context with generated IDs
    pub fn new() -> Self {
        Self {
            trace_id: Self::generate_trace_id(),
            span_id: Self::generate_span_id(),
            parent_span_id: None,
            trace_flags: 0x01, // Sampled flag
            trace_state: None,
            correlation_id: Some(Self::generate_correlation_id()),
        }
    }

    /// Create a child span context from a parent
    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            span_id: Self::generate_span_id(),
            parent_span_id: Some(self.span_id.clone()),
            trace_flags: self.trace_flags,
            trace_state: self.trace_state.clone(),
            correlation_id: self.correlation_id.clone(),
        }
    }

    /// Generate a new trace ID (128-bit, 32 hex chars)
    pub fn generate_trace_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let random: u64 = rand_simple();
        format!("{:016x}{:016x}", now as u64, random)
    }

    /// Generate a new span ID (64-bit, 16 hex chars)
    pub fn generate_span_id() -> String {
        let random: u64 = rand_simple();
        format!("{:016x}", random)
    }

    /// Generate a correlation ID
    pub fn generate_correlation_id() -> String {
        let random: u64 = rand_simple();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        format!("{:x}-{:x}", timestamp, random)
    }

    /// Check if trace is sampled
    pub fn is_sampled(&self) -> bool {
        self.trace_flags & 0x01 == 0x01
    }

    /// Set sampled flag
    pub fn set_sampled(&mut self, sampled: bool) {
        if sampled {
            self.trace_flags |= 0x01;
        } else {
            self.trace_flags &= !0x01;
        }
    }

    /// Format as W3C traceparent header value
    pub fn to_traceparent(&self) -> String {
        format!(
            "00-{}-{}-{:02x}",
            self.trace_id, self.span_id, self.trace_flags
        )
    }

    /// Parse from W3C traceparent header value
    pub fn from_traceparent(value: &str) -> Option<Self> {
        let parts: Vec<&str> = value.split('-').collect();
        if parts.len() != 4 {
            return None;
        }

        let version = parts[0];
        if version != "00" {
            return None; // Only version 00 is supported
        }

        let trace_id = parts[1];
        let span_id = parts[2];
        let flags = parts[3];

        // Validate lengths
        if trace_id.len() != 32 || span_id.len() != 16 || flags.len() != 2 {
            return None;
        }

        // Parse flags
        let trace_flags = u8::from_str_radix(flags, 16).ok()?;

        Some(Self {
            trace_id: trace_id.to_string(),
            span_id: span_id.to_string(),
            parent_span_id: None,
            trace_flags,
            trace_state: None,
            correlation_id: None,
        })
    }
}

impl fmt::Display for TraceContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_traceparent())
    }
}

/// Extract trace context from incoming request headers
pub fn extract_trace_context(request: &Request) -> TraceContext {
    let headers = request.headers();

    // Try to extract traceparent header
    let mut context = headers
        .get(TRACEPARENT_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(TraceContext::from_traceparent)
        .unwrap_or_else(TraceContext::new);

    // Extract tracestate if present
    if let Some(state) = headers.get(TRACESTATE_HEADER).and_then(|v| v.to_str().ok()) {
        context.trace_state = Some(state.to_string());
    }

    // Extract correlation ID from various headers
    context.correlation_id = headers
        .get(CORRELATION_ID_HEADER)
        .or_else(|| headers.get(REQUEST_ID_HEADER))
        .or_else(|| headers.get("x-amzn-trace-id"))
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .or_else(|| Some(TraceContext::generate_correlation_id()));

    context
}

/// Inject trace context into outgoing request headers
pub fn inject_trace_context(headers: &mut http::HeaderMap, context: &TraceContext) {
    use http::header::HeaderValue;

    // Inject traceparent
    if let Ok(value) = HeaderValue::from_str(&context.to_traceparent()) {
        headers.insert(TRACEPARENT_HEADER, value);
    }

    // Inject tracestate if present
    if let Some(ref state) = context.trace_state {
        if let Ok(value) = HeaderValue::from_str(state) {
            headers.insert(TRACESTATE_HEADER, value);
        }
    }

    // Inject correlation ID
    if let Some(ref correlation_id) = context.correlation_id {
        if let Ok(value) = HeaderValue::from_str(correlation_id) {
            headers.insert(CORRELATION_ID_HEADER, value);
        }
    }
}

/// Propagate trace context to response headers
pub fn propagate_trace_context(response_headers: &mut http::HeaderMap, context: &TraceContext) {
    use http::header::HeaderValue;

    // Include trace ID in response for debugging
    if let Ok(value) = HeaderValue::from_str(&context.trace_id) {
        response_headers.insert("x-trace-id", value);
    }

    // Include correlation ID in response
    if let Some(ref correlation_id) = context.correlation_id {
        if let Ok(value) = HeaderValue::from_str(correlation_id) {
            response_headers.insert(CORRELATION_ID_HEADER, value);
        }
    }
}

/// Simple random number generator (using XorShift)
fn rand_simple() -> u64 {
    use std::cell::Cell;
    use std::time::{SystemTime, UNIX_EPOCH};

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_context_new() {
        let ctx = TraceContext::new();
        assert_eq!(ctx.trace_id.len(), 32);
        assert_eq!(ctx.span_id.len(), 16);
        assert!(ctx.is_sampled());
        assert!(ctx.correlation_id.is_some());
    }

    #[test]
    fn test_trace_context_child() {
        let parent = TraceContext::new();
        let child = parent.child();

        assert_eq!(child.trace_id, parent.trace_id);
        assert_ne!(child.span_id, parent.span_id);
        assert_eq!(child.parent_span_id, Some(parent.span_id));
        assert_eq!(child.correlation_id, parent.correlation_id);
    }

    #[test]
    fn test_traceparent_round_trip() {
        let ctx = TraceContext::new();
        let traceparent = ctx.to_traceparent();
        let parsed = TraceContext::from_traceparent(&traceparent).unwrap();

        assert_eq!(parsed.trace_id, ctx.trace_id);
        assert_eq!(parsed.span_id, ctx.span_id);
        assert_eq!(parsed.trace_flags, ctx.trace_flags);
    }

    #[test]
    fn test_traceparent_parsing() {
        let traceparent = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let ctx = TraceContext::from_traceparent(traceparent).unwrap();

        assert_eq!(ctx.trace_id, "0af7651916cd43dd8448eb211c80319c");
        assert_eq!(ctx.span_id, "b7ad6b7169203331");
        assert_eq!(ctx.trace_flags, 0x01);
        assert!(ctx.is_sampled());
    }

    #[test]
    fn test_invalid_traceparent() {
        // Invalid version
        assert!(TraceContext::from_traceparent("01-abc-def-00").is_none());
        // Wrong number of parts
        assert!(TraceContext::from_traceparent("00-abc-def").is_none());
        // Invalid lengths
        assert!(TraceContext::from_traceparent("00-abc-def-00").is_none());
    }

    #[test]
    fn test_sampled_flag() {
        let mut ctx = TraceContext::new();
        assert!(ctx.is_sampled());

        ctx.set_sampled(false);
        assert!(!ctx.is_sampled());

        ctx.set_sampled(true);
        assert!(ctx.is_sampled());
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// **Feature: v1-features-roadmap, Property 13: Trace context propagation**
    /// **Validates: Requirements 7.3**
    ///
    /// For any distributed trace:
    /// - Child spans SHALL inherit parent trace ID
    /// - Child spans SHALL have unique span IDs
    /// - Correlation ID SHALL propagate through entire request chain
    /// - Traceparent format SHALL conform to W3C specification
    /// - Trace context SHALL survive serialization round-trip

    /// Strategy for generating trace IDs (32 hex chars)
    fn trace_id_strategy() -> impl Strategy<Value = String> {
        prop::string::string_regex("[0-9a-f]{32}").unwrap()
    }

    /// Strategy for generating span IDs (16 hex chars)
    fn span_id_strategy() -> impl Strategy<Value = String> {
        prop::string::string_regex("[0-9a-f]{16}").unwrap()
    }

    /// Strategy for generating trace flags
    fn trace_flags_strategy() -> impl Strategy<Value = u8> {
        0u8..=255
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 13: Generated trace IDs are unique
        #[test]
        fn prop_trace_ids_unique(_seed in 0u32..100) {
            let ctx1 = TraceContext::new();
            let ctx2 = TraceContext::new();

            // Each generation should produce unique IDs
            prop_assert_ne!(ctx1.trace_id, ctx2.trace_id);
            prop_assert_ne!(ctx1.span_id, ctx2.span_id);
        }

        /// Property 13: Generated IDs have correct format
        #[test]
        fn prop_generated_ids_format(_seed in 0u32..100) {
            let ctx = TraceContext::new();

            // Trace ID: 32 hex chars
            prop_assert_eq!(ctx.trace_id.len(), 32);
            prop_assert!(ctx.trace_id.chars().all(|c| c.is_ascii_hexdigit()));

            // Span ID: 16 hex chars
            prop_assert_eq!(ctx.span_id.len(), 16);
            prop_assert!(ctx.span_id.chars().all(|c| c.is_ascii_hexdigit()));
        }

        /// Property 13: Child spans inherit parent trace ID
        #[test]
        fn prop_child_inherits_trace_id(_seed in 0u32..100) {
            let parent = TraceContext::new();
            let child = parent.child();

            // Child MUST have same trace_id as parent
            prop_assert_eq!(child.trace_id, parent.trace_id);

            // Child MUST have different span_id
            prop_assert_ne!(child.span_id, parent.span_id);

            // Child's parent_span_id MUST be parent's span_id
            prop_assert_eq!(child.parent_span_id, Some(parent.span_id.clone()));
        }

        /// Property 13: Multi-level trace propagation preserves trace ID
        #[test]
        fn prop_multilevel_trace_propagation(_seed in 0u32..100) {
            let root = TraceContext::new();
            let child1 = root.child();
            let child2 = child1.child();
            let child3 = child2.child();

            // All spans in the chain MUST have same trace_id
            prop_assert_eq!(child1.trace_id, root.trace_id);
            prop_assert_eq!(child2.trace_id, root.trace_id);
            prop_assert_eq!(child3.trace_id, root.trace_id);

            // Each span MUST have unique span_id
            let span_ids = vec![&root.span_id, &child1.span_id, &child2.span_id, &child3.span_id];
            for i in 0..span_ids.len() {
                for j in (i+1)..span_ids.len() {
                    prop_assert_ne!(span_ids[i], span_ids[j]);
                }
            }

            // Parent relationships MUST be correct
            prop_assert_eq!(child1.parent_span_id, Some(root.span_id.clone()));
            prop_assert_eq!(child2.parent_span_id, Some(child1.span_id.clone()));
            prop_assert_eq!(child3.parent_span_id, Some(child2.span_id.clone()));
        }

        /// Property 13: Correlation ID propagates through chain
        #[test]
        fn prop_correlation_id_propagation(_seed in 0u32..100) {
            let root = TraceContext::new();
            let correlation_id = root.correlation_id.clone();

            let child1 = root.child();
            let child2 = child1.child();

            // Correlation ID MUST propagate through entire chain
            prop_assert_eq!(child1.correlation_id, correlation_id);
            prop_assert_eq!(child2.correlation_id, correlation_id);
        }

        /// Property 13: Traceparent format conforms to W3C spec
        #[test]
        fn prop_traceparent_format(
            trace_id in trace_id_strategy(),
            span_id in span_id_strategy(),
            flags in trace_flags_strategy(),
        ) {
            let ctx = TraceContext {
                trace_id: trace_id.clone(),
                span_id: span_id.clone(),
                parent_span_id: None,
                trace_flags: flags,
                trace_state: None,
                correlation_id: None,
            };

            let traceparent = ctx.to_traceparent();

            // Format: version-trace_id-span_id-flags
            let parts: Vec<&str> = traceparent.split('-').collect();
            prop_assert_eq!(parts.len(), 4);

            // Version must be "00"
            prop_assert_eq!(parts[0], "00");

            // Trace ID must match (32 hex chars)
            prop_assert_eq!(parts[1], trace_id);
            prop_assert_eq!(parts[1].len(), 32);

            // Span ID must match (16 hex chars)
            prop_assert_eq!(parts[2], span_id);
            prop_assert_eq!(parts[2].len(), 16);

            // Flags must be 2 hex chars
            prop_assert_eq!(parts[3].len(), 2);
            prop_assert_eq!(parts[3], format!("{:02x}", flags));
        }

        /// Property 13: Traceparent round-trip preserves data
        #[test]
        fn prop_traceparent_roundtrip(
            trace_id in trace_id_strategy(),
            span_id in span_id_strategy(),
            flags in trace_flags_strategy(),
        ) {
            let original = TraceContext {
                trace_id: trace_id.clone(),
                span_id: span_id.clone(),
                parent_span_id: None,
                trace_flags: flags,
                trace_state: None,
                correlation_id: None,
            };

            // Serialize to traceparent
            let traceparent = original.to_traceparent();

            // Deserialize back
            let parsed = TraceContext::from_traceparent(&traceparent).unwrap();

            // All fields must match
            prop_assert_eq!(parsed.trace_id, original.trace_id);
            prop_assert_eq!(parsed.span_id, original.span_id);
            prop_assert_eq!(parsed.trace_flags, original.trace_flags);
        }

        /// Property 13: Sampled flag is correctly encoded/decoded
        #[test]
        fn prop_sampled_flag_encoding(sampled in proptest::bool::ANY) {
            let mut ctx = TraceContext::new();
            ctx.set_sampled(sampled);

            // Sampled flag should be reflected in is_sampled()
            prop_assert_eq!(ctx.is_sampled(), sampled);

            // Sampled flag should survive serialization
            let traceparent = ctx.to_traceparent();
            let parsed = TraceContext::from_traceparent(&traceparent).unwrap();
            prop_assert_eq!(parsed.is_sampled(), sampled);
        }

        /// Property 13: Invalid traceparent strings are rejected
        #[test]
        fn prop_invalid_traceparent_rejected(
            invalid_version in "0[1-9]|[1-9][0-9]",
            trace_id in "[0-9a-f]{10,50}",
            span_id in "[0-9a-f]{8,20}",
            flags in "[0-9a-f]{1,4}",
        ) {
            // Wrong version
            let invalid1 = format!("{}-{}-{}-{}", invalid_version, trace_id, span_id, flags);
            prop_assert!(TraceContext::from_traceparent(&invalid1).is_none());

            // Missing parts
            let invalid2 = format!("00-{}-{}", trace_id, span_id);
            prop_assert!(TraceContext::from_traceparent(&invalid2).is_none());
        }

        /// Property 13: Trace state propagation
        #[test]
        fn prop_trace_state_propagation(state in "[a-z0-9=,]{5,50}") {
            let mut ctx = TraceContext::new();
            ctx.trace_state = Some(state.clone());

            let child = ctx.child();

            // Trace state MUST propagate to child
            prop_assert_eq!(child.trace_state, Some(state));
        }

        /// Property 13: Correlation ID format is valid
        #[test]
        fn prop_correlation_id_format(_seed in 0u32..100) {
            let ctx = TraceContext::new();

            prop_assert!(ctx.correlation_id.is_some());
            let corr_id = ctx.correlation_id.unwrap();

            // Should be non-empty
            prop_assert!(!corr_id.is_empty());

            // Should contain hex characters and hyphen
            prop_assert!(corr_id.contains('-'));

            // Parts should be hex
            let parts: Vec<&str> = corr_id.split('-').collect();
            prop_assert_eq!(parts.len(), 2);
            prop_assert!(parts[0].chars().all(|c| c.is_ascii_hexdigit()));
            prop_assert!(parts[1].chars().all(|c| c.is_ascii_hexdigit()));
        }

        /// Property 13: Header injection and extraction preserves context
        #[test]
        fn prop_header_injection_extraction(
            trace_id in trace_id_strategy(),
            span_id in span_id_strategy(),
            flags in trace_flags_strategy(),
        ) {
            let original = TraceContext {
                trace_id: trace_id.clone(),
                span_id: span_id.clone(),
                parent_span_id: None,
                trace_flags: flags,
                trace_state: None,
                correlation_id: Some("test-corr-id".to_string()),
            };

            // Inject into headers
            let mut headers = http::HeaderMap::new();
            inject_trace_context(&mut headers, &original);

            // Headers should contain traceparent
            prop_assert!(headers.contains_key(TRACEPARENT_HEADER));

            // Extract traceparent back
            let traceparent_value = headers.get(TRACEPARENT_HEADER).unwrap().to_str().unwrap();
            let extracted = TraceContext::from_traceparent(traceparent_value).unwrap();

            // Verify trace context is preserved
            prop_assert_eq!(extracted.trace_id, original.trace_id);
            prop_assert_eq!(extracted.span_id, original.span_id);
            prop_assert_eq!(extracted.trace_flags, original.trace_flags);
        }
    }
}
