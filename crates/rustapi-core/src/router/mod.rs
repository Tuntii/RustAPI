//! Router implementation using radix tree (matchit)
//!
//! This module provides HTTP routing functionality for RustAPI. Routes are
//! registered using path patterns and HTTP method handlers.

mod conflict;
mod core;
mod match_;
mod method_router;

pub use core::Router;
pub use match_::RouteMatch;
#[cfg(test)]
pub(crate) use match_::{convert_path_params, normalize_path_for_comparison, normalize_prefix};
pub use method_router::{delete, get, patch, post, put, MethodRouter};

#[cfg(test)]
mod tests {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/router_lib.rs"
    ));
}
