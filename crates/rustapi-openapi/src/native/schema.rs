//! Native schema building utilities
//!
//! This module provides builders and types for constructing OpenAPI schemas
//! programmatically without external dependencies.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashMap;

/// Schema type enumeration following OpenAPI/JSON Schema specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SchemaType {
    /// String type
    String,
    /// Number type (floating point)
    Number,
    /// Integer type
    Integer,
    /// Boolean type
    Boolean,
    /// Array type
    Array,
    /// Object type
    Object,
    /// Null type
    Null,
}

impl SchemaType {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            SchemaType::String => "string",
            SchemaType::Number => "number",
            SchemaType::Integer => "integer",
            SchemaType::Boolean => "boolean",
            SchemaType::Array => "array",
            SchemaType::Object => "object",
            SchemaType::Null => "null",
        }
    }
}

/// Common schema formats
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaFormat {
    /// 32-bit integer
    Int32,
    /// 64-bit integer
    Int64,
    /// 32-bit floating point
    Float,
    /// 64-bit floating point
    Double,
    /// Date (RFC 3339)
    Date,
    /// Date-time (RFC 3339)
    DateTime,
    /// Duration (ISO 8601)
    Duration,
    /// Email address
    Email,
    /// URI
    Uri,
    /// UUID
    Uuid,
    /// Hostname
    Hostname,
    /// IPv4 address
    Ipv4,
    /// IPv6 address
    Ipv6,
    /// Password (hint for UI)
    Password,
    /// Binary data (base64)
    Binary,
    /// Byte data (base64)
    Byte,
    /// Custom format
    Custom(Cow<'static, str>),
}

impl SchemaFormat {
    /// Convert to string representation
    pub fn as_str(&self) -> &str {
        match self {
            SchemaFormat::Int32 => "int32",
            SchemaFormat::Int64 => "int64",
            SchemaFormat::Float => "float",
            SchemaFormat::Double => "double",
            SchemaFormat::Date => "date",
            SchemaFormat::DateTime => "date-time",
            SchemaFormat::Duration => "duration",
            SchemaFormat::Email => "email",
            SchemaFormat::Uri => "uri",
            SchemaFormat::Uuid => "uuid",
            SchemaFormat::Hostname => "hostname",
            SchemaFormat::Ipv4 => "ipv4",
            SchemaFormat::Ipv6 => "ipv6",
            SchemaFormat::Password => "password",
            SchemaFormat::Binary => "binary",
            SchemaFormat::Byte => "byte",
            SchemaFormat::Custom(s) => s.as_ref(),
        }
    }
}

/// Information about a property in an object schema
#[derive(Debug, Clone)]
pub struct PropertyInfo {
    /// Property name
    pub name: Cow<'static, str>,
    /// Property schema
    pub schema: Value,
    /// Whether the property is required
    pub required: bool,
    /// Property description
    pub description: Option<Cow<'static, str>>,
    /// Default value
    pub default: Option<Value>,
    /// Whether the property is deprecated
    pub deprecated: bool,
    /// Whether the property is nullable
    pub nullable: bool,
    /// Whether the property is read-only
    pub read_only: bool,
    /// Whether the property is write-only
    pub write_only: bool,
}

impl PropertyInfo {
    /// Create a new property
    pub fn new(name: impl Into<Cow<'static, str>>, schema: Value) -> Self {
        Self {
            name: name.into(),
            schema,
            required: false,
            description: None,
            default: None,
            deprecated: false,
            nullable: false,
            read_only: false,
            write_only: false,
        }
    }

    /// Mark as required
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Add description
    pub fn with_description(mut self, desc: impl Into<Cow<'static, str>>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add default value
    pub fn with_default(mut self, default: Value) -> Self {
        self.default = Some(default);
        self
    }

    /// Mark as deprecated
    pub fn deprecated(mut self) -> Self {
        self.deprecated = true;
        self
    }

    /// Mark as nullable
    pub fn nullable(mut self) -> Self {
        self.nullable = true;
        self
    }

    /// Mark as read-only
    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    /// Mark as write-only
    pub fn write_only(mut self) -> Self {
        self.write_only = true;
        self
    }
}

/// Native schema representation
///
/// This type provides a fluent API for building OpenAPI schemas.
#[derive(Debug, Clone, Default)]
pub struct NativeSchema {
    /// Schema type
    pub schema_type: Option<SchemaType>,
    /// Schema format
    pub format: Option<SchemaFormat>,
    /// Title
    pub title: Option<Cow<'static, str>>,
    /// Description
    pub description: Option<Cow<'static, str>>,
    /// Default value
    pub default: Option<Value>,
    /// Enum values
    pub enum_values: Option<Vec<Value>>,
    /// Const value
    pub const_value: Option<Value>,
    /// Nullable flag
    pub nullable: bool,
    /// Deprecated flag
    pub deprecated: bool,
    /// Read-only flag
    pub read_only: bool,
    /// Write-only flag
    pub write_only: bool,
    /// Example value
    pub example: Option<Value>,
    /// External documentation URL
    pub external_docs_url: Option<Cow<'static, str>>,

    // String constraints
    /// Minimum length for strings
    pub min_length: Option<u64>,
    /// Maximum length for strings
    pub max_length: Option<u64>,
    /// Pattern (regex) for strings
    pub pattern: Option<Cow<'static, str>>,

    // Number constraints
    /// Minimum value
    pub minimum: Option<f64>,
    /// Maximum value
    pub maximum: Option<f64>,
    /// Exclusive minimum
    pub exclusive_minimum: Option<f64>,
    /// Exclusive maximum
    pub exclusive_maximum: Option<f64>,
    /// Multiple of constraint
    pub multiple_of: Option<f64>,

    // Array constraints
    /// Items schema for arrays
    pub items: Option<Box<NativeSchema>>,
    /// Minimum items
    pub min_items: Option<u64>,
    /// Maximum items
    pub max_items: Option<u64>,
    /// Unique items flag
    pub unique_items: bool,

    // Object constraints
    /// Properties for objects
    pub properties: Option<HashMap<String, NativeSchema>>,
    /// Required property names
    pub required: Option<Vec<String>>,
    /// Additional properties schema
    pub additional_properties: Option<Box<NativeSchema>>,
    /// Allow additional properties (boolean)
    pub additional_properties_bool: Option<bool>,
    /// Minimum properties
    pub min_properties: Option<u64>,
    /// Maximum properties
    pub max_properties: Option<u64>,

    // Composition
    /// allOf schemas
    pub all_of: Option<Vec<NativeSchema>>,
    /// anyOf schemas
    pub any_of: Option<Vec<NativeSchema>>,
    /// oneOf schemas
    pub one_of: Option<Vec<NativeSchema>>,
    /// not schema
    pub not: Option<Box<NativeSchema>>,

    // Reference
    /// Reference to another schema
    pub reference: Option<Cow<'static, str>>,

    // Discriminator
    /// Discriminator property name (for polymorphism)
    pub discriminator_property: Option<Cow<'static, str>>,
    /// Discriminator mapping
    pub discriminator_mapping: Option<HashMap<String, String>>,
}

impl NativeSchema {
    /// Create a new empty schema
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a string schema
    pub fn string() -> Self {
        Self {
            schema_type: Some(SchemaType::String),
            ..Default::default()
        }
    }

    /// Create a number schema
    pub fn number() -> Self {
        Self {
            schema_type: Some(SchemaType::Number),
            ..Default::default()
        }
    }

    /// Create an integer schema
    pub fn integer() -> Self {
        Self {
            schema_type: Some(SchemaType::Integer),
            ..Default::default()
        }
    }

    /// Create a boolean schema
    pub fn boolean() -> Self {
        Self {
            schema_type: Some(SchemaType::Boolean),
            ..Default::default()
        }
    }

    /// Create an array schema
    pub fn array(items: NativeSchema) -> Self {
        Self {
            schema_type: Some(SchemaType::Array),
            items: Some(Box::new(items)),
            ..Default::default()
        }
    }

    /// Create an object schema
    pub fn object() -> Self {
        Self {
            schema_type: Some(SchemaType::Object),
            ..Default::default()
        }
    }

    /// Create a null schema
    pub fn null() -> Self {
        Self {
            schema_type: Some(SchemaType::Null),
            ..Default::default()
        }
    }

    /// Create a reference schema
    pub fn reference(ref_path: impl Into<Cow<'static, str>>) -> Self {
        Self {
            reference: Some(ref_path.into()),
            ..Default::default()
        }
    }

    /// Create a reference to a component schema
    pub fn ref_to(schema_name: impl AsRef<str>) -> Self {
        Self::reference(format!("#/components/schemas/{}", schema_name.as_ref()))
    }

    /// Set the format
    pub fn with_format(mut self, format: SchemaFormat) -> Self {
        self.format = Some(format);
        self
    }

    /// Set the title
    pub fn with_title(mut self, title: impl Into<Cow<'static, str>>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the description
    pub fn with_description(mut self, desc: impl Into<Cow<'static, str>>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set the default value
    pub fn with_default(mut self, default: Value) -> Self {
        self.default = Some(default);
        self
    }

    /// Set enum values
    pub fn with_enum(mut self, values: Vec<Value>) -> Self {
        self.enum_values = Some(values);
        self
    }

    /// Set const value
    pub fn with_const(mut self, value: Value) -> Self {
        self.const_value = Some(value);
        self
    }

    /// Make nullable
    pub fn nullable(mut self) -> Self {
        self.nullable = true;
        self
    }

    /// Mark as deprecated
    pub fn deprecated(mut self) -> Self {
        self.deprecated = true;
        self
    }

    /// Mark as read-only
    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    /// Mark as write-only
    pub fn write_only(mut self) -> Self {
        self.write_only = true;
        self
    }

    /// Set example value
    pub fn with_example(mut self, example: Value) -> Self {
        self.example = Some(example);
        self
    }

    /// Set minimum length (for strings)
    pub fn min_length(mut self, len: u64) -> Self {
        self.min_length = Some(len);
        self
    }

    /// Set maximum length (for strings)
    pub fn max_length(mut self, len: u64) -> Self {
        self.max_length = Some(len);
        self
    }

    /// Set pattern (for strings)
    pub fn with_pattern(mut self, pattern: impl Into<Cow<'static, str>>) -> Self {
        self.pattern = Some(pattern.into());
        self
    }

    /// Set minimum value (for numbers)
    pub fn minimum(mut self, min: f64) -> Self {
        self.minimum = Some(min);
        self
    }

    /// Set maximum value (for numbers)
    pub fn maximum(mut self, max: f64) -> Self {
        self.maximum = Some(max);
        self
    }

    /// Set exclusive minimum (for numbers)
    pub fn exclusive_minimum(mut self, min: f64) -> Self {
        self.exclusive_minimum = Some(min);
        self
    }

    /// Set exclusive maximum (for numbers)
    pub fn exclusive_maximum(mut self, max: f64) -> Self {
        self.exclusive_maximum = Some(max);
        self
    }

    /// Set multiple of constraint (for numbers)
    pub fn multiple_of(mut self, val: f64) -> Self {
        self.multiple_of = Some(val);
        self
    }

    /// Set minimum items (for arrays)
    pub fn min_items(mut self, min: u64) -> Self {
        self.min_items = Some(min);
        self
    }

    /// Set maximum items (for arrays)
    pub fn max_items(mut self, max: u64) -> Self {
        self.max_items = Some(max);
        self
    }

    /// Set unique items (for arrays)
    pub fn unique_items(mut self) -> Self {
        self.unique_items = true;
        self
    }

    /// Add a property (for objects)
    pub fn with_property(mut self, name: impl Into<String>, schema: NativeSchema) -> Self {
        let properties = self.properties.get_or_insert_with(HashMap::new);
        properties.insert(name.into(), schema);
        self
    }

    /// Mark a property as required (for objects)
    pub fn with_required(mut self, name: impl Into<String>) -> Self {
        let required = self.required.get_or_insert_with(Vec::new);
        required.push(name.into());
        self
    }

    /// Set additional properties schema (for objects)
    pub fn additional_properties(mut self, schema: NativeSchema) -> Self {
        self.additional_properties = Some(Box::new(schema));
        self
    }

    /// Disallow additional properties (for objects)
    pub fn no_additional_properties(mut self) -> Self {
        self.additional_properties_bool = Some(false);
        self
    }

    /// Set allOf composition
    pub fn all_of(mut self, schemas: Vec<NativeSchema>) -> Self {
        self.all_of = Some(schemas);
        self
    }

    /// Set anyOf composition
    pub fn any_of(mut self, schemas: Vec<NativeSchema>) -> Self {
        self.any_of = Some(schemas);
        self
    }

    /// Set oneOf composition
    pub fn one_of(mut self, schemas: Vec<NativeSchema>) -> Self {
        self.one_of = Some(schemas);
        self
    }

    /// Set not composition
    pub fn not(mut self, schema: NativeSchema) -> Self {
        self.not = Some(Box::new(schema));
        self
    }

    /// Set discriminator for polymorphism
    pub fn discriminator(
        mut self,
        property_name: impl Into<Cow<'static, str>>,
        mapping: Option<HashMap<String, String>>,
    ) -> Self {
        self.discriminator_property = Some(property_name.into());
        self.discriminator_mapping = mapping;
        self
    }

    /// Convert to JSON value
    pub fn to_json(&self) -> Value {
        // Handle references
        if let Some(ref_path) = &self.reference {
            return serde_json::json!({ "$ref": ref_path });
        }

        let mut obj = serde_json::Map::new();

        // Type
        if let Some(ty) = &self.schema_type {
            if self.nullable {
                // OpenAPI 3.0 style
                obj.insert("type".into(), Value::String(ty.as_str().to_string()));
                obj.insert("nullable".into(), Value::Bool(true));
            } else {
                obj.insert("type".into(), Value::String(ty.as_str().to_string()));
            }
        }

        // Format
        if let Some(fmt) = &self.format {
            obj.insert("format".into(), Value::String(fmt.as_str().to_string()));
        }

        // Metadata
        if let Some(title) = &self.title {
            obj.insert("title".into(), Value::String(title.to_string()));
        }
        if let Some(desc) = &self.description {
            obj.insert("description".into(), Value::String(desc.to_string()));
        }
        if let Some(default) = &self.default {
            obj.insert("default".into(), default.clone());
        }
        if let Some(values) = &self.enum_values {
            obj.insert("enum".into(), Value::Array(values.clone()));
        }
        if let Some(value) = &self.const_value {
            obj.insert("const".into(), value.clone());
        }
        if self.deprecated {
            obj.insert("deprecated".into(), Value::Bool(true));
        }
        if self.read_only {
            obj.insert("readOnly".into(), Value::Bool(true));
        }
        if self.write_only {
            obj.insert("writeOnly".into(), Value::Bool(true));
        }
        if let Some(example) = &self.example {
            obj.insert("example".into(), example.clone());
        }

        // String constraints
        if let Some(min) = self.min_length {
            obj.insert("minLength".into(), Value::Number(min.into()));
        }
        if let Some(max) = self.max_length {
            obj.insert("maxLength".into(), Value::Number(max.into()));
        }
        if let Some(pattern) = &self.pattern {
            obj.insert("pattern".into(), Value::String(pattern.to_string()));
        }

        // Number constraints
        // Note: NaN and Infinity values are skipped as they cannot be represented in JSON
        if let Some(min) = self.minimum {
            if let Some(num) = serde_json::Number::from_f64(min) {
                obj.insert("minimum".into(), Value::Number(num));
            }
        }
        if let Some(max) = self.maximum {
            if let Some(num) = serde_json::Number::from_f64(max) {
                obj.insert("maximum".into(), Value::Number(num));
            }
        }
        if let Some(min) = self.exclusive_minimum {
            if let Some(num) = serde_json::Number::from_f64(min) {
                obj.insert("exclusiveMinimum".into(), Value::Number(num));
            }
        }
        if let Some(max) = self.exclusive_maximum {
            if let Some(num) = serde_json::Number::from_f64(max) {
                obj.insert("exclusiveMaximum".into(), Value::Number(num));
            }
        }
        if let Some(mult) = self.multiple_of {
            if let Some(num) = serde_json::Number::from_f64(mult) {
                obj.insert("multipleOf".into(), Value::Number(num));
            }
        }

        // Array constraints
        if let Some(items) = &self.items {
            obj.insert("items".into(), items.to_json());
        }
        if let Some(min) = self.min_items {
            obj.insert("minItems".into(), Value::Number(min.into()));
        }
        if let Some(max) = self.max_items {
            obj.insert("maxItems".into(), Value::Number(max.into()));
        }
        if self.unique_items {
            obj.insert("uniqueItems".into(), Value::Bool(true));
        }

        // Object constraints
        if let Some(props) = &self.properties {
            let props_obj: serde_json::Map<String, Value> =
                props.iter().map(|(k, v)| (k.clone(), v.to_json())).collect();
            obj.insert("properties".into(), Value::Object(props_obj));
        }
        if let Some(required) = &self.required {
            obj.insert(
                "required".into(),
                Value::Array(required.iter().map(|s| Value::String(s.clone())).collect()),
            );
        }
        if let Some(additional) = &self.additional_properties {
            obj.insert("additionalProperties".into(), additional.to_json());
        } else if let Some(allow) = self.additional_properties_bool {
            obj.insert("additionalProperties".into(), Value::Bool(allow));
        }
        if let Some(min) = self.min_properties {
            obj.insert("minProperties".into(), Value::Number(min.into()));
        }
        if let Some(max) = self.max_properties {
            obj.insert("maxProperties".into(), Value::Number(max.into()));
        }

        // Composition
        if let Some(schemas) = &self.all_of {
            obj.insert(
                "allOf".into(),
                Value::Array(schemas.iter().map(|s| s.to_json()).collect()),
            );
        }
        if let Some(schemas) = &self.any_of {
            obj.insert(
                "anyOf".into(),
                Value::Array(schemas.iter().map(|s| s.to_json()).collect()),
            );
        }
        if let Some(schemas) = &self.one_of {
            obj.insert(
                "oneOf".into(),
                Value::Array(schemas.iter().map(|s| s.to_json()).collect()),
            );
        }
        if let Some(schema) = &self.not {
            obj.insert("not".into(), schema.to_json());
        }

        // Discriminator
        if let Some(prop_name) = &self.discriminator_property {
            let mut disc = serde_json::Map::new();
            disc.insert(
                "propertyName".into(),
                Value::String(prop_name.to_string()),
            );
            if let Some(mapping) = &self.discriminator_mapping {
                let mapping_obj: serde_json::Map<String, Value> = mapping
                    .iter()
                    .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                    .collect();
                disc.insert("mapping".into(), Value::Object(mapping_obj));
            }
            obj.insert("discriminator".into(), Value::Object(disc));
        }

        Value::Object(obj)
    }
}

/// Builder for creating object schemas with a fluent API
#[derive(Debug, Clone, Default)]
pub struct ObjectSchemaBuilder {
    properties: Vec<PropertyInfo>,
    title: Option<Cow<'static, str>>,
    description: Option<Cow<'static, str>>,
    additional_properties: Option<Box<NativeSchema>>,
    additional_properties_bool: Option<bool>,
    min_properties: Option<u64>,
    max_properties: Option<u64>,
    deprecated: bool,
}

impl ObjectSchemaBuilder {
    /// Create a new object schema builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the title
    pub fn title(mut self, title: impl Into<Cow<'static, str>>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the description
    pub fn description(mut self, desc: impl Into<Cow<'static, str>>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add a property
    pub fn property(mut self, info: PropertyInfo) -> Self {
        self.properties.push(info);
        self
    }

    /// Add a required string property
    pub fn required_string(self, name: impl Into<Cow<'static, str>>) -> Self {
        self.property(
            PropertyInfo::new(name, serde_json::json!({ "type": "string" })).required(),
        )
    }

    /// Add an optional string property
    pub fn optional_string(self, name: impl Into<Cow<'static, str>>) -> Self {
        self.property(PropertyInfo::new(
            name,
            serde_json::json!({ "type": "string" }),
        ))
    }

    /// Add a required integer property
    pub fn required_integer(self, name: impl Into<Cow<'static, str>>) -> Self {
        self.property(
            PropertyInfo::new(name, serde_json::json!({ "type": "integer" })).required(),
        )
    }

    /// Add an optional integer property
    pub fn optional_integer(self, name: impl Into<Cow<'static, str>>) -> Self {
        self.property(PropertyInfo::new(
            name,
            serde_json::json!({ "type": "integer" }),
        ))
    }

    /// Add a required boolean property
    pub fn required_boolean(self, name: impl Into<Cow<'static, str>>) -> Self {
        self.property(
            PropertyInfo::new(name, serde_json::json!({ "type": "boolean" })).required(),
        )
    }

    /// Add an optional boolean property
    pub fn optional_boolean(self, name: impl Into<Cow<'static, str>>) -> Self {
        self.property(PropertyInfo::new(
            name,
            serde_json::json!({ "type": "boolean" }),
        ))
    }

    /// Set additional properties schema
    pub fn additional_properties(mut self, schema: NativeSchema) -> Self {
        self.additional_properties = Some(Box::new(schema));
        self
    }

    /// Disallow additional properties
    pub fn no_additional_properties(mut self) -> Self {
        self.additional_properties_bool = Some(false);
        self
    }

    /// Set minimum number of properties
    pub fn min_properties(mut self, min: u64) -> Self {
        self.min_properties = Some(min);
        self
    }

    /// Set maximum number of properties
    pub fn max_properties(mut self, max: u64) -> Self {
        self.max_properties = Some(max);
        self
    }

    /// Mark as deprecated
    pub fn deprecated(mut self) -> Self {
        self.deprecated = true;
        self
    }

    /// Build the schema as JSON value
    pub fn build(self) -> Value {
        let mut obj = serde_json::Map::new();
        obj.insert("type".into(), Value::String("object".to_string()));

        if let Some(title) = self.title {
            obj.insert("title".into(), Value::String(title.to_string()));
        }
        if let Some(desc) = self.description {
            obj.insert("description".into(), Value::String(desc.to_string()));
        }

        // Properties
        if !self.properties.is_empty() {
            let mut props = serde_json::Map::new();
            let mut required = Vec::new();

            for prop in &self.properties {
                let mut schema = prop.schema.clone();

                // Add property-level attributes
                if let Value::Object(ref mut schema_obj) = schema {
                    if let Some(desc) = &prop.description {
                        schema_obj.insert("description".into(), Value::String(desc.to_string()));
                    }
                    if let Some(default) = &prop.default {
                        schema_obj.insert("default".into(), default.clone());
                    }
                    if prop.deprecated {
                        schema_obj.insert("deprecated".into(), Value::Bool(true));
                    }
                    if prop.nullable {
                        schema_obj.insert("nullable".into(), Value::Bool(true));
                    }
                    if prop.read_only {
                        schema_obj.insert("readOnly".into(), Value::Bool(true));
                    }
                    if prop.write_only {
                        schema_obj.insert("writeOnly".into(), Value::Bool(true));
                    }
                }

                props.insert(prop.name.to_string(), schema);

                if prop.required {
                    required.push(Value::String(prop.name.to_string()));
                }
            }

            obj.insert("properties".into(), Value::Object(props));

            if !required.is_empty() {
                obj.insert("required".into(), Value::Array(required));
            }
        }

        // Additional properties
        if let Some(schema) = self.additional_properties {
            obj.insert("additionalProperties".into(), schema.to_json());
        } else if let Some(allow) = self.additional_properties_bool {
            obj.insert("additionalProperties".into(), Value::Bool(allow));
        }

        if let Some(min) = self.min_properties {
            obj.insert("minProperties".into(), Value::Number(min.into()));
        }
        if let Some(max) = self.max_properties {
            obj.insert("maxProperties".into(), Value::Number(max.into()));
        }

        if self.deprecated {
            obj.insert("deprecated".into(), Value::Bool(true));
        }

        Value::Object(obj)
    }

    /// Build as NativeSchema
    pub fn build_native(self) -> NativeSchema {
        let mut schema = NativeSchema::object();

        schema.title = self.title;
        schema.description = self.description;
        schema.deprecated = self.deprecated;
        schema.min_properties = self.min_properties;
        schema.max_properties = self.max_properties;
        schema.additional_properties = self.additional_properties;
        schema.additional_properties_bool = self.additional_properties_bool;

        // Convert properties
        let mut properties = HashMap::new();
        let mut required = Vec::new();

        for prop in self.properties {
            // Parse the JSON value into a NativeSchema (simplified)
            let prop_schema = NativeSchema {
                description: prop.description.map(|d| d.to_string().into()),
                default: prop.default,
                deprecated: prop.deprecated,
                nullable: prop.nullable,
                read_only: prop.read_only,
                write_only: prop.write_only,
                ..Default::default()
            };

            properties.insert(prop.name.to_string(), prop_schema);

            if prop.required {
                required.push(prop.name.to_string());
            }
        }

        if !properties.is_empty() {
            schema.properties = Some(properties);
        }
        if !required.is_empty() {
            schema.required = Some(required);
        }

        schema
    }
}

/// Builder for creating schemas with a fluent API
#[derive(Debug, Clone, Default)]
pub struct NativeSchemaBuilder {
    inner: NativeSchema,
    name: Option<Cow<'static, str>>,
}

impl NativeSchemaBuilder {
    /// Create a new schema builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a string schema builder
    pub fn string() -> Self {
        Self {
            inner: NativeSchema::string(),
            name: None,
        }
    }

    /// Create a number schema builder
    pub fn number() -> Self {
        Self {
            inner: NativeSchema::number(),
            name: None,
        }
    }

    /// Create an integer schema builder
    pub fn integer() -> Self {
        Self {
            inner: NativeSchema::integer(),
            name: None,
        }
    }

    /// Create a boolean schema builder
    pub fn boolean() -> Self {
        Self {
            inner: NativeSchema::boolean(),
            name: None,
        }
    }

    /// Create an array schema builder
    pub fn array(items: NativeSchema) -> Self {
        Self {
            inner: NativeSchema::array(items),
            name: None,
        }
    }

    /// Create an object schema builder
    pub fn object() -> Self {
        Self {
            inner: NativeSchema::object(),
            name: None,
        }
    }

    /// Set the schema name
    pub fn name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the format
    pub fn format(mut self, format: SchemaFormat) -> Self {
        self.inner = self.inner.with_format(format);
        self
    }

    /// Set the title
    pub fn title(mut self, title: impl Into<Cow<'static, str>>) -> Self {
        self.inner = self.inner.with_title(title);
        self
    }

    /// Set the description
    pub fn description(mut self, desc: impl Into<Cow<'static, str>>) -> Self {
        self.inner = self.inner.with_description(desc);
        self
    }

    /// Set the default value
    pub fn with_default(mut self, default: Value) -> Self {
        self.inner = self.inner.with_default(default);
        self
    }

    /// Set enum values
    pub fn enum_values(mut self, values: Vec<Value>) -> Self {
        self.inner = self.inner.with_enum(values);
        self
    }

    /// Make nullable
    pub fn nullable(mut self) -> Self {
        self.inner = self.inner.nullable();
        self
    }

    /// Mark as deprecated
    pub fn deprecated(mut self) -> Self {
        self.inner = self.inner.deprecated();
        self
    }

    /// Set example value
    pub fn example(mut self, example: Value) -> Self {
        self.inner = self.inner.with_example(example);
        self
    }

    /// Add a property (for objects)
    pub fn property(mut self, name: impl Into<String>, schema: NativeSchema) -> Self {
        self.inner = self.inner.with_property(name, schema);
        self
    }

    /// Mark a property as required (for objects)
    pub fn required(mut self, name: impl Into<String>) -> Self {
        self.inner = self.inner.with_required(name);
        self
    }

    /// Build the schema
    pub fn build(self) -> NativeSchema {
        self.inner
    }

    /// Build as a tuple of (name, schema JSON value)
    pub fn build_named(self) -> (Cow<'static, str>, Value) {
        let name = self.name.unwrap_or_else(|| "Schema".into());
        (name, self.inner.to_json())
    }
}
