//! OpenAPI document builder.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// OpenAPI document builder.
///
/// Create an OpenAPI specification for your API.
///
/// # Example
///
/// ```rust,ignore
/// let doc = OpenApiDoc::new("My API", "1.0.0")
///     .description("A sample API")
///     .server("http://localhost:8080");
/// ```
#[derive(Debug, Clone)]
pub struct OpenApiDoc {
    info: Info,
    servers: Vec<Server>,
    paths: BTreeMap<String, PathItem>,
    components: Components,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Info {
    title: String,
    version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    contact: Option<Contact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    license: Option<License>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Contact {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct License {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Server {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct PathItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    get: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    post: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    put: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delete: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    patch: Option<Operation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Operation {
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(rename = "operationId", skip_serializing_if = "Option::is_none")]
    operation_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    parameters: Vec<Parameter>,
    #[serde(rename = "requestBody", skip_serializing_if = "Option::is_none")]
    request_body: Option<RequestBody>,
    responses: BTreeMap<String, Response>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Parameter {
    name: String,
    #[serde(rename = "in")]
    location: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    required: bool,
    schema: SchemaRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RequestBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    required: bool,
    content: BTreeMap<String, MediaType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Response {
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<BTreeMap<String, MediaType>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MediaType {
    schema: SchemaRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum SchemaRef {
    Ref { #[serde(rename = "$ref")] reference: String },
    Inline(Schema),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Schema {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    schema_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<BTreeMap<String, Schema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    items: Option<Box<Schema>>,
    #[serde(rename = "minLength", skip_serializing_if = "Option::is_none")]
    min_length: Option<u64>,
    #[serde(rename = "maxLength", skip_serializing_if = "Option::is_none")]
    max_length: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    maximum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pattern: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Components {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    schemas: BTreeMap<String, Schema>,
}

/// Serializable OpenAPI document
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenApiSpec {
    openapi: String,
    info: Info,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    servers: Vec<Server>,
    paths: BTreeMap<String, PathItem>,
    #[serde(skip_serializing_if = "is_components_empty")]
    components: Components,
}

fn is_components_empty(c: &Components) -> bool {
    c.schemas.is_empty()
}

impl OpenApiDoc {
    /// Create a new OpenAPI document with title and version.
    pub fn new(title: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            info: Info {
                title: title.into(),
                version: version.into(),
                description: None,
                contact: None,
                license: None,
            },
            servers: Vec::new(),
            paths: BTreeMap::new(),
            components: Components::default(),
        }
    }

    /// Get the API title.
    pub fn title(&self) -> &str {
        &self.info.title
    }

    /// Get the API version.
    pub fn version(&self) -> &str {
        &self.info.version
    }

    /// Set the API description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.info.description = Some(description.into());
        self
    }

    /// Add a server URL.
    pub fn server(mut self, url: impl Into<String>) -> Self {
        self.servers.push(Server {
            url: url.into(),
            description: None,
        });
        self
    }

    /// Add a server with description.
    pub fn server_with_description(
        mut self,
        url: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        self.servers.push(Server {
            url: url.into(),
            description: Some(description.into()),
        });
        self
    }

    /// Set contact information.
    pub fn contact(
        mut self,
        name: Option<String>,
        url: Option<String>,
        email: Option<String>,
    ) -> Self {
        self.info.contact = Some(Contact { name, url, email });
        self
    }

    /// Set license information.
    pub fn license(mut self, name: impl Into<String>, url: Option<String>) -> Self {
        self.info.license = Some(License {
            name: name.into(),
            url,
        });
        self
    }

    /// Convert to JSON string.
    pub fn to_json(&self) -> String {
        let spec = OpenApiSpec {
            openapi: "3.1.0".to_string(),
            info: self.info.clone(),
            servers: self.servers.clone(),
            paths: self.paths.clone(),
            components: self.components.clone(),
        };
        serde_json::to_string_pretty(&spec).unwrap_or_else(|_| "{}".to_string())
    }

    /// Convert to JSON bytes.
    pub fn to_json_bytes(&self) -> Vec<u8> {
        self.to_json().into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_basic_doc() {
        let doc = OpenApiDoc::new("Test API", "1.0.0");
        assert_eq!(doc.title(), "Test API");
        assert_eq!(doc.version(), "1.0.0");
    }

    #[test]
    fn doc_with_description() {
        let doc = OpenApiDoc::new("Test API", "1.0.0")
            .description("A test API");
        
        let json = doc.to_json();
        assert!(json.contains("A test API"));
    }

    #[test]
    fn doc_with_server() {
        let doc = OpenApiDoc::new("Test API", "1.0.0")
            .server("http://localhost:8080");
        
        let json = doc.to_json();
        assert!(json.contains("http://localhost:8080"));
    }

    #[test]
    fn doc_to_json() {
        let doc = OpenApiDoc::new("Test API", "1.0.0");
        let json = doc.to_json();
        
        assert!(json.contains("\"openapi\": \"3.1.0\""));
        assert!(json.contains("\"title\": \"Test API\""));
        assert!(json.contains("\"version\": \"1.0.0\""));
    }
}
