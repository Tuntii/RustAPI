//! Auto-route registration using linkme distributed slices
//!
//! This module enables zero-config route registration. Routes decorated with
//! `#[rustapi::get]`, `#[rustapi::post]`, etc. are automatically collected at link-time.
//!
//! # How It Works
//!
//! When you use `#[rustapi::get("/path")]` or similar macros, they generate a
//! static registration that adds the route factory function to a distributed slice.
//! At runtime, `RustApi::auto()` collects all these routes and registers them.
//!
//! # Example
//!
//! ```rust,ignore
//! use rustapi_rs::prelude::*;
//!
//! #[rustapi::get("/users")]
//! async fn list_users() -> Json<Vec<User>> {
//!     Json(vec![])
//! }
//!
//! #[rustapi::post("/users")]
//! async fn create_user(Json(body): Json<CreateUser>) -> Created<User> {
//!     // ...
//! }
//!
//! #[rustapi::main]
//! async fn main() -> Result<()> {
//!     // Routes are auto-registered, no need for manual .mount() calls!
//!     RustApi::auto()
//!         .run("0.0.0.0:8080")
//!         .await
//! }
//! ```

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
/// Useful for debugging and logging.
#[allow(dead_code)]
pub fn auto_route_count() -> usize {
    AUTO_ROUTES.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_routes_slice_exists() {
        // The slice should exist, even if empty initially
        let _count = auto_route_count();
    }

    #[test]
    fn test_collect_auto_routes() {
        // Should not panic, returns empty vec if no routes registered
        let routes = collect_auto_routes();
        // In tests, we may or may not have routes depending on what's linked
        let _ = routes;
    }
}
