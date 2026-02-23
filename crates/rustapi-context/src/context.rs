use crate::{CostBudget, CostTracker, EventBus, SharedCostTracker, TraceTree};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// AuthContext — identity / claims carried by the request
// ---------------------------------------------------------------------------

/// Authentication and identity context for an AI request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    /// Subject identifier (user id, service account, API key hash).
    pub subject: String,
    /// Optional display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Roles / scopes / permissions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
    /// Raw claims map (e.g. JWT claims).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub claims: HashMap<String, serde_json::Value>,
}

impl AuthContext {
    /// Create a simple auth context with just a subject.
    pub fn new(subject: impl Into<String>) -> Self {
        Self {
            subject: subject.into(),
            name: None,
            roles: Vec::new(),
            claims: HashMap::new(),
        }
    }

    /// Builder: set display name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Builder: add a role.
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.roles.push(role.into());
        self
    }

    /// Check whether this auth context has a specific role.
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
}

// ---------------------------------------------------------------------------
// ObservabilityCtx — distributed tracing identifiers
// ---------------------------------------------------------------------------

/// Distributed tracing / observability identifiers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityCtx {
    /// OpenTelemetry-compatible trace id.
    pub trace_id: String,
    /// Current span id.
    pub span_id: String,
    /// Parent span id (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,
    /// W3C baggage key-value pairs.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub baggage: HashMap<String, String>,
}

impl ObservabilityCtx {
    /// Generate new random identifiers.
    pub fn new() -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string(),
            span_id: Uuid::new_v4().to_string(),
            parent_span_id: None,
            baggage: HashMap::new(),
        }
    }

    /// Create from an existing parent span.
    pub fn child_of(parent_trace_id: &str, parent_span_id: &str) -> Self {
        Self {
            trace_id: parent_trace_id.to_string(),
            span_id: Uuid::new_v4().to_string(),
            parent_span_id: Some(parent_span_id.to_string()),
            baggage: HashMap::new(),
        }
    }
}

impl Default for ObservabilityCtx {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// RequestContext — immutable per-request context
// ---------------------------------------------------------------------------

/// Immutable per-request context for AI execution.
///
/// Created once when an AI request arrives and threaded through every stage
/// of the execution pipeline (agent engine → tool graph → LLM → memory).
///
/// The context is **cheaply cloneable** (all fields are `Arc`-wrapped or `Clone`).
///
/// # Fields
///
/// | Field | Purpose |
/// |-------|---------|
/// | `id` | Unique request identifier (UUID v4) |
/// | `trace` | Append-only execution trace tree |
/// | `cost` | Atomic token/cost accounting |
/// | `auth` | Identity and claims |
/// | `metadata` | Extensible key-value store |
/// | `event_bus` | Broadcast event channel |
/// | `observability` | Distributed tracing IDs |
/// | `created_at` | Request arrival time |
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Unique identifier for this request.
    id: String,
    /// Hierarchical execution trace.
    trace: TraceTree,
    /// Cost accounting.
    cost: SharedCostTracker,
    /// Authentication / identity.
    auth: Option<AuthContext>,
    /// Extensible metadata.
    metadata: Arc<HashMap<String, serde_json::Value>>,
    /// Event bus for execution pipeline events.
    event_bus: EventBus,
    /// Distributed tracing identifiers.
    observability: ObservabilityCtx,
    /// When this context was created.
    created_at: DateTime<Utc>,
}

impl RequestContext {
    /// Get the unique request id.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the execution trace tree.
    pub fn trace(&self) -> &TraceTree {
        &self.trace
    }

    /// Get the cost tracker.
    pub fn cost(&self) -> &CostTracker {
        &self.cost
    }

    /// Get the shared cost tracker (Arc-wrapped).
    pub fn shared_cost(&self) -> SharedCostTracker {
        Arc::clone(&self.cost)
    }

    /// Get the auth context, if present.
    pub fn auth(&self) -> Option<&AuthContext> {
        self.auth.as_ref()
    }

    /// Get a metadata value by key.
    pub fn metadata_value(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.get(key)
    }

    /// Get the full metadata map.
    pub fn metadata(&self) -> &HashMap<String, serde_json::Value> {
        &self.metadata
    }

    /// Get the event bus.
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    /// Get observability context.
    pub fn observability(&self) -> &ObservabilityCtx {
        &self.observability
    }

    /// When this context was created.
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

// ---------------------------------------------------------------------------
// RequestContextBuilder
// ---------------------------------------------------------------------------

/// Builder for [`RequestContext`].
///
/// # Example
///
/// ```
/// use rustapi_context::{RequestContextBuilder, AuthContext, CostBudget};
///
/// let ctx = RequestContextBuilder::new()
///     .method("POST")
///     .path("/agent/research")
///     .auth(AuthContext::new("user-42"))
///     .budget(CostBudget::per_request_usd(0.10))
///     .metadata("tenant", serde_json::json!("acme"))
///     .build();
///
/// assert!(!ctx.id().is_empty());
/// assert!(ctx.auth().is_some());
/// ```
pub struct RequestContextBuilder {
    method: Option<String>,
    path: Option<String>,
    auth: Option<AuthContext>,
    metadata: HashMap<String, serde_json::Value>,
    budget: Option<CostBudget>,
    event_bus: Option<EventBus>,
    observability: Option<ObservabilityCtx>,
}

impl RequestContextBuilder {
    pub fn new() -> Self {
        Self {
            method: None,
            path: None,
            auth: None,
            metadata: HashMap::new(),
            budget: None,
            event_bus: None,
            observability: None,
        }
    }

    /// HTTP method (for trace root).
    pub fn method(mut self, method: &str) -> Self {
        self.method = Some(method.to_string());
        self
    }

    /// HTTP path (for trace root).
    pub fn path(mut self, path: &str) -> Self {
        self.path = Some(path.to_string());
        self
    }

    /// Set authentication context.
    pub fn auth(mut self, auth: AuthContext) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Set cost budget.
    pub fn budget(mut self, budget: CostBudget) -> Self {
        self.budget = Some(budget);
        self
    }

    /// Add a metadata key-value pair.
    pub fn metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Use an existing event bus (shared across requests for global subscribers).
    pub fn event_bus(mut self, bus: EventBus) -> Self {
        self.event_bus = Some(bus);
        self
    }

    /// Set observability context (e.g. propagated from incoming headers).
    pub fn observability(mut self, obs: ObservabilityCtx) -> Self {
        self.observability = Some(obs);
        self
    }

    /// Build the immutable [`RequestContext`].
    pub fn build(self) -> RequestContext {
        let method = self.method.as_deref().unwrap_or("UNKNOWN");
        let path = self.path.as_deref().unwrap_or("/");

        let cost = match self.budget {
            Some(budget) => Arc::new(CostTracker::with_budget(budget)),
            None => Arc::new(CostTracker::new()),
        };

        RequestContext {
            id: Uuid::new_v4().to_string(),
            trace: TraceTree::new_http(method, path),
            cost,
            auth: self.auth,
            metadata: Arc::new(self.metadata),
            event_bus: self.event_bus.unwrap_or_default(),
            observability: self.observability.unwrap_or_default(),
            created_at: Utc::now(),
        }
    }
}

impl Default for RequestContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CostDelta;

    #[test]
    fn test_context_builder_basic() {
        let ctx = RequestContextBuilder::new()
            .method("POST")
            .path("/agent/chat")
            .build();

        assert!(!ctx.id().is_empty());
        assert!(ctx.auth().is_none());
        assert_eq!(ctx.cost().total_tokens(), 0);
    }

    #[test]
    fn test_context_with_auth() {
        let auth = AuthContext::new("user-1")
            .with_name("Alice")
            .with_role("admin");

        let ctx = RequestContextBuilder::new().auth(auth).build();
        let a = ctx.auth().unwrap();
        assert_eq!(a.subject, "user-1");
        assert!(a.has_role("admin"));
        assert!(!a.has_role("viewer"));
    }

    #[test]
    fn test_context_with_metadata() {
        let ctx = RequestContextBuilder::new()
            .metadata("tenant", serde_json::json!("acme"))
            .metadata("env", serde_json::json!("production"))
            .build();

        assert_eq!(
            ctx.metadata_value("tenant"),
            Some(&serde_json::json!("acme"))
        );
    }

    #[test]
    fn test_context_cost_tracking() {
        let ctx = RequestContextBuilder::new()
            .budget(CostBudget::per_request_tokens(1000))
            .build();

        ctx.cost()
            .record(&CostDelta {
                input_tokens: 100,
                output_tokens: 50,
                cost_micros: 300,
                model: Some("gpt-4o".into()),
            })
            .unwrap();

        assert_eq!(ctx.cost().total_tokens(), 150);
    }

    #[test]
    fn test_context_clone_is_cheap() {
        let ctx = RequestContextBuilder::new().build();
        let ctx2 = ctx.clone();
        assert_eq!(ctx.id(), ctx2.id());
        // Both share the same trace tree (Arc)
        ctx.trace()
            .start_span(crate::TraceNodeKind::AgentStep, "test")
            .complete(None);
        assert_eq!(ctx2.trace().node_count(), 2);
    }

    #[tokio::test]
    async fn test_context_event_bus() {
        let bus = EventBus::new(16);
        let mut sub = bus.subscribe();

        let ctx = RequestContextBuilder::new()
            .event_bus(bus.clone())
            .build();

        ctx.event_bus().emit(crate::ExecutionEvent::ContextCreated {
            context_id: ctx.id().to_string(),
            timestamp: Utc::now(),
        });

        let event = sub.recv().await.unwrap();
        assert_eq!(event.context_id(), ctx.id());
    }
}
