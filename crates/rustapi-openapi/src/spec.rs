//! OpenAPI 3.1 specification types

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::schema::JsonSchema2020;
pub use crate::schema::SchemaRef;

/// OpenAPI 3.1.0 specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiSpec {
    /// OpenAPI version (always "3.1.0")
    pub openapi: String,

    /// API information
    pub info: ApiInfo,

    /// JSON Schema dialect (optional, defaults to JSON Schema 2020-12)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_schema_dialect: Option<String>,

    /// Server list
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub servers: Vec<Server>,

    /// API paths
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub paths: BTreeMap<String, PathItem>,

    /// Webhooks
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub webhooks: BTreeMap<String, PathItem>,

    /// Components
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Components>,

    /// Security requirements
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub security: Vec<BTreeMap<String, Vec<String>>>,

    /// Tags
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<Tag>,

    /// External documentation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_docs: Option<ExternalDocs>,
}

impl OpenApiSpec {
    pub fn new(title: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            openapi: "3.1.0".to_string(),
            info: ApiInfo {
                title: title.into(),
                version: version.into(),
                ..Default::default()
            },
            json_schema_dialect: Some("https://json-schema.org/draft/2020-12/schema".to_string()),
            servers: Vec::new(),
            paths: BTreeMap::new(),
            webhooks: BTreeMap::new(),
            components: None,
            security: Vec::new(),
            tags: Vec::new(),
            external_docs: None,
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.info.description = Some(desc.into());
        self
    }

    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.info.summary = Some(summary.into());
        self
    }

    pub fn path(mut self, path: &str, method: &str, operation: Operation) -> Self {
        let item = self.paths.entry(path.to_string()).or_default();
        match method.to_uppercase().as_str() {
            "GET" => item.get = Some(operation),
            "POST" => item.post = Some(operation),
            "PUT" => item.put = Some(operation),
            "PATCH" => item.patch = Some(operation),
            "DELETE" => item.delete = Some(operation),
            "HEAD" => item.head = Some(operation),
            "OPTIONS" => item.options = Some(operation),
            "TRACE" => item.trace = Some(operation),
            _ => {}
        }
        self
    }

    /// Register a type that implements RustApiSchema
    pub fn register<T: crate::schema::RustApiSchema>(mut self) -> Self {
        self.register_in_place::<T>();
        self
    }

    /// Register a type into this spec in-place.
    pub fn register_in_place<T: crate::schema::RustApiSchema>(&mut self) {
        let mut ctx = crate::schema::SchemaCtx::new();

        // Pre-load existing schemas to avoid re-generating or to handle deduplication correctly
        if let Some(c) = &self.components {
            ctx.components = c.schemas.clone();
        }

        // Generate schema for T (and dependencies)
        let _ = T::schema(&mut ctx);

        // Merge back into components
        let components = self.components.get_or_insert_with(Components::default);
        components.schemas.extend(ctx.components);
    }

    pub fn server(mut self, server: Server) -> Self {
        self.servers.push(server);
        self
    }

    pub fn security_scheme(mut self, name: impl Into<String>, scheme: SecurityScheme) -> Self {
        let components = self.components.get_or_insert_with(Components::default);
        components
            .security_schemes
            .entry(name.into())
            .or_insert(scheme);
        self
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApiInfo {
    pub title: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terms_of_service: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<Contact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Contact {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct License {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub variables: BTreeMap<String, ServerVariable>,
}

impl Server {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            description: None,
            variables: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerVariable {
    #[serde(rename = "enum", skip_serializing_if = "Vec::is_empty")]
    pub enum_values: Vec<String>,
    pub default: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PathItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub put: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<Operation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub servers: Vec<Server>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<Parameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_docs: Option<ExternalDocs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<Parameter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_body: Option<RequestBody>,
    pub responses: BTreeMap<String, ResponseSpec>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub security: Vec<BTreeMap<String, Vec<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
}

impl Operation {
    pub fn new() -> Self {
        Self {
            responses: BTreeMap::from([("200".to_string(), ResponseSpec::default())]),
            ..Default::default()
        }
    }

    pub fn summary(mut self, s: impl Into<String>) -> Self {
        self.summary = Some(s.into());
        self
    }

    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    #[serde(rename = "in")]
    pub location: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<SchemaRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub content: BTreeMap<String, MediaType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResponseSpec {
    pub description: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub content: BTreeMap<String, MediaType>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, Header>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaType {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<SchemaRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<SchemaRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Components {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub schemas: BTreeMap<String, JsonSchema2020>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub responses: BTreeMap<String, ResponseSpec>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, Parameter>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub examples: BTreeMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub request_bodies: BTreeMap<String, RequestBody>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, Header>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub security_schemes: BTreeMap<String, SecurityScheme>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub links: BTreeMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub callbacks: BTreeMap<String, BTreeMap<String, PathItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SecurityScheme {
    ApiKey {
        name: String,
        #[serde(rename = "in")]
        location: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    Http {
        scheme: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        bearer_format: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    Oauth2 {
        flows: OAuthFlows,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    OpenIdConnect {
        open_id_connect_url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OAuthFlows {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implicit: Option<OAuthFlow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<OAuthFlow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_credentials: Option<OAuthFlow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_code: Option<OAuthFlow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthFlow {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,
    pub scopes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_docs: Option<ExternalDocs>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDocs {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// Re-exports/Traits needed for backwards compatibility or easy migration
pub trait OperationModifier {
    fn update_operation(op: &mut Operation);
}

pub trait ResponseModifier {
    fn update_response(op: &mut Operation);
}

// Helper implementations for OperationModifier/ResponseModifier
impl<T: OperationModifier> OperationModifier for Option<T> {
    fn update_operation(op: &mut Operation) {
        T::update_operation(op);
        if let Some(body) = &mut op.request_body {
            body.required = Some(false);
        }
    }
}

impl<T: OperationModifier, E> OperationModifier for Result<T, E> {
    fn update_operation(op: &mut Operation) {
        T::update_operation(op);
    }
}

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

impl ResponseModifier for () {
    fn update_response(op: &mut Operation) {
        op.responses.insert(
            "200".to_string(),
            ResponseSpec {
                description: "Successful response".into(),
                ..Default::default()
            },
        );
    }
}

impl ResponseModifier for String {
    fn update_response(op: &mut Operation) {
        let mut content = BTreeMap::new();
        content.insert(
            "text/plain".to_string(),
            MediaType {
                schema: Some(SchemaRef::Inline(serde_json::json!({"type": "string"}))),
                example: None,
            },
        );
        op.responses.insert(
            "200".to_string(),
            ResponseSpec {
                description: "Successful response".into(),
                content,
                ..Default::default()
            },
        );
    }
}

impl ResponseModifier for &'static str {
    fn update_response(op: &mut Operation) {
        String::update_response(op);
    }
}

impl<T: ResponseModifier> ResponseModifier for Option<T> {
    fn update_response(op: &mut Operation) {
        T::update_response(op);
    }
}

impl<T: ResponseModifier, E: ResponseModifier> ResponseModifier for Result<T, E> {
    fn update_response(op: &mut Operation) {
        T::update_response(op);
        E::update_response(op);
    }
}

impl<T> ResponseModifier for http::Response<T> {
    fn update_response(op: &mut Operation) {
        op.responses.insert(
            "200".to_string(),
            ResponseSpec {
                description: "Successful response".into(),
                ..Default::default()
            },
        );
    }
}
