//! Tests for native OpenAPI schema generation

#[cfg(test)]
mod tests {
    use crate::native::*;
    use serde_json::json;

    // ========================================================================
    // ToOpenApiSchema trait tests
    // ========================================================================

    #[test]
    fn test_primitive_schemas() {
        // Test bool
        let (name, schema) = bool::schema();
        assert_eq!(name, "boolean");
        assert_eq!(schema, json!({ "type": "boolean" }));

        // Test i32
        let (name, schema) = i32::schema();
        assert_eq!(name, "i32");
        assert_eq!(schema["type"], "integer");
        assert_eq!(schema["format"], "int32");

        // Test i64
        let (name, schema) = i64::schema();
        assert_eq!(name, "i64");
        assert_eq!(schema["type"], "integer");
        assert_eq!(schema["format"], "int64");

        // Test f32
        let (name, schema) = f32::schema();
        assert_eq!(name, "f32");
        assert_eq!(schema["type"], "number");
        assert_eq!(schema["format"], "float");

        // Test f64
        let (name, schema) = f64::schema();
        assert_eq!(name, "f64");
        assert_eq!(schema["type"], "number");
        assert_eq!(schema["format"], "double");

        // Test String
        let (name, schema) = String::schema();
        assert_eq!(name, "String");
        assert_eq!(schema, json!({ "type": "string" }));
    }

    #[test]
    fn test_option_schema() {
        let (name, schema) = Option::<String>::schema();
        assert!(name.starts_with("Option_"));
        assert_eq!(schema["type"], "string");
        assert_eq!(schema["nullable"], true);
    }

    #[test]
    fn test_vec_schema() {
        let (name, schema) = Vec::<i32>::schema();
        assert!(name.starts_with("Vec_"));
        assert_eq!(schema["type"], "array");
        assert_eq!(schema["items"]["type"], "integer");
    }

    #[test]
    fn test_hashset_schema() {
        let (name, schema) = std::collections::HashSet::<String>::schema();
        assert!(name.starts_with("HashSet_"));
        assert_eq!(schema["type"], "array");
        assert_eq!(schema["uniqueItems"], true);
    }

    #[test]
    fn test_hashmap_schema() {
        let (name, schema) = std::collections::HashMap::<String, i32>::schema();
        assert!(name.starts_with("HashMap_"));
        assert_eq!(schema["type"], "object");
        assert!(schema["additionalProperties"].is_object());
    }

    #[test]
    fn test_result_schema() {
        let (name, schema) = Result::<String, i32>::schema();
        assert!(name.starts_with("Result_"));
        assert!(schema["oneOf"].is_array());
    }

    #[test]
    fn test_tuple_schema() {
        let (name, schema) = <(i32, String)>::schema();
        assert!(name.starts_with("Tuple2_"));
        assert_eq!(schema["type"], "array");
        assert_eq!(schema["minItems"], 2);
        assert_eq!(schema["maxItems"], 2);
        assert!(schema["prefixItems"].is_array());
    }

    #[test]
    fn test_schema_ref() {
        let schema_ref = String::schema_ref();
        assert_eq!(schema_ref["$ref"], "#/components/schemas/String");
    }

    // ========================================================================
    // NativeSchema tests
    // ========================================================================

    #[test]
    fn test_native_schema_string() {
        let schema = NativeSchema::string()
            .with_title("Name")
            .with_description("User's full name")
            .min_length(1)
            .max_length(100);

        let json = schema.to_json();
        assert_eq!(json["type"], "string");
        assert_eq!(json["title"], "Name");
        assert_eq!(json["description"], "User's full name");
        assert_eq!(json["minLength"], 1);
        assert_eq!(json["maxLength"], 100);
    }

    #[test]
    fn test_native_schema_number() {
        let schema = NativeSchema::number()
            .with_format(SchemaFormat::Double)
            .minimum(0.0)
            .maximum(100.0);

        let json = schema.to_json();
        assert_eq!(json["type"], "number");
        assert_eq!(json["format"], "double");
        assert_eq!(json["minimum"], 0.0);
        assert_eq!(json["maximum"], 100.0);
    }

    #[test]
    fn test_native_schema_integer() {
        let schema = NativeSchema::integer()
            .with_format(SchemaFormat::Int64)
            .exclusive_minimum(0.0);

        let json = schema.to_json();
        assert_eq!(json["type"], "integer");
        assert_eq!(json["format"], "int64");
        assert_eq!(json["exclusiveMinimum"], 0.0);
    }

    #[test]
    fn test_native_schema_array() {
        let schema = NativeSchema::array(NativeSchema::string())
            .min_items(1)
            .max_items(10)
            .unique_items();

        let json = schema.to_json();
        assert_eq!(json["type"], "array");
        assert_eq!(json["items"]["type"], "string");
        assert_eq!(json["minItems"], 1);
        assert_eq!(json["maxItems"], 10);
        assert_eq!(json["uniqueItems"], true);
    }

    #[test]
    fn test_native_schema_object() {
        let schema = NativeSchema::object()
            .with_property("id", NativeSchema::integer())
            .with_property("name", NativeSchema::string())
            .with_required("id")
            .with_required("name");

        let json = schema.to_json();
        assert_eq!(json["type"], "object");
        assert!(json["properties"]["id"].is_object());
        assert!(json["properties"]["name"].is_object());
        assert!(json["required"].as_array().unwrap().contains(&json!("id")));
        assert!(json["required"]
            .as_array()
            .unwrap()
            .contains(&json!("name")));
    }

    #[test]
    fn test_native_schema_nullable() {
        let schema = NativeSchema::string().nullable();
        let json = schema.to_json();
        assert_eq!(json["type"], "string");
        assert_eq!(json["nullable"], true);
    }

    #[test]
    fn test_native_schema_enum() {
        let schema =
            NativeSchema::string().with_enum(vec![json!("active"), json!("inactive"), json!("pending")]);

        let json = schema.to_json();
        assert_eq!(json["type"], "string");
        assert!(json["enum"].is_array());
        assert_eq!(json["enum"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_native_schema_reference() {
        let schema = NativeSchema::ref_to("User");
        let json = schema.to_json();
        assert_eq!(json["$ref"], "#/components/schemas/User");
    }

    #[test]
    fn test_native_schema_allof() {
        let schema = NativeSchema::new().all_of(vec![
            NativeSchema::ref_to("BaseEntity"),
            NativeSchema::object()
                .with_property("name", NativeSchema::string())
                .with_required("name"),
        ]);

        let json = schema.to_json();
        assert!(json["allOf"].is_array());
        assert_eq!(json["allOf"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_native_schema_oneof() {
        let schema = NativeSchema::new()
            .one_of(vec![
                NativeSchema::ref_to("Cat"),
                NativeSchema::ref_to("Dog"),
            ])
            .discriminator("petType", None);

        let json = schema.to_json();
        assert!(json["oneOf"].is_array());
        assert_eq!(json["discriminator"]["propertyName"], "petType");
    }

    #[test]
    fn test_native_schema_deprecated() {
        let schema = NativeSchema::string()
            .deprecated()
            .with_description("This field is deprecated");

        let json = schema.to_json();
        assert_eq!(json["deprecated"], true);
    }

    // ========================================================================
    // ObjectSchemaBuilder tests
    // ========================================================================

    #[test]
    fn test_object_schema_builder() {
        let schema = ObjectSchemaBuilder::new()
            .title("User")
            .description("A user in the system")
            .required_string("username")
            .required_string("email")
            .optional_string("bio")
            .optional_integer("age")
            .no_additional_properties()
            .build();

        assert_eq!(schema["type"], "object");
        assert_eq!(schema["title"], "User");
        assert_eq!(schema["description"], "A user in the system");
        assert!(schema["properties"]["username"].is_object());
        assert!(schema["properties"]["email"].is_object());
        assert!(schema["properties"]["bio"].is_object());
        assert!(schema["properties"]["age"].is_object());
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("username")));
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("email")));
        assert!(!schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("bio")));
        assert_eq!(schema["additionalProperties"], false);
    }

    #[test]
    fn test_object_schema_builder_with_custom_properties() {
        let schema = ObjectSchemaBuilder::new()
            .property(
                PropertyInfo::new("id", json!({ "type": "integer", "format": "int64" }))
                    .required()
                    .with_description("Unique identifier"),
            )
            .property(
                PropertyInfo::new("created_at", json!({ "type": "string", "format": "date-time" }))
                    .required()
                    .read_only(),
            )
            .property(
                PropertyInfo::new("password", json!({ "type": "string" }))
                    .write_only()
                    .with_description("User password (write-only)"),
            )
            .build();

        assert_eq!(schema["properties"]["id"]["format"], "int64");
        assert_eq!(
            schema["properties"]["id"]["description"],
            "Unique identifier"
        );
        assert_eq!(schema["properties"]["created_at"]["readOnly"], true);
        assert_eq!(schema["properties"]["password"]["writeOnly"], true);
    }

    // ========================================================================
    // NativeSchemaBuilder tests
    // ========================================================================

    #[test]
    fn test_native_schema_builder() {
        let (name, schema) = NativeSchemaBuilder::string()
            .name("Email")
            .format(SchemaFormat::Email)
            .description("Email address")
            .build_named();

        assert_eq!(name, "Email");
        assert_eq!(schema["type"], "string");
        assert_eq!(schema["format"], "email");
        assert_eq!(schema["description"], "Email address");
    }

    #[test]
    fn test_native_schema_builder_object() {
        let schema = NativeSchemaBuilder::object()
            .title("Address")
            .property("street", NativeSchema::string())
            .property("city", NativeSchema::string())
            .property("zipCode", NativeSchema::string())
            .required("street")
            .required("city")
            .build();

        let json = schema.to_json();
        assert_eq!(json["title"], "Address");
        assert!(json["properties"]["street"].is_object());
        assert!(json["properties"]["city"].is_object());
        assert!(json["required"]
            .as_array()
            .unwrap()
            .contains(&json!("street")));
    }

    // ========================================================================
    // ParamInfo tests
    // ========================================================================

    #[test]
    fn test_param_info_path() {
        let param = ParamInfo::path("id", json!({ "type": "integer" }))
            .with_description("Resource ID")
            .with_example(json!(123));

        let json = param.to_openapi();
        assert_eq!(json["name"], "id");
        assert_eq!(json["in"], "path");
        assert_eq!(json["required"], true);
        assert_eq!(json["description"], "Resource ID");
        assert_eq!(json["example"], 123);
    }

    #[test]
    fn test_param_info_query() {
        let param = ParamInfo::query("page", json!({ "type": "integer" }), false)
            .with_description("Page number")
            .with_example(json!(1));

        let json = param.to_openapi();
        assert_eq!(json["name"], "page");
        assert_eq!(json["in"], "query");
        assert_eq!(json["required"], false);
        assert_eq!(json["description"], "Page number");
    }

    #[test]
    fn test_param_info_header() {
        let param = ParamInfo::header("X-API-Key", json!({ "type": "string" }), true)
            .with_description("API Key for authentication")
            .deprecated();

        let json = param.to_openapi();
        assert_eq!(json["name"], "X-API-Key");
        assert_eq!(json["in"], "header");
        assert_eq!(json["required"], true);
        assert_eq!(json["deprecated"], true);
    }

    // ========================================================================
    // SchemaFormat tests
    // ========================================================================

    #[test]
    fn test_schema_format_as_str() {
        assert_eq!(SchemaFormat::Int32.as_str(), "int32");
        assert_eq!(SchemaFormat::Int64.as_str(), "int64");
        assert_eq!(SchemaFormat::Float.as_str(), "float");
        assert_eq!(SchemaFormat::Double.as_str(), "double");
        assert_eq!(SchemaFormat::Date.as_str(), "date");
        assert_eq!(SchemaFormat::DateTime.as_str(), "date-time");
        assert_eq!(SchemaFormat::Email.as_str(), "email");
        assert_eq!(SchemaFormat::Uri.as_str(), "uri");
        assert_eq!(SchemaFormat::Uuid.as_str(), "uuid");
        assert_eq!(SchemaFormat::Password.as_str(), "password");
        assert_eq!(SchemaFormat::Binary.as_str(), "binary");
        assert_eq!(
            SchemaFormat::Custom("custom-format".into()).as_str(),
            "custom-format"
        );
    }

    // ========================================================================
    // Integration tests
    // ========================================================================

    #[test]
    fn test_complex_schema_composition() {
        // Create a complex schema with inheritance and composition
        let base_entity = NativeSchema::object()
            .with_property("id", NativeSchema::integer().with_format(SchemaFormat::Int64))
            .with_property(
                "createdAt",
                NativeSchema::string().with_format(SchemaFormat::DateTime),
            )
            .with_required("id")
            .with_required("createdAt");

        let user_schema = NativeSchema::new().all_of(vec![
            NativeSchema::ref_to("BaseEntity"),
            NativeSchema::object()
                .with_property("username", NativeSchema::string().min_length(3).max_length(50))
                .with_property(
                    "email",
                    NativeSchema::string().with_format(SchemaFormat::Email),
                )
                .with_property(
                    "role",
                    NativeSchema::string().with_enum(vec![
                        json!("admin"),
                        json!("user"),
                        json!("guest"),
                    ]),
                )
                .with_required("username")
                .with_required("email")
                .with_required("role"),
        ]);

        let base_json = base_entity.to_json();
        let user_json = user_schema.to_json();

        // Verify base entity
        assert_eq!(base_json["type"], "object");
        assert!(base_json["properties"]["id"].is_object());

        // Verify user schema uses allOf
        assert!(user_json["allOf"].is_array());
        let all_of = user_json["allOf"].as_array().unwrap();
        assert_eq!(all_of.len(), 2);
        assert_eq!(all_of[0]["$ref"], "#/components/schemas/BaseEntity");
    }

    #[test]
    fn test_polymorphic_schema() {
        let animal_schema = NativeSchema::new()
            .one_of(vec![
                NativeSchema::ref_to("Cat"),
                NativeSchema::ref_to("Dog"),
                NativeSchema::ref_to("Bird"),
            ])
            .discriminator(
                "animalType",
                Some(
                    vec![
                        ("cat".to_string(), "#/components/schemas/Cat".to_string()),
                        ("dog".to_string(), "#/components/schemas/Dog".to_string()),
                        ("bird".to_string(), "#/components/schemas/Bird".to_string()),
                    ]
                    .into_iter()
                    .collect(),
                ),
            );

        let json = animal_schema.to_json();
        assert!(json["oneOf"].is_array());
        assert_eq!(json["discriminator"]["propertyName"], "animalType");
        assert!(json["discriminator"]["mapping"].is_object());
    }

    // ========================================================================
    // OpenApiSpec integration tests
    // ========================================================================

    #[test]
    fn test_openapi_spec_register_native() {
        use crate::OpenApiSpec;

        // Create a simple type for testing
        struct TestUser;

        impl ToOpenApiSchema for TestUser {
            fn schema() -> (std::borrow::Cow<'static, str>, serde_json::Value) {
                (
                    "TestUser".into(),
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "id": { "type": "integer" },
                            "name": { "type": "string" }
                        },
                        "required": ["id", "name"]
                    }),
                )
            }
        }

        let spec = OpenApiSpec::new("Test API", "1.0.0")
            .description("Test API description")
            .register_native::<TestUser>();

        let json = spec.to_json();

        // Verify the schema was registered
        assert_eq!(json["openapi"], "3.0.3");
        assert_eq!(json["info"]["title"], "Test API");
        assert!(json["components"]["schemas"]["TestUser"].is_object());
        assert_eq!(json["components"]["schemas"]["TestUser"]["type"], "object");
    }

    #[test]
    fn test_openapi_spec_register_native_in_place() {
        use crate::OpenApiSpec;

        struct TestProduct;

        impl ToOpenApiSchema for TestProduct {
            fn schema() -> (std::borrow::Cow<'static, str>, serde_json::Value) {
                (
                    "TestProduct".into(),
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "sku": { "type": "string" },
                            "price": { "type": "number" }
                        }
                    }),
                )
            }
        }

        let mut spec = OpenApiSpec::new("Test API", "1.0.0");
        spec.register_native_in_place::<TestProduct>();

        let json = spec.to_json();

        // Verify the schema was registered
        assert!(json["components"]["schemas"]["TestProduct"].is_object());
        assert!(json["components"]["schemas"]["TestProduct"]["properties"]["sku"].is_object());
    }

    #[test]
    fn test_openapi_31_spec_register_native() {
        use crate::v31::OpenApi31Spec;

        struct TestOrder;

        impl ToOpenApiSchema for TestOrder {
            fn schema() -> (std::borrow::Cow<'static, str>, serde_json::Value) {
                (
                    "TestOrder".into(),
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "orderId": { "type": "string", "format": "uuid" },
                            "amount": { "type": "number" },
                            "status": { "type": "string", "nullable": true }
                        },
                        "required": ["orderId", "amount"]
                    }),
                )
            }
        }

        let spec = OpenApi31Spec::new("Test API", "1.0.0")
            .description("Test API for OpenAPI 3.1")
            .register_native::<TestOrder>();

        let json = spec.to_json();

        // Verify OpenAPI 3.1 version
        assert_eq!(json["openapi"], "3.1.0");
        assert!(json["components"]["schemas"]["TestOrder"].is_object());
    }

    #[test]
    fn test_openapi_spec_multiple_native_schemas() {
        use crate::OpenApiSpec;

        struct SchemaA;
        struct SchemaB;

        impl ToOpenApiSchema for SchemaA {
            fn schema() -> (std::borrow::Cow<'static, str>, serde_json::Value) {
                ("SchemaA".into(), serde_json::json!({ "type": "string" }))
            }
        }

        impl ToOpenApiSchema for SchemaB {
            fn schema() -> (std::borrow::Cow<'static, str>, serde_json::Value) {
                ("SchemaB".into(), serde_json::json!({ "type": "integer" }))
            }
        }

        let spec = OpenApiSpec::new("Test API", "1.0.0")
            .register_native::<SchemaA>()
            .register_native::<SchemaB>();

        let json = spec.to_json();

        // Verify both schemas were registered
        assert!(json["components"]["schemas"]["SchemaA"].is_object());
        assert!(json["components"]["schemas"]["SchemaB"].is_object());
        assert_eq!(json["components"]["schemas"]["SchemaA"]["type"], "string");
        assert_eq!(json["components"]["schemas"]["SchemaB"]["type"], "integer");
    }
}
