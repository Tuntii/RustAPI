//! OpenAPI specification types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// API information for OpenAPI spec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInfo {
    pub title: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// OpenAPI specification builder
#[derive(Debug, Clone)]
pub struct OpenApiSpec {
    pub info: ApiInfo,
    pub paths: HashMap<String, PathItem>,
    pub schemas: HashMap<String, crate::schema::RefOr<crate::schema::Schema>>,
}

/// Path item in OpenAPI spec
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PathItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub put: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<Operation>,
}

/// Operation (endpoint) in OpenAPI spec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<Parameter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "requestBody")]
    pub request_body: Option<RequestBody>,
    pub responses: HashMap<String, ResponseSpec>,
}

/// Parameter location
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ParameterIn {
    Query,
    Header,
    Path,
    Cookie,
}

impl ToString for ParameterIn {
    fn to_string(&self) -> String {
        match self {
            Self::Query => "query".to_string(),
            Self::Header => "header".to_string(),
            Self::Path => "path".to_string(),
            Self::Cookie => "cookie".to_string(),
        }
    }
}

/// Trait for types that can populate OpenAPI parameters
pub trait IntoParams {
    fn into_params(parameter_in_provider: impl Fn() -> Option<ParameterIn>) -> Vec<Parameter>;
}

/// Parameter in OpenAPI spec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    #[serde(rename = "in")]
    pub location: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub schema: SchemaRef,
}

/// Request body in OpenAPI spec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    pub required: bool,
    pub content: HashMap<String, MediaType>,
}

/// Media type in OpenAPI spec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaType {
    pub schema: SchemaRef,
}

/// Response specification
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResponseSpec {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<HashMap<String, MediaType>>,
}

/// Schema reference or inline schema
///
/// Legacy type alias to bridge old and new code if needed.
/// In new code, prefer crate::schema::RefOr<crate::schema::Schema>
pub type SchemaRef = crate::schema::RefOr<crate::schema::Schema>;

impl OpenApiSpec {
    /// Create a new OpenAPI specification
    pub fn new(title: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            info: ApiInfo {
                title: title.into(),
                version: version.into(),
                description: None,
            },
            paths: HashMap::new(),
            schemas: HashMap::new(),
        }
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.info.description = Some(desc.into());
        self
    }

    /// Add a path operation
    pub fn path(mut self, path: &str, method: &str, operation: Operation) -> Self {
        let item = self.paths.entry(path.to_string()).or_default();
        match method.to_uppercase().as_str() {
            "GET" => item.get = Some(operation),
            "POST" => item.post = Some(operation),
            "PUT" => item.put = Some(operation),
            "PATCH" => item.patch = Some(operation),
            "DELETE" => item.delete = Some(operation),
            _ => {}
        }
        self
    }

    /// Add a schema definition
    pub fn schema(
        mut self,
        name: &str,
        schema: crate::schema::RefOr<crate::schema::Schema>,
    ) -> Self {
        self.schemas.insert(name.to_string(), schema);
        self
    }

    /// Register a type that implements Schema (crate::ToSchema)
    pub fn register<T: crate::ToSchema>(mut self) -> Self {
        let (name, schema) = T::schema();
        self.schemas.insert(name, schema);
        self
    }

    /// Register a type into this spec in-place.
    ///
    /// This is useful for zero-config registration paths where the spec is stored
    /// by value in another struct (e.g., the application builder).
    pub fn register_in_place<T: crate::ToSchema>(&mut self) {
        let (name, schema) = T::schema();
        self.schemas.insert(name, schema);
    }

    /// Convert to JSON value
    pub fn to_json(&self) -> serde_json::Value {
        let mut spec = serde_json::json!({
            "openapi": "3.0.3",
            "info": self.info,
            "paths": self.paths,
        });

        if !self.schemas.is_empty() {
            spec["components"] = serde_json::json!({
                "schemas": self.schemas
            });
        }

        spec
    }
}

impl Operation {
    /// Create a new operation
    pub fn new() -> Self {
        Self {
            summary: None,
            description: None,
            tags: None,
            parameters: None,
            request_body: None,
            responses: HashMap::from([(
                "200".to_string(),
                ResponseSpec {
                    description: "Successful response".to_string(),
                    content: None,
                },
            )]),
        }
    }

    /// Set summary
    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add tags
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }
}

impl Default for Operation {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for types that can modify an OpenAPI operation
///
/// This is used by extractors to automatically update the operation
/// documentation (e.g. adding request body schema, parameters, etc.)
pub trait OperationModifier {
    /// Update the operation
    fn update_operation(op: &mut Operation);
}

// Implement for Option<T>
impl<T: OperationModifier> OperationModifier for Option<T> {
    fn update_operation(op: &mut Operation) {
        T::update_operation(op);
        // If request body was added, make it optional
        if let Some(body) = &mut op.request_body {
            body.required = false;
        }
    }
}

// Implement for Result<T, E>
impl<T: OperationModifier, E> OperationModifier for std::result::Result<T, E> {
    fn update_operation(op: &mut Operation) {
        T::update_operation(op);
    }
}

// Implement for primitives (no-op)
macro_rules! impl_op_modifier_for_primitives {
    ($($ty:ty),*) => {
        $(
            impl OperationModifier for $ty {
                fn update_operation(_op: &mut Operation) {}
            }
        )*
    };
}

impl_op_modifier_for_primitives!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64, bool, String
);

// ResponseModifier trait
pub trait ResponseModifier {
    /// Update the operation with response information
    fn update_response(op: &mut Operation);
}

// Implement for () - 200 OK (empty)
impl ResponseModifier for () {
    fn update_response(op: &mut Operation) {
        let response = ResponseSpec {
            description: "Successful response".to_string(),
            ..Default::default()
        };
        op.responses.insert("200".to_string(), response);
    }
}

// Implement for String - 200 OK (text/plain)
impl ResponseModifier for String {
    fn update_response(op: &mut Operation) {
        let mut content = std::collections::HashMap::new();
        content.insert(
            "text/plain".to_string(),
            MediaType {
                schema: crate::schema::RefOr::T(crate::schema::Schema {
                    schema_type: Some(crate::schema::SchemaType::String),
                    ..Default::default()
                }),
            },
        );

        let response = ResponseSpec {
            description: "Successful response".to_string(),
            content: Some(content),
        };
        op.responses.insert("200".to_string(), response);
    }
}

// Implement for &'static str - 200 OK (text/plain)
impl ResponseModifier for &'static str {
    fn update_response(op: &mut Operation) {
        let mut content = std::collections::HashMap::new();
        content.insert(
            "text/plain".to_string(),
            MediaType {
                schema: crate::schema::RefOr::T(crate::schema::Schema {
                    schema_type: Some(crate::schema::SchemaType::String),
                    ..Default::default()
                }),
            },
        );

        let response = ResponseSpec {
            description: "Successful response".to_string(),
            content: Some(content),
        };
        op.responses.insert("200".to_string(), response);
    }
}

// Implement for Option<T> - Delegates to T
impl<T: ResponseModifier> ResponseModifier for Option<T> {
    fn update_response(op: &mut Operation) {
        T::update_response(op);
    }
}

// Implement for Result<T, E> - Delegates to T (success) and E (error)
impl<T: ResponseModifier, E: ResponseModifier> ResponseModifier for Result<T, E> {
    fn update_response(op: &mut Operation) {
        T::update_response(op);
        E::update_response(op);
    }
}

// Implement for http::Response<T> - Generic 200 OK
impl<T> ResponseModifier for http::Response<T> {
    fn update_response(op: &mut Operation) {
        op.responses.insert(
            "200".to_string(),
            ResponseSpec {
                description: "Successful response".to_string(),
                ..Default::default()
            },
        );
    }
}
