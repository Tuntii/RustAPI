//! Native OpenAPI traits for schema generation
//!
//! These traits replace external dependencies like `utoipa` with native implementations.

use serde_json::Value;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Trait for types that can be converted to an OpenAPI schema
///
/// This is the native replacement for `utoipa::ToSchema`. Implement this trait
/// for your types to automatically generate OpenAPI schema documentation.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_openapi::native::ToOpenApiSchema;
///
/// struct User {
///     id: i64,
///     name: String,
///     email: Option<String>,
/// }
///
/// impl ToOpenApiSchema for User {
///     fn schema() -> (Cow<'static, str>, Value) {
///         (
///             "User".into(),
///             serde_json::json!({
///                 "type": "object",
///                 "properties": {
///                     "id": { "type": "integer", "format": "int64" },
///                     "name": { "type": "string" },
///                     "email": { "type": "string", "nullable": true }
///                 },
///                 "required": ["id", "name"]
///             })
///         )
///     }
/// }
/// ```
pub trait ToOpenApiSchema {
    /// Return the schema name and the JSON Schema representation
    ///
    /// Returns a tuple of:
    /// - Schema name (used for `$ref` references in OpenAPI)
    /// - JSON Schema value following OpenAPI 3.0/3.1 specification
    fn schema() -> (Cow<'static, str>, Value);

    /// Get just the schema name
    fn schema_name() -> Cow<'static, str> {
        Self::schema().0
    }

    /// Get just the schema value
    fn schema_value() -> Value {
        Self::schema().1
    }

    /// Get a reference to this schema (for use in `$ref`)
    fn schema_ref() -> Value {
        let name = Self::schema_name();
        serde_json::json!({
            "$ref": format!("#/components/schemas/{}", name)
        })
    }
}

/// Location of a parameter in the HTTP request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamLocation {
    /// Parameter in URL path (e.g., /users/{id})
    Path,
    /// Parameter in query string (e.g., ?page=1)
    Query,
    /// Parameter in HTTP header
    Header,
    /// Parameter in cookie
    Cookie,
}

impl ParamLocation {
    /// Convert to OpenAPI string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ParamLocation::Path => "path",
            ParamLocation::Query => "query",
            ParamLocation::Header => "header",
            ParamLocation::Cookie => "cookie",
        }
    }
}

/// Information about a single parameter
#[derive(Debug, Clone)]
pub struct ParamInfo {
    /// Parameter name
    pub name: Cow<'static, str>,
    /// Parameter location
    pub location: ParamLocation,
    /// Whether the parameter is required
    pub required: bool,
    /// Parameter description
    pub description: Option<Cow<'static, str>>,
    /// Parameter schema (JSON Schema)
    pub schema: Value,
    /// Whether the parameter is deprecated
    pub deprecated: bool,
    /// Example value
    pub example: Option<Value>,
}

impl ParamInfo {
    /// Create a new required path parameter
    pub fn path(name: impl Into<Cow<'static, str>>, schema: Value) -> Self {
        Self {
            name: name.into(),
            location: ParamLocation::Path,
            required: true, // Path parameters are always required
            description: None,
            schema,
            deprecated: false,
            example: None,
        }
    }

    /// Create a new query parameter
    pub fn query(name: impl Into<Cow<'static, str>>, schema: Value, required: bool) -> Self {
        Self {
            name: name.into(),
            location: ParamLocation::Query,
            required,
            description: None,
            schema,
            deprecated: false,
            example: None,
        }
    }

    /// Create a new header parameter
    pub fn header(name: impl Into<Cow<'static, str>>, schema: Value, required: bool) -> Self {
        Self {
            name: name.into(),
            location: ParamLocation::Header,
            required,
            description: None,
            schema,
            deprecated: false,
            example: None,
        }
    }

    /// Add a description to this parameter
    pub fn with_description(mut self, desc: impl Into<Cow<'static, str>>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Mark this parameter as deprecated
    pub fn deprecated(mut self) -> Self {
        self.deprecated = true;
        self
    }

    /// Add an example value
    pub fn with_example(mut self, example: Value) -> Self {
        self.example = Some(example);
        self
    }

    /// Convert to OpenAPI parameter object
    pub fn to_openapi(&self) -> Value {
        let mut param = serde_json::json!({
            "name": self.name,
            "in": self.location.as_str(),
            "required": self.required,
            "schema": self.schema
        });

        if let Some(desc) = &self.description {
            param["description"] = Value::String(desc.to_string());
        }

        if self.deprecated {
            param["deprecated"] = Value::Bool(true);
        }

        if let Some(example) = &self.example {
            param["example"] = example.clone();
        }

        param
    }
}

/// Trait for types that can be converted to OpenAPI parameters
///
/// This is the native replacement for `utoipa::IntoParams`. Implement this trait
/// for types that represent query parameters, path parameters, or headers.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_openapi::native::{IntoOpenApiParams, ParamInfo, ParamLocation};
///
/// struct PaginationQuery {
///     page: Option<u32>,
///     per_page: Option<u32>,
/// }
///
/// impl IntoOpenApiParams for PaginationQuery {
///     fn params() -> Vec<ParamInfo> {
///         vec![
///             ParamInfo::query("page", serde_json::json!({"type": "integer"}), false)
///                 .with_description("Page number (1-indexed)"),
///             ParamInfo::query("per_page", serde_json::json!({"type": "integer"}), false)
///                 .with_description("Number of items per page"),
///         ]
///     }
/// }
/// ```
pub trait IntoOpenApiParams {
    /// Return a list of parameter definitions
    fn params() -> Vec<ParamInfo>;

    /// Convert parameters to OpenAPI parameter objects
    fn to_openapi_params() -> Vec<Value> {
        Self::params().iter().map(|p| p.to_openapi()).collect()
    }
}

// ============================================================================
// Implementations for primitive types
// ============================================================================

impl ToOpenApiSchema for bool {
    fn schema() -> (Cow<'static, str>, Value) {
        ("boolean".into(), serde_json::json!({ "type": "boolean" }))
    }
}

impl ToOpenApiSchema for i8 {
    fn schema() -> (Cow<'static, str>, Value) {
        (
            "i8".into(),
            serde_json::json!({ "type": "integer", "format": "int32", "minimum": -128, "maximum": 127 }),
        )
    }
}

impl ToOpenApiSchema for i16 {
    fn schema() -> (Cow<'static, str>, Value) {
        (
            "i16".into(),
            serde_json::json!({ "type": "integer", "format": "int32", "minimum": -32768, "maximum": 32767 }),
        )
    }
}

impl ToOpenApiSchema for i32 {
    fn schema() -> (Cow<'static, str>, Value) {
        (
            "i32".into(),
            serde_json::json!({ "type": "integer", "format": "int32" }),
        )
    }
}

impl ToOpenApiSchema for i64 {
    fn schema() -> (Cow<'static, str>, Value) {
        (
            "i64".into(),
            serde_json::json!({ "type": "integer", "format": "int64" }),
        )
    }
}

impl ToOpenApiSchema for i128 {
    fn schema() -> (Cow<'static, str>, Value) {
        (
            "i128".into(),
            serde_json::json!({ "type": "integer", "format": "int64" }),
        )
    }
}

impl ToOpenApiSchema for isize {
    fn schema() -> (Cow<'static, str>, Value) {
        (
            "isize".into(),
            serde_json::json!({ "type": "integer", "format": "int64" }),
        )
    }
}

impl ToOpenApiSchema for u8 {
    fn schema() -> (Cow<'static, str>, Value) {
        (
            "u8".into(),
            serde_json::json!({ "type": "integer", "format": "int32", "minimum": 0, "maximum": 255 }),
        )
    }
}

impl ToOpenApiSchema for u16 {
    fn schema() -> (Cow<'static, str>, Value) {
        (
            "u16".into(),
            serde_json::json!({ "type": "integer", "format": "int32", "minimum": 0, "maximum": 65535 }),
        )
    }
}

impl ToOpenApiSchema for u32 {
    fn schema() -> (Cow<'static, str>, Value) {
        (
            "u32".into(),
            serde_json::json!({ "type": "integer", "format": "int32", "minimum": 0 }),
        )
    }
}

impl ToOpenApiSchema for u64 {
    fn schema() -> (Cow<'static, str>, Value) {
        (
            "u64".into(),
            serde_json::json!({ "type": "integer", "format": "int64", "minimum": 0 }),
        )
    }
}

impl ToOpenApiSchema for u128 {
    fn schema() -> (Cow<'static, str>, Value) {
        (
            "u128".into(),
            serde_json::json!({ "type": "integer", "format": "int64", "minimum": 0 }),
        )
    }
}

impl ToOpenApiSchema for usize {
    fn schema() -> (Cow<'static, str>, Value) {
        (
            "usize".into(),
            serde_json::json!({ "type": "integer", "format": "int64", "minimum": 0 }),
        )
    }
}

impl ToOpenApiSchema for f32 {
    fn schema() -> (Cow<'static, str>, Value) {
        (
            "f32".into(),
            serde_json::json!({ "type": "number", "format": "float" }),
        )
    }
}

impl ToOpenApiSchema for f64 {
    fn schema() -> (Cow<'static, str>, Value) {
        (
            "f64".into(),
            serde_json::json!({ "type": "number", "format": "double" }),
        )
    }
}

impl ToOpenApiSchema for String {
    fn schema() -> (Cow<'static, str>, Value) {
        ("String".into(), serde_json::json!({ "type": "string" }))
    }
}

impl ToOpenApiSchema for str {
    fn schema() -> (Cow<'static, str>, Value) {
        ("str".into(), serde_json::json!({ "type": "string" }))
    }
}

impl ToOpenApiSchema for char {
    fn schema() -> (Cow<'static, str>, Value) {
        (
            "char".into(),
            serde_json::json!({ "type": "string", "minLength": 1, "maxLength": 1 }),
        )
    }
}

impl ToOpenApiSchema for () {
    fn schema() -> (Cow<'static, str>, Value) {
        ("unit".into(), serde_json::json!({ "type": "null" }))
    }
}

// ============================================================================
// Implementations for Option<T>
// ============================================================================

impl<T: ToOpenApiSchema> ToOpenApiSchema for Option<T> {
    fn schema() -> (Cow<'static, str>, Value) {
        let (inner_name, inner_schema) = T::schema();
        let name = Cow::Owned(format!("Option_{}", inner_name));

        // Make the schema nullable
        let mut schema = inner_schema;
        if let Value::Object(ref mut map) = schema {
            map.insert("nullable".to_string(), Value::Bool(true));
        }

        (name, schema)
    }
}

// ============================================================================
// Implementations for collections
// ============================================================================

impl<T: ToOpenApiSchema> ToOpenApiSchema for Vec<T> {
    fn schema() -> (Cow<'static, str>, Value) {
        let (inner_name, inner_schema) = T::schema();
        let name = Cow::Owned(format!("Vec_{}", inner_name));

        (
            name,
            serde_json::json!({
                "type": "array",
                "items": inner_schema
            }),
        )
    }
}

impl<T: ToOpenApiSchema, const N: usize> ToOpenApiSchema for [T; N] {
    fn schema() -> (Cow<'static, str>, Value) {
        let (inner_name, inner_schema) = T::schema();
        let name = Cow::Owned(format!("Array_{}_{}", inner_name, N));

        (
            name,
            serde_json::json!({
                "type": "array",
                "items": inner_schema,
                "minItems": N,
                "maxItems": N
            }),
        )
    }
}

impl<T: ToOpenApiSchema> ToOpenApiSchema for HashSet<T> {
    fn schema() -> (Cow<'static, str>, Value) {
        let (inner_name, inner_schema) = T::schema();
        let name = Cow::Owned(format!("HashSet_{}", inner_name));

        (
            name,
            serde_json::json!({
                "type": "array",
                "items": inner_schema,
                "uniqueItems": true
            }),
        )
    }
}

impl<T: ToOpenApiSchema> ToOpenApiSchema for BTreeSet<T> {
    fn schema() -> (Cow<'static, str>, Value) {
        let (inner_name, inner_schema) = T::schema();
        let name = Cow::Owned(format!("BTreeSet_{}", inner_name));

        (
            name,
            serde_json::json!({
                "type": "array",
                "items": inner_schema,
                "uniqueItems": true
            }),
        )
    }
}

impl<K: ToOpenApiSchema, V: ToOpenApiSchema> ToOpenApiSchema for HashMap<K, V> {
    fn schema() -> (Cow<'static, str>, Value) {
        let (_, value_schema) = V::schema();
        let (key_name, _) = K::schema();
        let (value_name, _) = V::schema();
        let name = Cow::Owned(format!("HashMap_{}_{}", key_name, value_name));

        (
            name,
            serde_json::json!({
                "type": "object",
                "additionalProperties": value_schema
            }),
        )
    }
}

impl<K: ToOpenApiSchema, V: ToOpenApiSchema> ToOpenApiSchema for BTreeMap<K, V> {
    fn schema() -> (Cow<'static, str>, Value) {
        let (_, value_schema) = V::schema();
        let (key_name, _) = K::schema();
        let (value_name, _) = V::schema();
        let name = Cow::Owned(format!("BTreeMap_{}_{}", key_name, value_name));

        (
            name,
            serde_json::json!({
                "type": "object",
                "additionalProperties": value_schema
            }),
        )
    }
}

// ============================================================================
// Implementations for Box, Rc, Arc
// ============================================================================

impl<T: ToOpenApiSchema> ToOpenApiSchema for Box<T> {
    fn schema() -> (Cow<'static, str>, Value) {
        T::schema()
    }
}

impl<T: ToOpenApiSchema> ToOpenApiSchema for std::rc::Rc<T> {
    fn schema() -> (Cow<'static, str>, Value) {
        T::schema()
    }
}

impl<T: ToOpenApiSchema> ToOpenApiSchema for std::sync::Arc<T> {
    fn schema() -> (Cow<'static, str>, Value) {
        T::schema()
    }
}

impl<T: ToOpenApiSchema + ?Sized> ToOpenApiSchema for std::borrow::Cow<'_, T>
where
    T: ToOwned,
{
    fn schema() -> (Cow<'static, str>, Value) {
        T::schema()
    }
}

// ============================================================================
// Implementations for Result<T, E>
// ============================================================================

impl<T: ToOpenApiSchema, E: ToOpenApiSchema> ToOpenApiSchema for Result<T, E> {
    fn schema() -> (Cow<'static, str>, Value) {
        let (ok_name, ok_schema) = T::schema();
        let (err_name, err_schema) = E::schema();
        let name = Cow::Owned(format!("Result_{}_{}", ok_name, err_name));

        (
            name,
            serde_json::json!({
                "oneOf": [
                    {
                        "type": "object",
                        "properties": {
                            "Ok": ok_schema
                        },
                        "required": ["Ok"]
                    },
                    {
                        "type": "object",
                        "properties": {
                            "Err": err_schema
                        },
                        "required": ["Err"]
                    }
                ]
            }),
        )
    }
}

// ============================================================================
// Implementations for tuples
// ============================================================================

impl<A: ToOpenApiSchema> ToOpenApiSchema for (A,) {
    fn schema() -> (Cow<'static, str>, Value) {
        let (a_name, a_schema) = A::schema();
        let name = Cow::Owned(format!("Tuple1_{}", a_name));

        (
            name,
            serde_json::json!({
                "type": "array",
                "prefixItems": [a_schema],
                "minItems": 1,
                "maxItems": 1
            }),
        )
    }
}

impl<A: ToOpenApiSchema, B: ToOpenApiSchema> ToOpenApiSchema for (A, B) {
    fn schema() -> (Cow<'static, str>, Value) {
        let (a_name, a_schema) = A::schema();
        let (b_name, b_schema) = B::schema();
        let name = Cow::Owned(format!("Tuple2_{}_{}", a_name, b_name));

        (
            name,
            serde_json::json!({
                "type": "array",
                "prefixItems": [a_schema, b_schema],
                "minItems": 2,
                "maxItems": 2
            }),
        )
    }
}

impl<A: ToOpenApiSchema, B: ToOpenApiSchema, C: ToOpenApiSchema> ToOpenApiSchema for (A, B, C) {
    fn schema() -> (Cow<'static, str>, Value) {
        let (a_name, a_schema) = A::schema();
        let (b_name, b_schema) = B::schema();
        let (c_name, c_schema) = C::schema();
        let name = Cow::Owned(format!("Tuple3_{}_{}_{}", a_name, b_name, c_name));

        (
            name,
            serde_json::json!({
                "type": "array",
                "prefixItems": [a_schema, b_schema, c_schema],
                "minItems": 3,
                "maxItems": 3
            }),
        )
    }
}

impl<A: ToOpenApiSchema, B: ToOpenApiSchema, C: ToOpenApiSchema, D: ToOpenApiSchema>
    ToOpenApiSchema for (A, B, C, D)
{
    fn schema() -> (Cow<'static, str>, Value) {
        let (a_name, a_schema) = A::schema();
        let (b_name, b_schema) = B::schema();
        let (c_name, c_schema) = C::schema();
        let (d_name, d_schema) = D::schema();
        let name = Cow::Owned(format!(
            "Tuple4_{}_{}_{}_{}", a_name, b_name, c_name, d_name
        ));

        (
            name,
            serde_json::json!({
                "type": "array",
                "prefixItems": [a_schema, b_schema, c_schema, d_schema],
                "minItems": 4,
                "maxItems": 4
            }),
        )
    }
}
