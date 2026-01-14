//! JSON Schema 2020-12 support for OpenAPI 3.1
//!
//! OpenAPI 3.1 uses JSON Schema 2020-12 directly, which has some key differences
//! from the JSON Schema draft used in OpenAPI 3.0.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type array for nullable types in JSON Schema 2020-12
///
/// In OpenAPI 3.1/JSON Schema 2020-12, nullable types are represented as:
/// `"type": ["string", "null"]` instead of `"type": "string", "nullable": true`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum TypeArray {
    /// Single type (e.g., "string")
    Single(String),
    /// Multiple types (e.g., ["string", "null"])
    Array(Vec<String>),
}

impl TypeArray {
    /// Create a single type
    pub fn single(ty: impl Into<String>) -> Self {
        Self::Single(ty.into())
    }

    /// Create a nullable type
    pub fn nullable(ty: impl Into<String>) -> Self {
        Self::Array(vec![ty.into(), "null".to_string()])
    }

    /// Create a type array from multiple types
    pub fn array(types: Vec<String>) -> Self {
        if types.len() == 1 {
            Self::Single(types.into_iter().next().unwrap())
        } else {
            Self::Array(types)
        }
    }

    /// Check if this type is nullable
    pub fn is_nullable(&self) -> bool {
        match self {
            Self::Single(_) => false,
            Self::Array(types) => types.iter().any(|t| t == "null"),
        }
    }

    /// Add null to make this type nullable
    pub fn make_nullable(self) -> Self {
        match self {
            Self::Single(ty) => Self::Array(vec![ty, "null".to_string()]),
            Self::Array(mut types) => {
                if !types.iter().any(|t| t == "null") {
                    types.push("null".to_string());
                }
                Self::Array(types)
            }
        }
    }
}

/// JSON Schema 2020-12 schema definition
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JsonSchema2020 {
    /// Schema dialect identifier
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// Schema identifier
    #[serde(rename = "$id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Reference to another schema
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,

    /// Dynamic reference
    #[serde(rename = "$dynamicRef", skip_serializing_if = "Option::is_none")]
    pub dynamic_ref: Option<String>,

    /// Type of the schema
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub schema_type: Option<TypeArray>,

    /// Title of the schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Description of the schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Default value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,

    /// Constant value
    #[serde(rename = "const", skip_serializing_if = "Option::is_none")]
    pub const_value: Option<serde_json::Value>,

    /// Enum values
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<serde_json::Value>>,

    // String constraints
    /// Minimum length for strings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u64>,

    /// Maximum length for strings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u64>,

    /// Pattern for strings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,

    /// Format hint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    // Number constraints
    /// Minimum value (inclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,

    /// Maximum value (inclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,

    /// Exclusive minimum (JSON Schema 2020-12 uses number, not boolean)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusive_minimum: Option<f64>,

    /// Exclusive maximum (JSON Schema 2020-12 uses number, not boolean)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusive_maximum: Option<f64>,

    /// Multiple of constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiple_of: Option<f64>,

    // Array constraints
    /// Items schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<JsonSchema2020>>,

    /// Prefix items (replaces "items" array in draft-07)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix_items: Option<Vec<JsonSchema2020>>,

    /// Contains constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contains: Option<Box<JsonSchema2020>>,

    /// Minimum items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_items: Option<u64>,

    /// Maximum items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_items: Option<u64>,

    /// Unique items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_items: Option<bool>,

    // Object constraints
    /// Object properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, JsonSchema2020>>,

    /// Pattern properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_properties: Option<HashMap<String, JsonSchema2020>>,

    /// Additional properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<Box<AdditionalProperties>>,

    /// Required properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,

    /// Property names schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property_names: Option<Box<JsonSchema2020>>,

    /// Minimum properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_properties: Option<u64>,

    /// Maximum properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_properties: Option<u64>,

    // Composition
    /// All of (must match all schemas)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_of: Option<Vec<JsonSchema2020>>,

    /// Any of (must match at least one schema)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<JsonSchema2020>>,

    /// One of (must match exactly one schema)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub one_of: Option<Vec<JsonSchema2020>>,

    /// Not (must not match schema)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not: Option<Box<JsonSchema2020>>,

    // Conditionals
    /// If condition
    #[serde(rename = "if", skip_serializing_if = "Option::is_none")]
    pub if_schema: Option<Box<JsonSchema2020>>,

    /// Then schema
    #[serde(rename = "then", skip_serializing_if = "Option::is_none")]
    pub then_schema: Option<Box<JsonSchema2020>>,

    /// Else schema
    #[serde(rename = "else", skip_serializing_if = "Option::is_none")]
    pub else_schema: Option<Box<JsonSchema2020>>,

    // OpenAPI extensions
    /// Whether this property is deprecated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,

    /// Whether this property is read-only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,

    /// Whether this property is write-only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_only: Option<bool>,

    /// Example value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,

    /// Examples (JSON Schema 2020-12)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<serde_json::Value>>,

    /// OpenAPI discriminator
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discriminator: Option<Discriminator>,

    /// External documentation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_docs: Option<ExternalDocumentation>,

    /// XML metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xml: Option<Xml>,
}

/// Additional properties can be a boolean or a schema
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum AdditionalProperties {
    Bool(bool),
    Schema(Box<JsonSchema2020>),
}

/// Discriminator for polymorphism
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Discriminator {
    /// Property name for discriminator
    #[serde(rename = "propertyName")]
    pub property_name: String,

    /// Mapping of values to schema references
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mapping: Option<HashMap<String, String>>,
}

/// External documentation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExternalDocumentation {
    /// URL to external documentation
    pub url: String,

    /// Description of external documentation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// XML metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Xml {
    /// XML element name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// XML namespace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,

    /// XML prefix
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,

    /// Whether to use attribute (vs element)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribute: Option<bool>,

    /// Whether to wrap array elements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrapped: Option<bool>,
}

impl JsonSchema2020 {
    /// Create a new empty schema
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a string schema
    pub fn string() -> Self {
        Self {
            schema_type: Some(TypeArray::single("string")),
            ..Default::default()
        }
    }

    /// Create a number schema
    pub fn number() -> Self {
        Self {
            schema_type: Some(TypeArray::single("number")),
            ..Default::default()
        }
    }

    /// Create an integer schema
    pub fn integer() -> Self {
        Self {
            schema_type: Some(TypeArray::single("integer")),
            ..Default::default()
        }
    }

    /// Create a boolean schema
    pub fn boolean() -> Self {
        Self {
            schema_type: Some(TypeArray::single("boolean")),
            ..Default::default()
        }
    }

    /// Create an array schema
    pub fn array(items: JsonSchema2020) -> Self {
        Self {
            schema_type: Some(TypeArray::single("array")),
            items: Some(Box::new(items)),
            ..Default::default()
        }
    }

    /// Create an object schema
    pub fn object() -> Self {
        Self {
            schema_type: Some(TypeArray::single("object")),
            ..Default::default()
        }
    }

    /// Create a null schema
    pub fn null() -> Self {
        Self {
            schema_type: Some(TypeArray::single("null")),
            ..Default::default()
        }
    }

    /// Create a reference schema
    pub fn reference(ref_path: impl Into<String>) -> Self {
        Self {
            reference: Some(ref_path.into()),
            ..Default::default()
        }
    }

    /// Make this schema nullable
    pub fn nullable(mut self) -> Self {
        self.schema_type = self.schema_type.map(|t| t.make_nullable());
        self
    }

    /// Add a title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Add a description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a format
    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    /// Add a property to an object schema
    pub fn with_property(mut self, name: impl Into<String>, schema: JsonSchema2020) -> Self {
        let properties = self.properties.get_or_insert_with(HashMap::new);
        properties.insert(name.into(), schema);
        self
    }

    /// Add a required property
    pub fn with_required(mut self, name: impl Into<String>) -> Self {
        let required = self.required.get_or_insert_with(Vec::new);
        required.push(name.into());
        self
    }

    /// Add an example
    pub fn with_example(mut self, example: serde_json::Value) -> Self {
        self.example = Some(example);
        self
    }
}

/// Transformer for converting OpenAPI 3.0 schemas to 3.1
pub struct SchemaTransformer;

impl SchemaTransformer {
    /// Transform an OpenAPI 3.0 schema (serde_json::Value) to OpenAPI 3.1 format
    ///
    /// Key transformations:
    /// - `nullable: true` becomes `type: ["<type>", "null"]`
    /// - `exclusiveMinimum: true` with `minimum: X` becomes `exclusiveMinimum: X`
    /// - `exclusiveMaximum: true` with `maximum: X` becomes `exclusiveMaximum: X`
    pub fn transform_30_to_31(schema: serde_json::Value) -> serde_json::Value {
        match schema {
            serde_json::Value::Object(mut map) => {
                // Transform nullable
                if map.get("nullable") == Some(&serde_json::Value::Bool(true)) {
                    map.remove("nullable");
                    if let Some(serde_json::Value::String(ty)) = map.get("type") {
                        let type_array = serde_json::json!([ty.clone(), "null"]);
                        map.insert("type".to_string(), type_array);
                    }
                }

                // Transform exclusiveMinimum
                if map.get("exclusiveMinimum") == Some(&serde_json::Value::Bool(true)) {
                    if let Some(min) = map.remove("minimum") {
                        map.insert("exclusiveMinimum".to_string(), min);
                    }
                }

                // Transform exclusiveMaximum
                if map.get("exclusiveMaximum") == Some(&serde_json::Value::Bool(true)) {
                    if let Some(max) = map.remove("maximum") {
                        map.insert("exclusiveMaximum".to_string(), max);
                    }
                }

                // Recursively transform nested schemas
                for key in [
                    "items",
                    "additionalProperties",
                    "not",
                    "if",
                    "then",
                    "else",
                    "contains",
                    "propertyNames",
                ] {
                    if let Some(nested) = map.remove(key) {
                        map.insert(key.to_string(), Self::transform_30_to_31(nested));
                    }
                }

                // Transform arrays
                for key in ["allOf", "anyOf", "oneOf", "prefixItems"] {
                    if let Some(serde_json::Value::Array(arr)) = map.remove(key) {
                        let transformed: Vec<_> =
                            arr.into_iter().map(Self::transform_30_to_31).collect();
                        map.insert(key.to_string(), serde_json::Value::Array(transformed));
                    }
                }

                // Transform properties
                if let Some(serde_json::Value::Object(props)) = map.remove("properties") {
                    let transformed: serde_json::Map<String, serde_json::Value> = props
                        .into_iter()
                        .map(|(k, v)| (k, Self::transform_30_to_31(v)))
                        .collect();
                    map.insert(
                        "properties".to_string(),
                        serde_json::Value::Object(transformed),
                    );
                }

                // Transform patternProperties
                if let Some(serde_json::Value::Object(props)) = map.remove("patternProperties") {
                    let transformed: serde_json::Map<String, serde_json::Value> = props
                        .into_iter()
                        .map(|(k, v)| (k, Self::transform_30_to_31(v)))
                        .collect();
                    map.insert(
                        "patternProperties".to_string(),
                        serde_json::Value::Object(transformed),
                    );
                }

                serde_json::Value::Object(map)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(Self::transform_30_to_31).collect())
            }
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_array_single() {
        let ty = TypeArray::single("string");
        assert!(!ty.is_nullable());
        assert_eq!(serde_json::to_string(&ty).unwrap(), r#""string""#);
    }

    #[test]
    fn test_type_array_nullable() {
        let ty = TypeArray::nullable("string");
        assert!(ty.is_nullable());
        assert_eq!(serde_json::to_string(&ty).unwrap(), r#"["string","null"]"#);
    }

    #[test]
    fn test_make_nullable() {
        let ty = TypeArray::single("integer").make_nullable();
        assert!(ty.is_nullable());

        // Making nullable again should not add duplicate null
        let ty2 = ty.make_nullable();
        if let TypeArray::Array(types) = ty2 {
            assert_eq!(types.iter().filter(|t| *t == "null").count(), 1);
        }
    }

    #[test]
    fn test_schema_transformer_nullable() {
        let schema30 = serde_json::json!({
            "type": "string",
            "nullable": true
        });

        let schema31 = SchemaTransformer::transform_30_to_31(schema30);

        assert_eq!(
            schema31,
            serde_json::json!({
                "type": ["string", "null"]
            })
        );
    }

    #[test]
    fn test_schema_transformer_exclusive_minimum() {
        let schema30 = serde_json::json!({
            "type": "integer",
            "minimum": 0,
            "exclusiveMinimum": true
        });

        let schema31 = SchemaTransformer::transform_30_to_31(schema30);

        assert_eq!(
            schema31,
            serde_json::json!({
                "type": "integer",
                "exclusiveMinimum": 0
            })
        );
    }

    #[test]
    fn test_json_schema_2020_builder() {
        let schema = JsonSchema2020::object()
            .with_property("name", JsonSchema2020::string())
            .with_property("age", JsonSchema2020::integer().nullable())
            .with_required("name");

        assert!(schema.properties.is_some());
        assert_eq!(schema.required, Some(vec!["name".to_string()]));
    }
}
