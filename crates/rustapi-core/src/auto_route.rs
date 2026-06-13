//! Auto-route registration using linkme distributed slices
//!
//! **Implementation detail**: This module provides the current backend for
//! automatic, zero-boilerplate route collection using the `linkme` crate's
//! distributed slice mechanism.
//!
//! This is an internal implementation detail of `RustApi::auto()`.
//! The public contract is only:
//! - Handlers annotated with the route macros are discovered automatically.
//! - `collect_auto_routes()` and `auto_route_count()` for introspection.
//!
//! We may change the underlying mechanism in the future (while keeping the
//! observable behavior stable).
//!
//! This module enables **zero-config route registration**. Routes decorated with
//! `#[rustapi_rs::get]`, `#[rustapi_rs::post]`, etc. are automatically collected at
//! **link time** using the [`linkme`](https://docs.rs/linkme) crate.
//!
//! `RustApi::auto()` (and `RustApi::config()`) rely on this mechanism.
//!
//! # How It Works
//!
//! The attribute macros emit a small static initializer that appends a factory
//! function into a `linkme::distributed_slice`. At runtime we simply iterate the
//! slice and build the router.
//!
//! After collection we sort routes into a `BTreeMap` so registration order is
//! deterministic regardless of link order.
//!
//! # Public API
//!
//! - [`collect_auto_routes`] – returns all discovered routes as `Vec<Route>`
//! - [`auto_route_count`] – cheap way to check how many handlers were linked in
//!
//! # Example
//!
//! ```rust,ignore
//! use rustapi_rs::prelude::*;
//!
//! #[rustapi_rs::get("/users")]
//! async fn list_users() -> Json<Vec<User>> {
//!     Json(vec![])
//! }
//!
//! #[rustapi_rs::post("/users")]
//! async fn create_user(Json(body): Json<CreateUser>) -> Created<User> {
//!     // ...
//! }
//!
//! #[rustapi_rs::main]
//! async fn main() -> Result<()> {
//!     // No manual .route() calls needed!
//!     RustApi::auto()
//!         .run("0.0.0.0:8080")
//!         .await
//! }
//! ```
//!
//! # Limitations & Known Gotchas
//!
//! Link-time registration is powerful but comes with trade-offs:
//!
//! - **Tests can be flaky** — the test binary sometimes links differently than the
//!   main binary. You may see `auto_route_count() == 0` inside `#[test]` even if
//!   your annotated functions exist. Use `collect_auto_routes()` + filtering or
//!   fall back to manual `.route()` in tests when necessary.
//!
//! - **Non-executable artifacts** (cdylib, rlib, staticlib, wasm32, etc.) often do
//!   **not** populate distributed slices reliably because the linker may discard
//!   the registration code.
//!
//! - **Multiple separate binaries** in the same workspace each get their own
//!   independent slice. Routes defined in one binary are invisible to another.
//!
//! - There is **no runtime unregistration**. Once linked, the routes are there for
//!   the lifetime of the process.
//!
//! - If you see zero routes at runtime with `RustApi::auto()`, the most common
//!   causes are:
//!   1. No handlers were annotated with the route attributes.
//!   2. The module containing the handler was not linked into this binary.
//!   3. You are inside a test, library, or cdylib target.
//!
//! A clear warning is now emitted automatically when `RustApi::auto()` collects
//! zero routes (see `mount_auto_routes_grouped`).
//!
//! You can always inspect the situation with:
//!
//! ```rust,ignore
//! println!("Auto routes discovered: {}", rustapi_rs::auto_route_count());
//! ```
//!
//! # Manual Registration (always available)
//!
//! If auto-registration causes problems in your environment, you can ignore the
//! attribute macros entirely and use the classic builder:
//!
//! ```rust,ignore
//! RustApi::new()
//!     .route("/users", get(list_users).post(create_user))
//!     .run("0.0.0.0:8080")
//!     .await
//! ```
//!
//! Both styles can be mixed freely.

use crate::handler::Route;
use linkme::distributed_slice;

/// Distributed slice containing all auto-registered route factory functions.
///
/// Each element is a function that returns a [`Route`] when called.
/// The macro `#[rustapi::get]`, `#[rustapi::post]`, etc. automatically
/// add entries to this slice at compile/link time.
#[distributed_slice]
pub static AUTO_ROUTES: [fn() -> Route];

/// Collect all auto-registered routes.
///
/// This function iterates over the distributed slice and calls each
/// route factory function to produce the actual [`Route`] instances.
///
/// # Returns
///
/// A vector containing all routes that were registered using the
/// `#[rustapi::get]`, `#[rustapi::post]`, etc. macros.
///
/// # Example
///
/// ```rust,ignore
/// let routes = collect_auto_routes();
/// println!("Found {} auto-registered routes", routes.len());
/// ```
pub fn collect_auto_routes() -> Vec<Route> {
    AUTO_ROUTES.iter().map(|f| f()).collect()
}

/// Get the count of auto-registered routes without collecting them.
///
/// This is useful for:
/// - Debugging (e.g. in tests or startup logs)
/// - Asserting that your annotated handlers were actually linked in
///
/// # Example
///
/// ```rust,ignore
/// let count = rustapi_core::auto_route_count();
/// if count == 0 {
///     eprintln!("Warning: No auto-routes were discovered!");
/// }
/// ```
pub fn auto_route_count() -> usize {
    AUTO_ROUTES.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_routes_slice_exists() {
        // The slice always exists (even if empty). This test mainly ensures
        // the linkme static was emitted correctly by the build.
        let _count = auto_route_count();
    }

    #[test]
    fn test_collect_auto_routes_does_not_panic() {
        // Collection must be safe even when no annotated handlers are present
        // in the current test binary (very common situation).
        let routes = collect_auto_routes();
        // We don't assert a specific count here because linkme behavior in
        // test binaries is not guaranteed to be the same as in the final binary.
        let _ = routes;
    }

    #[test]
    fn test_auto_route_count_is_accessible() {
        // Public API smoke test – users and integration tests should be able
        // to call this without importing internal modules.
        let _ = auto_route_count();
    }
}
