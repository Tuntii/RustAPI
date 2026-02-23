use rustapi_context::RequestContext;
use rustapi_memory::MemoryStore;
use rustapi_tools::ToolRegistry;
use std::collections::HashMap;
use std::sync::Arc;

/// Mutable per-execution context available to each [`Step`](crate::Step).
///
/// Provides access to:
/// - The immutable [`RequestContext`] (trace, cost, auth, events)
/// - The [`ToolRegistry`] for invoking tools
/// - The [`MemoryStore`] for reading/writing agent memory
/// - Mutable agent-local state for passing data between steps
pub struct AgentContext {
    /// The immutable request context.
    request_context: RequestContext,
    /// Tool registry.
    tools: ToolRegistry,
    /// Memory store.
    memory: Arc<dyn MemoryStore>,
    /// Mutable agent-local state (shared between steps within one execution).
    state: HashMap<String, serde_json::Value>,
    /// The current step index.
    step_index: usize,
    /// Accumulated partial yields (for streaming).
    yields: Vec<serde_json::Value>,
}

impl AgentContext {
    /// Create a new agent context.
    pub fn new(
        request_context: RequestContext,
        tools: ToolRegistry,
        memory: Arc<dyn MemoryStore>,
    ) -> Self {
        Self {
            request_context,
            tools,
            memory,
            state: HashMap::new(),
            step_index: 0,
            yields: Vec::new(),
        }
    }

    // -- RequestContext delegation --

    /// Get the immutable request context.
    pub fn request_context(&self) -> &RequestContext {
        &self.request_context
    }

    /// Shorthand: get the context id.
    pub fn context_id(&self) -> &str {
        self.request_context.id()
    }

    // -- Tools --

    /// Get the tool registry.
    pub fn tools(&self) -> &ToolRegistry {
        &self.tools
    }

    /// Execute a tool by name with the given input.
    pub async fn call_tool(
        &self,
        name: &str,
        input: serde_json::Value,
    ) -> Result<rustapi_tools::ToolOutput, rustapi_tools::ToolError> {
        self.tools.execute(name, &self.request_context, input).await
    }

    // -- Memory --

    /// Get the memory store.
    pub fn memory(&self) -> &dyn MemoryStore {
        self.memory.as_ref()
    }

    /// Store a value in memory.
    pub async fn remember(
        &self,
        key: &str,
        value: serde_json::Value,
    ) -> Result<(), rustapi_memory::MemoryError> {
        let entry = rustapi_memory::MemoryEntry::new(key, value);
        self.memory.store(entry).await
    }

    /// Recall a value from memory.
    pub async fn recall(
        &self,
        key: &str,
    ) -> Result<Option<serde_json::Value>, rustapi_memory::MemoryError> {
        Ok(self.memory.get(key).await?.map(|e| e.value))
    }

    // -- Agent-local state --

    /// Get a value from agent-local state.
    pub fn get_state(&self, key: &str) -> Option<&serde_json::Value> {
        self.state.get(key)
    }

    /// Set a value in agent-local state.
    pub fn set_state(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.state.insert(key.into(), value);
    }

    /// Remove a value from agent-local state.
    pub fn remove_state(&mut self, key: &str) -> Option<serde_json::Value> {
        self.state.remove(key)
    }

    /// Get the full agent-local state.
    pub fn state(&self) -> &HashMap<String, serde_json::Value> {
        &self.state
    }

    // -- Step tracking --

    /// Current step index.
    pub fn step_index(&self) -> usize {
        self.step_index
    }

    /// Advance to next step (called by the engine).
    pub(crate) fn advance_step(&mut self) {
        self.step_index += 1;
    }

    // -- Yields (streaming) --

    /// Record a partial yield.
    pub(crate) fn record_yield(&mut self, value: serde_json::Value) {
        self.yields.push(value);
    }

    /// Get all accumulated yields.
    pub fn yields(&self) -> &[serde_json::Value] {
        &self.yields
    }
}
