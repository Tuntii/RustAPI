//! OpenAPI 3.1.0 specification types
//!
//! This module provides the complete OpenAPI 3.1.0 specification builder
//! with JSON Schema 2020-12 support.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::schema::{JsonSchema2020, SchemaTransformer};
use super::webhooks::{Callback, Webhook};
use crate::{Operation, PathItem};

/// OpenAPI 3.1.0 specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApi31Spec {
    /// OpenAPI version (always "3.1.0")
    pub openapi: String,

    /// API information
    pub info: ApiInfo31,

    /// JSON Schema dialect (optional, defaults to JSON Schema 2020-12)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_schema_dialect: Option<String>,

    /// Server list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub servers: Option<Vec<Server>>,

    /// API paths
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub paths: HashMap<String, PathItem>,

    /// Webhooks (new in 3.1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhooks: Option<HashMap<String, Webhook>>,

    /// Components (schemas, security schemes, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Components31>,

    /// Security requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security: Option<Vec<HashMap<String, Vec<String>>>>,

    /// Tags for organization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<Tag>>,

    /// External documentation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_docs: Option<ExternalDocs>,
}

impl OpenApi31Spec {
    /// Create a new OpenAPI 3.1.0 specification
    pub fn new(title: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            openapi: "3.1.0".to_string(),
            info: ApiInfo31 {
                title: title.into(),
                version: version.into(),
                summary: None,
                description: None,
                terms_of_service: None,
                contact: None,
                license: None,
            },
            json_schema_dialect: Some("https://json-schema.org/draft/2020-12/schema".to_string()),
            servers: None,
            paths: HashMap::new(),
            webhooks: None,
            components: None,
            security: None,
            tags: None,
            external_docs: None,
        }
    }

    /// Set API description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.info.description = Some(desc.into());
        self
    }

    /// Set API summary (new in 3.1)
    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.info.summary = Some(summary.into());
        self
    }

    /// Set terms of service URL
    pub fn terms_of_service(mut self, url: impl Into<String>) -> Self {
        self.info.terms_of_service = Some(url.into());
        self
    }

    /// Set contact information
    pub fn contact(mut self, contact: Contact) -> Self {
        self.info.contact = Some(contact);
        self
    }

    /// Set license information
    pub fn license(mut self, license: License) -> Self {
        self.info.license = Some(license);
        self
    }

    /// Add a server
    pub fn server(mut self, server: Server) -> Self {
        self.servers.get_or_insert_with(Vec::new).push(server);
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

    /// Add a webhook (new in 3.1)
    pub fn webhook(mut self, name: impl Into<String>, webhook: Webhook) -> Self {
        self.webhooks
            .get_or_insert_with(HashMap::new)
            .insert(name.into(), webhook);
        self
    }

    /// Add a schema to components
    pub fn schema(mut self, name: impl Into<String>, schema: JsonSchema2020) -> Self {
        let components = self.components.get_or_insert_with(Components31::default);
        components
            .schemas
            .get_or_insert_with(HashMap::new)
            .insert(name.into(), schema);
        self
    }

    /// Add a schema from an existing OpenAPI 3.0 schema (will be transformed)
    pub fn schema_from_30(mut self, name: impl Into<String>, schema: serde_json::Value) -> Self {
        let transformed = SchemaTransformer::transform_30_to_31(schema);
        if let Ok(schema31) = serde_json::from_value::<JsonSchema2020>(transformed) {
            let components = self.components.get_or_insert_with(Components31::default);
            components
                .schemas
                .get_or_insert_with(HashMap::new)
                .insert(name.into(), schema31);
        }
        self
    }

    /// Register a type that implements crate::ToSchema
    ///
    /// The schema will be automatically transformed to OpenAPI 3.1 format
    pub fn register<T: crate::ToSchema>(mut self) -> Self {
        let (name, schema_struct) = T::schema();
        // Convert to JSON Value for transformation
        if let Ok(json_schema) = serde_json::to_value(schema_struct) {
            let transformed = SchemaTransformer::transform_30_to_31(json_schema);
            if let Ok(schema31) = serde_json::from_value::<JsonSchema2020>(transformed) {
                let components = self.components.get_or_insert_with(Components31::default);
                components
                    .schemas
                    .get_or_insert_with(HashMap::new)
                    .insert(name, schema31);
            }
        }
        self
    }

    /// Add a security scheme
    pub fn security_scheme(mut self, name: impl Into<String>, scheme: SecurityScheme) -> Self {
        let components = self.components.get_or_insert_with(Components31::default);
        components
            .security_schemes
            .get_or_insert_with(HashMap::new)
            .insert(name.into(), scheme);
        self
    }

    /// Add a global security requirement
    pub fn security_requirement(mut self, name: impl Into<String>, scopes: Vec<String>) -> Self {
        let mut req = HashMap::new();
        req.insert(name.into(), scopes);
        self.security.get_or_insert_with(Vec::new).push(req);
        self
    }

    /// Add a tag
    pub fn tag(mut self, tag: Tag) -> Self {
        self.tags.get_or_insert_with(Vec::new).push(tag);
        self
    }

    /// Set external documentation
    pub fn external_docs(mut self, docs: ExternalDocs) -> Self {
        self.external_docs = Some(docs);
        self
    }

    /// Add a callback to components
    pub fn callback(mut self, name: impl Into<String>, callback: Callback) -> Self {
        let components = self.components.get_or_insert_with(Components31::default);
        components
            .callbacks
            .get_or_insert_with(HashMap::new)
            .insert(name.into(), callback);
        self
    }

    /// Convert to JSON value
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({}))
    }

    /// Convert to pretty-printed JSON string
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// API information for OpenAPI 3.1
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiInfo31 {
    /// API title
    pub title: String,

    /// API version
    pub version: String,

    /// Short summary (new in 3.1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// Full description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Terms of service URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terms_of_service: Option<String>,

    /// Contact information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<Contact>,

    /// License information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,
}

/// Contact information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    /// Contact name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Contact URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Contact email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

impl Contact {
    /// Create new contact
    pub fn new() -> Self {
        Self {
            name: None,
            url: None,
            email: None,
        }
    }

    /// Set name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set URL
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Set email
    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }
}

impl Default for Contact {
    fn default() -> Self {
        Self::new()
    }
}

/// License information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    /// License name
    pub name: String,

    /// License identifier (SPDX, new in 3.1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,

    /// License URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl License {
    /// Create a new license
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            identifier: None,
            url: None,
        }
    }

    /// Create a license with SPDX identifier (new in 3.1)
    pub fn spdx(name: impl Into<String>, identifier: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            identifier: Some(identifier.into()),
            url: None,
        }
    }

    /// Set URL
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }
}

/// Server definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    /// Server URL
    pub url: String,

    /// Server description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Server variables
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<HashMap<String, ServerVariable>>,
}

impl Server {
    /// Create a new server
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            description: None,
            variables: None,
        }
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add a variable
    pub fn variable(mut self, name: impl Into<String>, var: ServerVariable) -> Self {
        self.variables
            .get_or_insert_with(HashMap::new)
            .insert(name.into(), var);
        self
    }
}

/// Server variable
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerVariable {
    /// Possible values
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,

    /// Default value
    pub default: String,

    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl ServerVariable {
    /// Create a new variable with default value
    pub fn new(default: impl Into<String>) -> Self {
        Self {
            enum_values: None,
            default: default.into(),
            description: None,
        }
    }

    /// Set allowed values
    pub fn enum_values(mut self, values: Vec<String>) -> Self {
        self.enum_values = Some(values);
        self
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Components container (schemas, security schemes, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Components31 {
    /// Schema definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schemas: Option<HashMap<String, JsonSchema2020>>,

    /// Response definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub responses: Option<HashMap<String, serde_json::Value>>,

    /// Parameter definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<HashMap<String, serde_json::Value>>,

    /// Example definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<HashMap<String, serde_json::Value>>,

    /// Request body definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_bodies: Option<HashMap<String, serde_json::Value>>,

    /// Header definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, serde_json::Value>>,

    /// Security scheme definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_schemes: Option<HashMap<String, SecurityScheme>>,

    /// Link definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<HashMap<String, serde_json::Value>>,

    /// Callback definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callbacks: Option<HashMap<String, Callback>>,

    /// Path item definitions (new in 3.1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_items: Option<HashMap<String, PathItem>>,
}

/// Security scheme definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityScheme {
    /// Type of security scheme
    #[serde(rename = "type")]
    pub scheme_type: String,

    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Header/query parameter name (for apiKey)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Location (header, query, cookie) for apiKey
    #[serde(rename = "in", skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,

    /// Scheme name (for http)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,

    /// Bearer format (for http bearer)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bearer_format: Option<String>,

    /// OAuth flows
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flows: Option<OAuthFlows>,

    /// OpenID Connect URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_id_connect_url: Option<String>,
}

impl SecurityScheme {
    /// Create an API key security scheme
    pub fn api_key(name: impl Into<String>, location: impl Into<String>) -> Self {
        Self {
            scheme_type: "apiKey".to_string(),
            description: None,
            name: Some(name.into()),
            location: Some(location.into()),
            scheme: None,
            bearer_format: None,
            flows: None,
            open_id_connect_url: None,
        }
    }

    /// Create a bearer token security scheme
    pub fn bearer(format: impl Into<String>) -> Self {
        Self {
            scheme_type: "http".to_string(),
            description: None,
            name: None,
            location: None,
            scheme: Some("bearer".to_string()),
            bearer_format: Some(format.into()),
            flows: None,
            open_id_connect_url: None,
        }
    }

    /// Create a basic auth security scheme
    pub fn basic() -> Self {
        Self {
            scheme_type: "http".to_string(),
            description: None,
            name: None,
            location: None,
            scheme: Some("basic".to_string()),
            bearer_format: None,
            flows: None,
            open_id_connect_url: None,
        }
    }

    /// Create an OAuth2 security scheme
    pub fn oauth2(flows: OAuthFlows) -> Self {
        Self {
            scheme_type: "oauth2".to_string(),
            description: None,
            name: None,
            location: None,
            scheme: None,
            bearer_format: None,
            flows: Some(flows),
            open_id_connect_url: None,
        }
    }

    /// Create an OpenID Connect security scheme
    pub fn openid_connect(url: impl Into<String>) -> Self {
        Self {
            scheme_type: "openIdConnect".to_string(),
            description: None,
            name: None,
            location: None,
            scheme: None,
            bearer_format: None,
            flows: None,
            open_id_connect_url: Some(url.into()),
        }
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// OAuth2 flows
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OAuthFlows {
    /// Implicit flow
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implicit: Option<OAuthFlow>,

    /// Password flow
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<OAuthFlow>,

    /// Client credentials flow
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_credentials: Option<OAuthFlow>,

    /// Authorization code flow
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_code: Option<OAuthFlow>,
}

/// OAuth2 flow
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthFlow {
    /// Authorization URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_url: Option<String>,

    /// Token URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_url: Option<String>,

    /// Refresh URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,

    /// Available scopes
    pub scopes: HashMap<String, String>,
}

/// Tag for grouping operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    /// Tag name
    pub name: String,

    /// Tag description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// External documentation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_docs: Option<ExternalDocs>,
}

impl Tag {
    /// Create a new tag
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            external_docs: None,
        }
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set external documentation
    pub fn external_docs(mut self, docs: ExternalDocs) -> Self {
        self.external_docs = Some(docs);
        self
    }
}

/// External documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDocs {
    /// URL to external documentation
    pub url: String,

    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl ExternalDocs {
    /// Create new external documentation
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            description: None,
        }
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v31::Webhook;

    #[test]
    fn test_openapi31_spec_creation() {
        let spec = OpenApi31Spec::new("Test API", "1.0.0")
            .description("A test API")
            .summary("Test API Summary");

        assert_eq!(spec.openapi, "3.1.0");
        assert_eq!(spec.info.title, "Test API");
        assert_eq!(spec.info.version, "1.0.0");
        assert_eq!(spec.info.summary, Some("Test API Summary".to_string()));
        assert_eq!(
            spec.json_schema_dialect,
            Some("https://json-schema.org/draft/2020-12/schema".to_string())
        );
    }

    #[test]
    fn test_license_spdx() {
        let license = License::spdx("MIT License", "MIT");
        assert_eq!(license.name, "MIT License");
        assert_eq!(license.identifier, Some("MIT".to_string()));
    }

    #[test]
    fn test_webhook_addition() {
        let spec = OpenApi31Spec::new("Test API", "1.0.0").webhook(
            "orderPlaced",
            Webhook::with_summary("Order placed notification"),
        );

        assert!(spec.webhooks.is_some());
        assert!(spec.webhooks.as_ref().unwrap().contains_key("orderPlaced"));
    }

    #[test]
    fn test_security_scheme_bearer() {
        let scheme = SecurityScheme::bearer("JWT").description("JWT Bearer token authentication");

        assert_eq!(scheme.scheme_type, "http");
        assert_eq!(scheme.scheme, Some("bearer".to_string()));
        assert_eq!(scheme.bearer_format, Some("JWT".to_string()));
    }

    #[test]
    fn test_server_with_variables() {
        let server = Server::new("https://{environment}.api.example.com")
            .description("Server with environment variable")
            .variable(
                "environment",
                ServerVariable::new("production")
                    .enum_values(vec![
                        "development".to_string(),
                        "staging".to_string(),
                        "production".to_string(),
                    ])
                    .description("Server environment"),
            );

        assert!(server.variables.is_some());
        assert!(server
            .variables
            .as_ref()
            .unwrap()
            .contains_key("environment"));
    }

    #[test]
    fn test_spec_to_json() {
        let spec =
            OpenApi31Spec::new("Test API", "1.0.0").server(Server::new("https://api.example.com"));

        let json = spec.to_json();
        assert_eq!(json["openapi"], "3.1.0");
        assert_eq!(json["info"]["title"], "Test API");
    }
}
