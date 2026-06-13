//! Auto-schema registration using linkme distributed slices
//!
//! **Implementation detail**: Companion to `auto_route`.
//! Provides link-time registration of types that should appear in the generated
//! OpenAPI spec (via `#[rustapi_rs::schema]` and implicit schema derivation).
//!
//! This is an internal detail. The only stable surface is that schemas referenced
//! by auto-registered routes (and explicitly annotated types) end up in the
//! OpenAPI document when using `RustApi::auto()`.
//!
//! This module enables zero-config OpenAPI schema registration.
//! Route macros can register schemas at link-time, and `RustApi::auto()`
//! will collect and apply them before serving docs.

use linkme::distributed_slice;

/// Distributed slice containing all auto-registered schema registration functions.
///
/// Each element is a function that takes a mutable reference to the current
/// [`rustapi_openapi::OpenApiSpec`] and registers one or more schemas.
#[distributed_slice]
pub static AUTO_SCHEMAS: [fn(&mut rustapi_openapi::OpenApiSpec)];

/// Apply all auto-registered schemas into the given OpenAPI spec.
pub fn apply_auto_schemas(spec: &mut rustapi_openapi::OpenApiSpec) {
    for f in AUTO_SCHEMAS.iter() {
        f(spec);
    }
}

/// Get the count of auto-registered schema registration functions.
#[allow(dead_code)]
pub fn auto_schema_count() -> usize {
    AUTO_SCHEMAS.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_schemas_slice_exists() {
        let _count = auto_schema_count();
    }

    #[test]
    fn test_apply_auto_schemas_does_not_panic() {
        let mut spec = rustapi_openapi::OpenApiSpec::new("Test", "0.0.0");
        apply_auto_schemas(&mut spec);
    }
}
