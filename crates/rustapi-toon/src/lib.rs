//! # TOON Format Support for RustAPI
//!
//! This crate provides [TOON (Token-Oriented Object Notation)](https://toonformat.dev/)
//! support for the RustAPI framework. TOON is a compact, human-readable format
//! designed for passing structured data to Large Language Models (LLMs) with
//! significantly reduced token usage (typically 20-40% savings).
//!
//! ## Quick Example
//!
//! **JSON (16 tokens, 40 bytes):**
//! ```json
//! {
//!   "users": [
//!     { "id": 1, "name": "Alice" },
//!     { "id": 2, "name": "Bob" }
//!   ]
//! }
//! ```
//!
//! **TOON (13 tokens, 28 bytes) - 18.75% token savings:**
//! ```text
//! users[2]{id,name}:
//!   1,Alice
//!   2,Bob
//! ```
//!
//! ## Usage
//!
//! Add to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! rustapi-rs = { version = "0.1.275", features = ["toon"] }
//! ```
//!
//! ### Toon Extractor
//!
//! Parse TOON request bodies:
//!
//! ```rust,ignore
//! use rustapi_rs::prelude::*;
//! use rustapi_rs::toon::Toon;
//!
//! #[derive(Deserialize)]
//! struct CreateUser {
//!     name: String,
//!     email: String,
//! }
//!
//! async fn create_user(Toon(user): Toon<CreateUser>) -> impl IntoResponse {
//!     Json(user)
//! }
//! ```
//!
//! ### Toon Response
//!
//! Return TOON formatted responses:
//!
//! ```rust,ignore
//! use rustapi_rs::prelude::*;
//! use rustapi_rs::toon::Toon;
//!
//! #[derive(Serialize)]
//! struct User {
//!     id: u64,
//!     name: String,
//! }
//!
//! async fn get_user() -> Toon<User> {
//!     Toon(User {
//!         id: 1,
//!         name: "Alice".to_string(),
//!     })
//! }
//! ```
//!
//! ## Content Types
//!
//! - Request: `application/toon` or `text/toon`
//! - Response: `application/toon`

mod error;
mod extractor;
mod llm_response;
mod negotiate;
mod openapi;

pub use error::ToonError;
pub use extractor::Toon;
pub use llm_response::{
    LlmResponse, X_FORMAT_USED, X_TOKEN_COUNT_JSON, X_TOKEN_COUNT_TOON, X_TOKEN_SAVINGS,
};
pub use negotiate::{AcceptHeader, MediaTypeEntry, Negotiate, OutputFormat, JSON_CONTENT_TYPE};
pub use openapi::{
    api_description_with_toon, format_comparison_example, token_headers_schema, toon_extension,
    toon_schema, TOON_FORMAT_DESCRIPTION,
};

// Re-export toon-format types for advanced usage
pub use toon_format::{
    decode, decode_default, encode, encode_default, DecodeOptions, EncodeOptions,
};

/// TOON Content-Type header value
pub const TOON_CONTENT_TYPE: &str = "application/toon";

/// Alternative TOON Content-Type (text-based)
pub const TOON_CONTENT_TYPE_TEXT: &str = "text/toon";
