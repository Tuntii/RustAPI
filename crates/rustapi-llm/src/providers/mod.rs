//! Concrete LLM provider implementations.
//!
//! Each provider is feature-gated:
//! - `openai` → [`OpenAiProvider`]
//! - `anthropic` → [`AnthropicProvider`]
//! - `local` → [`LocalProvider`] (Ollama / vLLM)

#[cfg(feature = "openai")]
mod openai;
#[cfg(feature = "openai")]
pub use openai::*;

#[cfg(feature = "anthropic")]
mod anthropic;
#[cfg(feature = "anthropic")]
pub use anthropic::*;

#[cfg(feature = "local")]
mod local;
#[cfg(feature = "local")]
pub use local::*;
