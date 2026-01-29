use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Trait for types that can generate their own OpenAPI schema.
pub trait ToSchema {
    /// Get the name of the schema (for ref)
    fn name() -> String;

    /// Generate the schema object
    fn schema() -> (String, RefOr<Schema>);
}

/// Reference or inline schema
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum RefOr<T> {
    Ref(Reference),
    T(T),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Reference {
    #[serde(rename = "$ref")]
    pub ref_path: String,
}

/// OpenAPI Schema Object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Schema {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub schema_type: Option<SchemaType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, RefOr<Schema>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<RefOr<Schema>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SchemaType {
    String,
    Integer,
    Number,
    Boolean,
    Object,
    Array,
}

impl<T> From<T> for RefOr<T> {
    fn from(t: T) -> Self {
        RefOr::T(t)
    }
}

// Primitives Implementation

impl ToSchema for String {
    fn name() -> String {
        "String".to_string()
    }

    fn schema() -> (String, RefOr<Schema>) {
        (
            "String".to_string(),
            Schema {
                schema_type: Some(SchemaType::String),
                ..Default::default()
            }
            .into(),
        )
    }
}

impl ToSchema for &str {
    fn name() -> String {
        "String".to_string()
    }

    fn schema() -> (String, RefOr<Schema>) {
        (
            "String".to_string(),
            Schema {
                schema_type: Some(SchemaType::String),
                ..Default::default()
            }
            .into(),
        )
    }
}

impl ToSchema for bool {
    fn name() -> String {
        "Boolean".to_string()
    }

    fn schema() -> (String, RefOr<Schema>) {
        (
            "Boolean".to_string(),
            Schema {
                schema_type: Some(SchemaType::Boolean),
                ..Default::default()
            }
            .into(),
        )
    }
}

// Integer types
macro_rules! impl_int_schema {
    ($($ty:ty),*) => {
        $(
            impl ToSchema for $ty {
                fn name() -> String {
                    "Integer".to_string()
                }

                fn schema() -> (String, RefOr<Schema>) {
                    (
                        "Integer".to_string(),
                        Schema {
                            schema_type: Some(SchemaType::Integer),
                            format: if std::mem::size_of::<$ty>() > 4 { Some("int64".to_string()) } else { Some("int32".to_string()) },
                            ..Default::default()
                        }
                        .into(),
                    )
                }
            }
        )*
    };
}

impl_int_schema!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

// Float types
macro_rules! impl_float_schema {
    ($($ty:ty),*) => {
        $(
            impl ToSchema for $ty {
                fn name() -> String {
                    "Number".to_string()
                }

                fn schema() -> (String, RefOr<Schema>) {
                    (
                        "Number".to_string(),
                        Schema {
                            schema_type: Some(SchemaType::Number),
                            format: if std::mem::size_of::<$ty>() > 4 { Some("double".to_string()) } else { Some("float".to_string()) },
                            ..Default::default()
                        }
                        .into(),
                    )
                }
            }
        )*
    };
}

impl_float_schema!(f32, f64);

// Option
impl<T: ToSchema> ToSchema for Option<T> {
    fn name() -> String {
        T::name()
    }

    fn schema() -> (String, RefOr<Schema>) {
        // Option doesn't change the schema structure in OpenAPI 3.0 usually,
        // it just means it's not in 'required' list of parent object, or nullable: true
        // For simplicity, we delegate to T
        T::schema()
    }
}

// Vec
impl<T: ToSchema> ToSchema for Vec<T> {
    fn name() -> String {
        format!("Array_of_{}", T::name())
    }

    fn schema() -> (String, RefOr<Schema>) {
        let (_, item_schema) = T::schema();
        (
            Self::name(),
            Schema {
                schema_type: Some(SchemaType::Array),
                items: Some(Box::new(item_schema)),
                ..Default::default()
            }
            .into(),
        )
    }
}

// UUID support (if feature enabled, or just hardcode as string for now as it's common)
#[cfg(feature = "uuid")]
impl ToSchema for uuid::Uuid {
    fn name() -> String {
        "Uuid".to_string()
    }

    fn schema() -> (String, RefOr<Schema>) {
        (
            "Uuid".to_string(),
            Schema {
                schema_type: Some(SchemaType::String),
                format: Some("uuid".to_string()),
                ..Default::default()
            }
            .into(),
        )
    }
}
