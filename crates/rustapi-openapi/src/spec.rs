//! OpenAPI 3.1 specification types

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

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
        for (name, schema) in ctx.components {
            if let Some(existing) = components.schemas.get(&name) {
                if existing != &schema {
                    panic!("Schema collision detected for component '{}'. Existing schema differs from new schema. This usually means two different types are mapped to the same component name. Please implement `RustApiSchema::name()` or alias the type.", name);
                }
            } else {
                components.schemas.insert(name, schema);
            }
        }
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

    /// Validate that all $ref references point to existing components.
    /// Returns Ok(()) if valid, or a list of missing references.
    pub fn validate_integrity(&self) -> Result<(), Vec<String>> {
        let mut defined_schemas = HashSet::new();
        if let Some(components) = &self.components {
            for key in components.schemas.keys() {
                defined_schemas.insert(format!("#/components/schemas/{}", key));
            }
        }

        let mut missing_refs = Vec::new();

        // Helper to check a single ref
        let mut check_ref = |r: &str| {
            if r.starts_with("#/components/schemas/") && !defined_schemas.contains(r) {
                missing_refs.push(r.to_string());
            }
            // Ignore other refs for now (e.g. external or non-schema refs)
        };

        // 1. Visit Paths
        for path_item in self.paths.values() {
            visit_path_item(path_item, &mut |s| visit_schema_ref(s, &mut check_ref));
        }

        // 2. Visit Webhooks
        for path_item in self.webhooks.values() {
            visit_path_item(path_item, &mut |s| visit_schema_ref(s, &mut check_ref));
        }

        // 3. Visit Components
        if let Some(components) = &self.components {
            for schema in components.schemas.values() {
                visit_json_schema(schema, &mut check_ref);
            }
            for resp in components.responses.values() {
                visit_response(resp, &mut |s| visit_schema_ref(s, &mut check_ref));
            }
            for param in components.parameters.values() {
                visit_parameter(param, &mut |s| visit_schema_ref(s, &mut check_ref));
            }
            for body in components.request_bodies.values() {
                visit_request_body(body, &mut |s| visit_schema_ref(s, &mut check_ref));
            }
            for header in components.headers.values() {
                visit_header(header, &mut |s| visit_schema_ref(s, &mut check_ref));
            }
            for callback_map in components.callbacks.values() {
                for item in callback_map.values() {
                    visit_path_item(item, &mut |s| visit_schema_ref(s, &mut check_ref));
                }
            }
        }

        if missing_refs.is_empty() {
            Ok(())
        } else {
            // Deduplicate
            missing_refs.sort();
            missing_refs.dedup();
            Err(missing_refs)
        }
    }
}

fn visit_path_item<F>(item: &PathItem, visit: &mut F)
where
    F: FnMut(&SchemaRef),
{
    if let Some(op) = &item.get {
        visit_operation(op, visit);
    }
    if let Some(op) = &item.put {
        visit_operation(op, visit);
    }
    if let Some(op) = &item.post {
        visit_operation(op, visit);
    }
    if let Some(op) = &item.delete {
        visit_operation(op, visit);
    }
    if let Some(op) = &item.options {
        visit_operation(op, visit);
    }
    if let Some(op) = &item.head {
        visit_operation(op, visit);
    }
    if let Some(op) = &item.patch {
        visit_operation(op, visit);
    }
    if let Some(op) = &item.trace {
        visit_operation(op, visit);
    }

    for param in &item.parameters {
        visit_parameter(param, visit);
    }
}

fn visit_operation<F>(op: &Operation, visit: &mut F)
where
    F: FnMut(&SchemaRef),
{
    for param in &op.parameters {
        visit_parameter(param, visit);
    }
    if let Some(body) = &op.request_body {
        visit_request_body(body, visit);
    }
    for resp in op.responses.values() {
        visit_response(resp, visit);
    }
}

fn visit_parameter<F>(param: &Parameter, visit: &mut F)
where
    F: FnMut(&SchemaRef),
{
    if let Some(s) = &param.schema {
        visit(s);
    }
}

fn visit_response<F>(resp: &ResponseSpec, visit: &mut F)
where
    F: FnMut(&SchemaRef),
{
    for media in resp.content.values() {
        visit_media_type(media, visit);
    }
    for header in resp.headers.values() {
        visit_header(header, visit);
    }
}

fn visit_request_body<F>(body: &RequestBody, visit: &mut F)
where
    F: FnMut(&SchemaRef),
{
    for media in body.content.values() {
        visit_media_type(media, visit);
    }
}

fn visit_header<F>(header: &Header, visit: &mut F)
where
    F: FnMut(&SchemaRef),
{
    if let Some(s) = &header.schema {
        visit(s);
    }
}

fn visit_media_type<F>(media: &MediaType, visit: &mut F)
where
    F: FnMut(&SchemaRef),
{
    if let Some(s) = &media.schema {
        visit(s);
    }
}

fn visit_schema_ref<F>(s: &SchemaRef, check: &mut F)
where
    F: FnMut(&str),
{
    match s {
        SchemaRef::Ref { reference } => check(reference),
        SchemaRef::Schema(boxed) => visit_json_schema(boxed, check),
        SchemaRef::Inline(_) => {} // Inline JSON value, assume safe or valid
    }
}

fn visit_json_schema<F>(s: &JsonSchema2020, check: &mut F)
where
    F: FnMut(&str),
{
    if let Some(r) = &s.reference {
        check(r);
    }
    if let Some(items) = &s.items {
        visit_json_schema(items, check);
    }
    if let Some(props) = &s.properties {
        for p in props.values() {
            visit_json_schema(p, check);
        }
    }
    if let Some(crate::schema::AdditionalProperties::Schema(p)) =
        &s.additional_properties.as_deref()
    {
        visit_json_schema(p, check);
    }
    if let Some(one_of) = &s.one_of {
        for p in one_of {
            visit_json_schema(p, check);
        }
    }
    if let Some(any_of) = &s.any_of {
        for p in any_of {
            visit_json_schema(p, check);
        }
    }
    if let Some(all_of) = &s.all_of {
        for p in all_of {
            visit_json_schema(p, check);
        }
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
        flows: Box<OAuthFlows>,
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
