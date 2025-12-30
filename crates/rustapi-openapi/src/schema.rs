//! Schema trait for OpenAPI type generation.

/// Marker trait for types that can be converted to OpenAPI schemas.
///
/// This trait is automatically implemented via `#[derive(ToSchema)]`
/// using the utoipa crate internally.
///
/// # Example
///
/// ```rust,ignore
/// use utoipa::ToSchema;
///
/// #[derive(ToSchema)]
/// struct User {
///     id: i64,
///     name: String,
///     email: String,
/// }
/// ```
pub trait ToSchema {
    /// Get the schema name.
    fn schema_name() -> &'static str;
}

// Implement for common types
impl ToSchema for String {
    fn schema_name() -> &'static str {
        "string"
    }
}

impl ToSchema for i32 {
    fn schema_name() -> &'static str {
        "integer"
    }
}

impl ToSchema for i64 {
    fn schema_name() -> &'static str {
        "integer"
    }
}

impl ToSchema for f64 {
    fn schema_name() -> &'static str {
        "number"
    }
}

impl ToSchema for bool {
    fn schema_name() -> &'static str {
        "boolean"
    }
}

impl<T: ToSchema> ToSchema for Vec<T> {
    fn schema_name() -> &'static str {
        "array"
    }
}

impl<T: ToSchema> ToSchema for Option<T> {
    fn schema_name() -> &'static str {
        T::schema_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_schemas() {
        assert_eq!(String::schema_name(), "string");
        assert_eq!(i32::schema_name(), "integer");
        assert_eq!(i64::schema_name(), "integer");
        assert_eq!(f64::schema_name(), "number");
        assert_eq!(bool::schema_name(), "boolean");
    }

    #[test]
    fn container_schemas() {
        assert_eq!(Vec::<String>::schema_name(), "array");
        assert_eq!(Option::<String>::schema_name(), "string");
    }
}
