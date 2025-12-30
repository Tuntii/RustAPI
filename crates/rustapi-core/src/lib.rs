//! # RustAPI Core
//!
//! Core library providing the foundational types and traits for RustAPI.
//!
//! This crate is not meant to be used directly. Use `rustapi-rs` instead.

mod app;
mod error;
mod extract;
mod handler;
mod request;
mod response;
mod router;
mod server;

// Public API
pub use app::RustApi;
pub use error::{ApiError, Result};
pub use extract::{Body, FromRequest, FromRequestParts, Json, Path, Query, State, ValidatedJson};
pub use handler::{
    Handler, HandlerService, Route, RouteHandler,
    get_route, post_route, put_route, patch_route, delete_route,
};
pub use request::Request;
pub use response::{Created, Html, IntoResponse, NoContent, Redirect, Response};
pub use router::{delete, get, patch, post, put, MethodRouter, Router};
