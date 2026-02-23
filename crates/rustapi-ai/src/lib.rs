//! # rustapi-ai
//!
//! **AI-Native Backend Runtime** — unified facade over the RustAPI AI crate
//! family. This crate re-exports every AI primitive and provides the
//! [`AiRuntime`] builder that wires context, memory, tools, agents, and LLM
//! routing into a single coherent runtime.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use rustapi_ai::prelude::*;
//!
//! # async fn example() {
//! let runtime = AiRuntime::builder()
//!     .memory(InMemoryStore::new())
//!     .llm(LlmRouter::builder()
//!         .provider(MockProvider::new("dev"))
//!         .build())
//!     .build();
//! # }
//! ```
//!
//! ## Pipeline
//!
//! ```text
//! HTTP Request
//!     │
//!     ▼
//! RequestContext  ← cost budget, trace tree, event bus
//!     │
//!     ▼
//! AgentEngine    ← step-based execution loop
//!     ├── Planner        (decides execution plan)
//!     ├── ToolGraph      (parallel / sequential tool execution)
//!     ├── MemoryStore    (conversation + semantic memory)
//!     └── LlmRouter     (cost-aware, fallback-capable LLM calls)
//!     │
//!     ▼
//! StructuredOutput<T>    ← schema-first guaranteed decoding
//!     │
//!     ▼
//! HTTP Response  (Json / Toon / SSE stream)
//! ```

// Re-export sub-crates
pub use rustapi_context as context;
pub use rustapi_memory as memory;
pub use rustapi_tools as tools;
pub use rustapi_agent as agent;
pub use rustapi_llm as llm;

#[cfg(feature = "http")]
pub mod middleware;

mod runtime;

pub use runtime::*;

/// Convenience prelude — import everything needed for AI-native handlers.
pub mod prelude {
    // Context
    pub use rustapi_context::{
        AuthContext, CostBudget, CostSnapshot, EventBus, EventSubscriber,
        ExecutionEvent, RequestContext, RequestContextBuilder, SharedCostTracker,
        TraceNode, TraceTree,
    };

    // Memory
    pub use rustapi_memory::{
        ConversationMemory, MemoryEntry, MemoryQuery, MemoryStore,
        Role as MemoryRole, ScoredEntry, SemanticMemoryStore, SemanticQuery, Turn,
    };
    pub use rustapi_memory::backend::InMemoryStore;
    #[cfg(feature = "redis")]
    pub use rustapi_memory::backend::RedisStore;

    // Tools
    pub use rustapi_tools::{
        ClosureTool, FunctionDefinition, GraphOutput, SideEffect, Tool,
        ToolDescription, ToolGraph, ToolNode, ToolOutput, ToolRegistry,
    };

    // Agent
    pub use rustapi_agent::{
        AgentContext, AgentEngine, AgentResult, ClosureStep, EngineConfig,
        ExecutionPlan, Planner, PlannedStep, ReActPlanner,
        ReplayEngine, ReplayResult, ReplaySession, StaticPlanner,
        Step, StepResult,
    };

    // LLM
    pub use rustapi_llm::{
        CompletionRequest, CompletionResponse, FinishReason, FunctionDef,
        LlmProvider, LlmRouter, Message, MessageRole, MockProvider,
        ModelInfo, RoutingStrategy, StreamChunk, StructuredOutput,
        TokenUsage, ToolCall,
    };

    // This crate
    pub use crate::{AiRuntime, AiRuntimeBuilder};
}
