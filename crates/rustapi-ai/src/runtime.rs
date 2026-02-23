use rustapi_agent::{AgentEngine, EngineConfig};
use rustapi_llm::{LlmRouter, MockProvider};
use rustapi_memory::backend::InMemoryStore;
use rustapi_memory::MemoryStore;
use rustapi_tools::{Tool, ToolRegistry};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// AiRuntime — the unified AI-native runtime
// ---------------------------------------------------------------------------

/// Unified AI-native backend runtime.
///
/// `AiRuntime` wires together memory, tools, LLM routing, and the agent
/// engine into a single object that can be attached to a RustAPI application
/// via `State<AiRuntime>`.
///
/// # Example
///
/// ```rust,no_run
/// use rustapi_ai::prelude::*;
///
/// let runtime = AiRuntime::builder()
///     .memory(InMemoryStore::new())
///     .llm(LlmRouter::builder()
///         .provider(MockProvider::new("dev"))
///         .build())
///     .build();
///
/// // Attach to RustAPI via state
/// // RustApi::new().state(runtime).route(...)
/// ```
#[derive(Clone)]
pub struct AiRuntime {
    /// Pluggable memory backend.
    memory: Arc<dyn MemoryStore>,
    /// Tool registry (shared across requests).
    tools: Arc<ToolRegistry>,
    /// LLM router with fallback / cost routing.
    llm: Arc<LlmRouter>,
    /// Default agent configuration.
    engine_config: EngineConfig,
}

impl AiRuntime {
    /// Start building an `AiRuntime`.
    pub fn builder() -> AiRuntimeBuilder {
        AiRuntimeBuilder::default()
    }

    // -- accessors --

    /// Get a reference to the memory store.
    pub fn memory(&self) -> &dyn MemoryStore {
        self.memory.as_ref()
    }

    /// Get a clone of the memory store `Arc`.
    pub fn memory_arc(&self) -> Arc<dyn MemoryStore> {
        Arc::clone(&self.memory)
    }

    /// Get a reference to the tool registry.
    pub fn tools(&self) -> &ToolRegistry {
        &self.tools
    }

    /// Get a clone of the tool registry `Arc`.
    pub fn tools_arc(&self) -> Arc<ToolRegistry> {
        Arc::clone(&self.tools)
    }

    /// Get a reference to the LLM router.
    pub fn llm(&self) -> &LlmRouter {
        &self.llm
    }

    /// Get a clone of the LLM router `Arc`.
    pub fn llm_arc(&self) -> Arc<LlmRouter> {
        Arc::clone(&self.llm)
    }

    /// Get the default engine configuration.
    pub fn engine_config(&self) -> &EngineConfig {
        &self.engine_config
    }

    /// Create a new [`AgentEngine`] using this runtime's config.
    pub fn create_engine(&self) -> AgentEngine {
        AgentEngine::new(self.engine_config.clone())
    }
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Builder for [`AiRuntime`].
pub struct AiRuntimeBuilder {
    memory: Option<Arc<dyn MemoryStore>>,
    tools: ToolRegistry,
    llm: Option<LlmRouter>,
    engine_config: EngineConfig,
}

impl Default for AiRuntimeBuilder {
    fn default() -> Self {
        Self {
            memory: None,
            tools: ToolRegistry::new(),
            llm: None,
            engine_config: EngineConfig::default(),
        }
    }
}

impl AiRuntimeBuilder {
    /// Set the memory backend.
    pub fn memory(mut self, store: impl MemoryStore + 'static) -> Self {
        self.memory = Some(Arc::new(store));
        self
    }

    /// Set the memory backend from an `Arc`.
    pub fn memory_arc(mut self, store: Arc<dyn MemoryStore>) -> Self {
        self.memory = Some(store);
        self
    }

    /// Register a tool.
    pub fn tool(mut self, tool: impl Tool + 'static) -> Self {
        self.tools = self.tools.register(tool);
        self
    }

    /// Register a tool behind an `Arc`.
    pub fn tool_arc(mut self, tool: Arc<dyn Tool>) -> Self {
        self.tools = self.tools.register_arc(tool);
        self
    }

    /// Set the LLM router.
    pub fn llm(mut self, router: LlmRouter) -> Self {
        self.llm = Some(router);
        self
    }

    /// Customize the default engine configuration.
    pub fn engine_config(mut self, config: EngineConfig) -> Self {
        self.engine_config = config;
        self
    }

    /// Build the runtime.
    ///
    /// # Defaults
    /// - Memory: [`InMemoryStore`] (ephemeral)
    /// - LLM: [`MockProvider`] fallback (dev mode)
    pub fn build(self) -> AiRuntime {
        let memory = self.memory.unwrap_or_else(|| {
            tracing::info!("AiRuntime: using InMemoryStore (no memory backend configured)");
            Arc::new(InMemoryStore::new())
        });

        let llm = self.llm.unwrap_or_else(|| {
            tracing::info!("AiRuntime: using MockProvider (no LLM router configured)");
            LlmRouter::builder()
                .provider(MockProvider::new("ai-runtime-default"))
                .build()
        });

        AiRuntime {
            memory,
            tools: Arc::new(self.tools),
            llm: Arc::new(llm),
            engine_config: self.engine_config,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let runtime = AiRuntime::builder().build();
        // Should have default in-memory store and mock provider
        assert!(!runtime.llm().available_models().is_empty());
    }

    #[test]
    fn test_builder_custom_memory() {
        let store = InMemoryStore::new();
        let runtime = AiRuntime::builder().memory(store).build();
        // Runtime is clonable (needed for State<T>)
        let _clone = runtime.clone();
    }

    #[test]
    fn test_runtime_is_clone_send_sync() {
        fn assert_clone_send_sync<T: Clone + Send + Sync>() {}
        assert_clone_send_sync::<AiRuntime>();
    }
}
