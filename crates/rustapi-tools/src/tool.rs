use crate::ToolError;
use async_trait::async_trait;
use rustapi_context::{CostDelta, RequestContext};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// ToolOutput — result of a tool execution
// ---------------------------------------------------------------------------

/// Result of a single tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    /// The output value produced by the tool.
    pub value: serde_json::Value,
    /// Cost incurred by this tool execution (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<CostDelta>,
    /// Side effects produced (for observability / replay).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub side_effects: Vec<SideEffect>,
}

impl ToolOutput {
    /// Create a simple output with just a value.
    pub fn value(value: serde_json::Value) -> Self {
        Self {
            value,
            cost: None,
            side_effects: Vec::new(),
        }
    }

    /// Builder: attach cost delta.
    pub fn with_cost(mut self, cost: CostDelta) -> Self {
        self.cost = Some(cost);
        self
    }

    /// Builder: add a side effect.
    pub fn with_side_effect(mut self, effect: SideEffect) -> Self {
        self.side_effects.push(effect);
        self
    }
}

/// A side effect produced by a tool (for audit / replay purposes).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SideEffect {
    /// An HTTP request was made to an external service.
    HttpRequest { url: String, method: String },
    /// Data was written to a database or store.
    DataWrite { target: String, key: String },
    /// A file was created or modified.
    FileWrite { path: String },
    /// A message was sent (email, webhook, etc.).
    MessageSent { channel: String, recipient: String },
    /// Custom side effect.
    Custom { kind: String, details: serde_json::Value },
}

// ---------------------------------------------------------------------------
// FunctionDefinition — LLM function calling schema
// ---------------------------------------------------------------------------

/// OpenAI/Anthropic-compatible function definition for LLM tool calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// Function name (must be unique within a tool set).
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema for the input parameters.
    pub parameters: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Tool trait — the core abstraction
// ---------------------------------------------------------------------------

/// A callable tool that can be invoked by an AI agent.
///
/// Tools are the primary way agents interact with external systems
/// (APIs, databases, search engines, etc.).
///
/// # Example
///
/// ```ignore
/// use rustapi_tools::{Tool, ToolOutput, ToolError, FunctionDefinition};
/// use rustapi_context::RequestContext;
/// use async_trait::async_trait;
///
/// struct WebSearch;
///
/// #[async_trait]
/// impl Tool for WebSearch {
///     fn name(&self) -> &str { "web_search" }
///     fn description(&self) -> &str { "Search the web for information" }
///
///     fn parameters_schema(&self) -> serde_json::Value {
///         serde_json::json!({
///             "type": "object",
///             "properties": {
///                 "query": { "type": "string", "description": "Search query" }
///             },
///             "required": ["query"]
///         })
///     }
///
///     async fn execute(
///         &self,
///         ctx: &RequestContext,
///         input: serde_json::Value,
///     ) -> Result<ToolOutput, ToolError> {
///         let query = input["query"].as_str().unwrap_or_default();
///         // ... perform search ...
///         Ok(ToolOutput::value(serde_json::json!({"results": []})))
///     }
/// }
/// ```
#[async_trait]
pub trait Tool: Send + Sync + 'static {
    /// Unique name for this tool.
    fn name(&self) -> &str;

    /// Human-readable description (shown to LLM for function calling).
    fn description(&self) -> &str;

    /// JSON Schema describing the input parameters.
    fn parameters_schema(&self) -> serde_json::Value;

    /// Execute the tool with the given input.
    async fn execute(
        &self,
        ctx: &RequestContext,
        input: serde_json::Value,
    ) -> Result<ToolOutput, ToolError>;

    /// Convert to an LLM function definition.
    fn to_function_definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters_schema(),
        }
    }
}

// ---------------------------------------------------------------------------
// ClosureTool — create tools from closures
// ---------------------------------------------------------------------------

/// A tool created from a closure, for quick prototyping.
pub struct ClosureTool<F>
where
    F: Fn(&RequestContext, serde_json::Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ToolOutput, ToolError>> + Send + '_>>
        + Send
        + Sync
        + 'static,
{
    name: String,
    description: String,
    parameters: serde_json::Value,
    handler: F,
}

impl<F> ClosureTool<F>
where
    F: Fn(&RequestContext, serde_json::Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ToolOutput, ToolError>> + Send + '_>>
        + Send
        + Sync
        + 'static,
{
    /// Create a new closure-based tool.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
        handler: F,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
            handler,
        }
    }
}

#[async_trait]
impl<F> Tool for ClosureTool<F>
where
    F: Fn(&RequestContext, serde_json::Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ToolOutput, ToolError>> + Send + '_>>
        + Send
        + Sync
        + 'static,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.parameters.clone()
    }

    async fn execute(
        &self,
        ctx: &RequestContext,
        input: serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        (self.handler)(ctx, input).await
    }
}
