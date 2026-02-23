//! # rustapi-agent
//!
//! Step-based, deterministic agent execution engine for the RustAPI AI Runtime.
//!
//! ## Core Concepts
//!
//! - [`Step`] trait — a single unit of agent work
//! - [`AgentEngine`] — orchestrates step execution with planning and branching
//! - [`Planner`] trait — decomposes goals into execution plans
//! - [`AgentContext`] — mutable per-execution state accessible by steps
//! - [`ReplayEngine`] — deterministic replay from recorded trace trees
//!
//! ## Execution Flow
//!
//! ```text
//! Goal / Input
//!     │
//!     ▼
//! Planner::plan()  ──► ExecutionPlan (sequence of PlannedSteps)
//!     │
//!     ▼
//! AgentEngine::run() ──► for each step:
//!     │                      Step::execute(AgentContext) → StepResult
//!     │                          ├── Continue(value)    → next step
//!     │                          ├── Branch(name, val)  → jump to branch
//!     │                          ├── Yield(value)       → stream partial
//!     │                          └── Complete(value)    → finish
//!     │
//!     ▼
//! Final result + TraceTree
//! ```

mod context;
mod engine;
mod error;
mod planner;
mod replay;
mod step;

pub use context::*;
pub use engine::*;
pub use error::*;
pub use planner::*;
pub use replay::*;
pub use step::*;
