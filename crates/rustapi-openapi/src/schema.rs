//! JSON Schema 2020-12 support and RustApiSchema trait

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Type array for nullable types in JSON Schema 2020-12
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum TypeArray {
    Single(String),
    Array(Vec<String>),
}

impl TypeArray {
    pub fn single(ty: impl Into<String>) -> Self {
        Self::Single(ty.into())
    }

    pub fn nullable(ty: impl Into<String>) -> Self {
        Self::Array(vec![ty.into(), "null".to_string()])
    }
}

/// JSON Schema 2020-12 schema definition
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JsonSchema2020 {
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(rename = "$id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub schema_type: Option<TypeArray>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    #[serde(rename = "const", skip_serializing_if = "Option::is_none")]
    pub const_value: Option<serde_json::Value>,
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<JsonSchema2020>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<BTreeMap<String, JsonSchema2020>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<Box<AdditionalProperties>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub one_of: Option<Vec<JsonSchema2020>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<JsonSchema2020>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_of: Option<Vec<JsonSchema2020>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
}

impl JsonSchema2020 {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn string() -> Self {
        Self {
            schema_type: Some(TypeArray::single("string")),
            ..Default::default()
        }
    }
    pub fn integer() -> Self {
        Self {
            schema_type: Some(TypeArray::single("integer")),
            ..Default::default()
        }
    }
    pub fn number() -> Self {
        Self {
            schema_type: Some(TypeArray::single("number")),
            ..Default::default()
        }
    }
    pub fn boolean() -> Self {
        Self {
            schema_type: Some(TypeArray::single("boolean")),
            ..Default::default()
        }
    }
    pub fn array(items: JsonSchema2020) -> Self {
        Self {
            schema_type: Some(TypeArray::single("array")),
            items: Some(Box::new(items)),
            ..Default::default()
        }
    }
    pub fn object() -> Self {
        Self {
            schema_type: Some(TypeArray::single("object")),
            properties: Some(BTreeMap::new()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum AdditionalProperties {
    Bool(bool),
    Schema(Box<JsonSchema2020>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SchemaRef {
    Ref {
        #[serde(rename = "$ref")]
        reference: String,
    },
    Schema(Box<JsonSchema2020>),
    Inline(serde_json::Value),
}

pub struct SchemaCtx {
    pub components: BTreeMap<String, JsonSchema2020>,
}

impl Default for SchemaCtx {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaCtx {
    pub fn new() -> Self {
        Self {
            components: BTreeMap::new(),
        }
    }
}

pub trait RustApiSchema {
    fn schema(ctx: &mut SchemaCtx) -> SchemaRef;
    fn component_name() -> Option<&'static str> {
        None
    }

    /// Get a unique name for this type, including generic parameters.
    /// Used for preventing name collisions in schema registry.
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Unknown")
    }

    /// Get field schemas if this type is a struct (for Query params extraction)
    fn field_schemas(_ctx: &mut SchemaCtx) -> Option<BTreeMap<String, SchemaRef>> {
        None
    }
}

// Primitives
impl RustApiSchema for String {
    fn schema(_: &mut SchemaCtx) -> SchemaRef {
        SchemaRef::Schema(Box::new(JsonSchema2020::string()))
    }
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("String")
    }
}
impl RustApiSchema for &str {
    fn schema(_: &mut SchemaCtx) -> SchemaRef {
        SchemaRef::Schema(Box::new(JsonSchema2020::string()))
    }
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("String")
    }
}
impl RustApiSchema for bool {
    fn schema(_: &mut SchemaCtx) -> SchemaRef {
        SchemaRef::Schema(Box::new(JsonSchema2020::boolean()))
    }
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Boolean")
    }
}
impl RustApiSchema for i32 {
    fn schema(_: &mut SchemaCtx) -> SchemaRef {
        let mut s = JsonSchema2020::integer();
        s.format = Some("int32".to_string());
        SchemaRef::Schema(Box::new(s))
    }
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Int32")
    }
}
impl RustApiSchema for i64 {
    fn schema(_: &mut SchemaCtx) -> SchemaRef {
        let mut s = JsonSchema2020::integer();
        s.format = Some("int64".to_string());
        SchemaRef::Schema(Box::new(s))
    }
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Int64")
    }
}
impl RustApiSchema for f64 {
    fn schema(_: &mut SchemaCtx) -> SchemaRef {
        let mut s = JsonSchema2020::number();
        s.format = Some("double".to_string());
        SchemaRef::Schema(Box::new(s))
    }
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Float64")
    }
}
impl RustApiSchema for f32 {
    fn schema(_: &mut SchemaCtx) -> SchemaRef {
        let mut s = JsonSchema2020::number();
        s.format = Some("float".to_string());
        SchemaRef::Schema(Box::new(s))
    }
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Float32")
    }
}

impl RustApiSchema for i8 {
    fn schema(_: &mut SchemaCtx) -> SchemaRef {
        let mut s = JsonSchema2020::integer();
        s.format = Some("int8".to_string());
        SchemaRef::Schema(Box::new(s))
    }
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Int8")
    }
}
impl RustApiSchema for i16 {
    fn schema(_: &mut SchemaCtx) -> SchemaRef {
        let mut s = JsonSchema2020::integer();
        s.format = Some("int16".to_string());
        SchemaRef::Schema(Box::new(s))
    }
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Int16")
    }
}
impl RustApiSchema for isize {
    fn schema(_: &mut SchemaCtx) -> SchemaRef {
        let mut s = JsonSchema2020::integer();
        s.format = Some("int64".to_string());
        SchemaRef::Schema(Box::new(s))
    }
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Int64")
    }
}
impl RustApiSchema for u8 {
    fn schema(_: &mut SchemaCtx) -> SchemaRef {
        let mut s = JsonSchema2020::integer();
        s.format = Some("uint8".to_string());
        SchemaRef::Schema(Box::new(s))
    }
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Uint8")
    }
}
impl RustApiSchema for u16 {
    fn schema(_: &mut SchemaCtx) -> SchemaRef {
        let mut s = JsonSchema2020::integer();
        s.format = Some("uint16".to_string());
        SchemaRef::Schema(Box::new(s))
    }
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Uint16")
    }
}
impl RustApiSchema for u32 {
    fn schema(_: &mut SchemaCtx) -> SchemaRef {
        let mut s = JsonSchema2020::integer();
        s.format = Some("uint32".to_string());
        SchemaRef::Schema(Box::new(s))
    }
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Uint32")
    }
}
impl RustApiSchema for u64 {
    fn schema(_: &mut SchemaCtx) -> SchemaRef {
        let mut s = JsonSchema2020::integer();
        s.format = Some("uint64".to_string());
        SchemaRef::Schema(Box::new(s))
    }
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Uint64")
    }
}
impl RustApiSchema for usize {
    fn schema(_: &mut SchemaCtx) -> SchemaRef {
        let mut s = JsonSchema2020::integer();
        s.format = Some("uint64".to_string());
        SchemaRef::Schema(Box::new(s))
    }
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Uint64")
    }
}

// Vec
impl<T: RustApiSchema> RustApiSchema for Vec<T> {
    fn schema(ctx: &mut SchemaCtx) -> SchemaRef {
        match T::schema(ctx) {
            SchemaRef::Schema(s) => SchemaRef::Schema(Box::new(JsonSchema2020::array(*s))),
            SchemaRef::Ref { reference } => {
                // If T is a ref, items: {$ref: ...}
                let mut s = JsonSchema2020::new();
                s.schema_type = Some(TypeArray::single("array"));
                let mut ref_schema = JsonSchema2020::new();
                ref_schema.reference = Some(reference);
                s.items = Some(Box::new(ref_schema));
                SchemaRef::Schema(Box::new(s))
            }
            SchemaRef::Inline(_) => SchemaRef::Schema(Box::new(JsonSchema2020 {
                schema_type: Some(TypeArray::single("array")),
                // Inline not easily convertible to JsonSchema2020 without parsing
                // Fallback to minimal array
                ..Default::default()
            })),
        }
    }
    fn name() -> std::borrow::Cow<'static, str> {
        format!("Array_{}", T::name()).into()
    }
}

// Option
impl<T: RustApiSchema> RustApiSchema for Option<T> {
    fn schema(ctx: &mut SchemaCtx) -> SchemaRef {
        let inner = T::schema(ctx);
        match inner {
            SchemaRef::Schema(mut s) => {
                if let Some(t) = s.schema_type {
                    s.schema_type = Some(TypeArray::nullable(match t {
                        TypeArray::Single(v) => v,
                        TypeArray::Array(v) => v[0].clone(), // Approximate
                    }));
                }
                SchemaRef::Schema(s)
            }
            SchemaRef::Ref { reference } => {
                // oneOf [{$ref}, {type: null}]
                let mut s = JsonSchema2020::new();
                let mut ref_s = JsonSchema2020::new();
                ref_s.reference = Some(reference);
                let mut null_s = JsonSchema2020::new();
                null_s.schema_type = Some(TypeArray::single("null"));
                s.one_of = Some(vec![ref_s, null_s]);
                SchemaRef::Schema(Box::new(s))
            }
            _ => inner,
        }
    }
    fn name() -> std::borrow::Cow<'static, str> {
        format!("Option_{}", T::name()).into()
    }
}

// HashMap
impl<T: RustApiSchema> RustApiSchema for std::collections::HashMap<String, T> {
    fn schema(ctx: &mut SchemaCtx) -> SchemaRef {
        let inner = T::schema(ctx);
        let mut s = JsonSchema2020::object();

        let add_prop = match inner {
            SchemaRef::Schema(is) => AdditionalProperties::Schema(is),
            SchemaRef::Ref { reference } => {
                let mut rs = JsonSchema2020::new();
                rs.reference = Some(reference);
                AdditionalProperties::Schema(Box::new(rs))
            }
            _ => AdditionalProperties::Bool(true),
        };

        s.additional_properties = Some(Box::new(add_prop));
        SchemaRef::Schema(Box::new(s))
    }
    fn name() -> std::borrow::Cow<'static, str> {
        format!("Map_{}", T::name()).into()
    }
}

// serde_json::Value
impl RustApiSchema for serde_json::Value {
    fn schema(_: &mut SchemaCtx) -> SchemaRef {
        SchemaRef::Schema(Box::new(JsonSchema2020::new()))
    }
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Any")
    }
}

// Add empty SchemaTransformer for spec.rs usage
pub struct SchemaTransformer;
impl SchemaTransformer {
    pub fn transform_30_to_31(v: serde_json::Value) -> serde_json::Value {
        v
    }
}
