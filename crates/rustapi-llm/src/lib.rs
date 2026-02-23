//! # rustapi-llm
//!
//! Model-agnostic LLM router with structured output, function calling, and
//! cost-aware routing for the RustAPI AI Runtime.
//!
//! ## Core Concepts
//!
//! - [`LlmProvider`] trait — abstraction over any LLM API (OpenAI, Anthropic, local)
//! - [`LlmRouter`] — cost-aware, fallback-capable model router
//! - [`CompletionRequest`] / [`CompletionResponse`] — model-agnostic I/O types
//! - [`StructuredOutput`] — schema-first, guaranteed structured decoding
//! - [`MockProvider`] — deterministic provider for testing
//!
//! ## Architecture
//!
//! ```text
//! Agent Step
//!     │
//!     ▼
//! LlmRouter
//!     ├── RoutingStrategy (cost / latency / quality)
//!     ├── FallbackChain [primary → secondary → tertiary]
//!     └── CircuitBreaker (per provider)
//!     │
//!     ▼
//! LlmProvider trait
//!     ├── OpenAiProvider   (feature: openai)
//!     ├── AnthropicProvider (feature: anthropic)
//!     ├── LocalProvider    (feature: local)
//!     └── MockProvider     (always available)
//! ```

mod error;
mod provider;
pub mod providers;
mod router;
mod structured;
mod types;

pub use error::*;
pub use provider::*;
pub use router::*;
pub use structured::*;
pub use types::*;
