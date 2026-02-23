use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Message — multi-turn conversation messages
// ---------------------------------------------------------------------------

/// Role of a message sender.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    /// Tool call id (for tool role messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Tool calls requested by the assistant.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            tool_call_id: None,
            tool_calls: Vec::new(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            tool_call_id: None,
            tool_calls: Vec::new(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_call_id: None,
            tool_calls: Vec::new(),
        }
    }

    pub fn tool_result(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.into(),
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: Vec::new(),
        }
    }
}

/// A tool call requested by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique id for this call.
    pub id: String,
    /// Name of the tool/function to call.
    pub name: String,
    /// Arguments as a JSON string or value.
    pub arguments: serde_json::Value,
}

// ---------------------------------------------------------------------------
// CompletionRequest
// ---------------------------------------------------------------------------

/// Model-agnostic completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// Conversation messages.
    pub messages: Vec<Message>,
    /// Model identifier (e.g. "gpt-4o", "claude-sonnet-4-20250514").
    /// If None, the router picks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Maximum output tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Temperature (0.0 – 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p nucleus sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Stop sequences.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stop: Vec<String>,
    /// Available tools for function calling.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<FunctionDef>,
    /// JSON Schema for structured output (if required).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_schema: Option<serde_json::Value>,
    /// Arbitrary metadata / provider-specific params.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
}

impl CompletionRequest {
    /// Create a minimal request with a single user message.
    pub fn simple(content: impl Into<String>) -> Self {
        Self {
            messages: vec![Message::user(content)],
            model: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
            stop: Vec::new(),
            tools: Vec::new(),
            response_schema: None,
            extra: HashMap::new(),
        }
    }

    /// Builder: set model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Builder: set max tokens.
    pub fn with_max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max);
        self
    }

    /// Builder: set temperature.
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Builder: add a system message at the start.
    pub fn with_system(mut self, content: impl Into<String>) -> Self {
        self.messages.insert(0, Message::system(content));
        self
    }

    /// Builder: add tool definitions.
    pub fn with_tools(mut self, tools: Vec<FunctionDef>) -> Self {
        self.tools = tools;
        self
    }

    /// Builder: require structured output with a JSON schema.
    pub fn with_response_schema(mut self, schema: serde_json::Value) -> Self {
        self.response_schema = Some(schema);
        self
    }
}

/// Tool/function definition for LLM function calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

// ---------------------------------------------------------------------------
// CompletionResponse
// ---------------------------------------------------------------------------

/// Model-agnostic completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// The model that was used.
    pub model: String,
    /// The generated text content.
    pub content: String,
    /// Tool calls requested by the model (if any).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    /// Token usage.
    pub usage: TokenUsage,
    /// Finish reason.
    pub finish_reason: FinishReason,
    /// Provider name.
    pub provider: String,
}

/// Token usage counters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

/// Why the generation stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// Natural end of text.
    Stop,
    /// Hit max_tokens limit.
    Length,
    /// Model wants to call a tool.
    ToolCalls,
    /// Content was filtered.
    ContentFilter,
    /// Unknown reason.
    Other(String),
}

// ---------------------------------------------------------------------------
// ModelInfo — capabilities and pricing
// ---------------------------------------------------------------------------

/// Information about a specific model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model identifier.
    pub model_id: String,
    /// Provider name.
    pub provider: String,
    /// Maximum context window (tokens).
    pub context_window: u64,
    /// Maximum output tokens.
    pub max_output_tokens: u64,
    /// Cost per million input tokens (USD).
    pub cost_per_m_input: f64,
    /// Cost per million output tokens (USD).
    pub cost_per_m_output: f64,
    /// Whether the model supports function calling.
    pub supports_tools: bool,
    /// Whether the model supports structured output.
    pub supports_structured_output: bool,
    /// Whether the model supports streaming.
    pub supports_streaming: bool,
}

impl ModelInfo {
    /// Estimate cost in micro-USD for a given token usage.
    pub fn estimate_cost_micros(&self, input_tokens: u64, output_tokens: u64) -> u64 {
        let input_cost = (input_tokens as f64 / 1_000_000.0) * self.cost_per_m_input;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * self.cost_per_m_output;
        ((input_cost + output_cost) * 1_000_000.0) as u64
    }
}

// ---------------------------------------------------------------------------
// StreamChunk — streaming response fragments
// ---------------------------------------------------------------------------

/// A single chunk in a streaming response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Text delta (may be empty for tool call chunks).
    pub content_delta: String,
    /// Tool call deltas (streaming tool call arguments).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_call_deltas: Vec<ToolCallDelta>,
    /// Whether this is the final chunk.
    pub done: bool,
    /// Accumulated usage (available on last chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
}

/// A delta update for a streaming tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallDelta {
    /// Tool call id.
    pub id: String,
    /// Function name (may be empty after first chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Arguments delta (JSON string fragment).
    pub arguments_delta: String,
}
