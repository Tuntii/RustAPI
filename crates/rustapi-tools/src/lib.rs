//! # rustapi-tools
//!
//! Tool registry and declarative execution graph for the RustAPI AI Runtime.
//!
//! ## Core Concepts
//!
//! - [`Tool`] trait — defines a callable tool with typed input/output
//! - [`ToolRegistry`] — runtime registry for discovering and invoking tools
//! - [`ToolGraph`] — a DAG of tool invocations with dependency resolution
//! - [`ToolNode`] — nodes in the execution graph (tool calls, parallel, conditional, etc.)
//!
//! ## Architecture
//!
//! ```text
//! Agent Engine
//!     │
//!     ▼
//! ToolGraph (DAG of ToolNodes)
//!     │
//!     ├── ToolNode::Call("web_search")   ──► ToolRegistry::execute()
//!     ├── ToolNode::Parallel([...])      ──► tokio::JoinSet
//!     ├── ToolNode::Conditional(pred)    ──► branch selection
//!     └── ToolNode::Sequence([...])      ──► ordered execution
//! ```

mod error;
mod graph;
mod registry;
mod tool;

pub use error::*;
pub use graph::*;
pub use registry::*;
pub use tool::*;
