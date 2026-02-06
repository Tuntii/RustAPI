//! Replay - Time-travel debugging types and traits.
//!
//! This module provides the core data structures for recording HTTP
//! request/response pairs and computing diffs between replayed and
//! original responses. All types are framework-agnostic with no IO.
//!
//! # Security
//!
//! - Recording disabled by default
//! - Admin token required for all replay endpoints
//! - Sensitive headers (authorization, cookie, etc.) redacted by default
//! - JSON body field redaction supported
//! - Configurable TTL with automatic retention cleanup
//!
//! # Crate Organization
//!
//! - **rustapi-core** (this module): Pure types, traits, and utilities
//! - **rustapi-extras**: Middleware (`ReplayLayer`), stores, HTTP routes
//! - **cargo-rustapi**: CLI commands for replay management

mod config;
mod diff;
mod entry;
mod meta;
mod redaction;
mod store;
mod truncation;

pub use config::ReplayConfig;
pub use diff::{compute_diff, diff_json, BodyDiff, DiffField, DiffResult, FieldDiff};
pub use entry::{RecordedRequest, RecordedResponse, ReplayEntry, ReplayId};
pub use meta::ReplayMeta;
pub use redaction::{redact_body, redact_headers, RedactionConfig};
pub use store::{ReplayQuery, ReplayStore, ReplayStoreError, ReplayStoreResult};
pub use truncation::{content_sniff, truncate_body, try_parse_json, ContentKind, TruncationConfig};
