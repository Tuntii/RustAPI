#[cfg(test)]
mod tests {
    use crate::schema::{JsonSchema2020, RustApiSchema, SchemaCtx, SchemaRef};
    use crate::spec::OpenApiSpec;
    use rustapi_macros::Schema;

    #[test]
    fn test_schema_generation_primitive() {
        let mut ctx = SchemaCtx::new();
        let schema = String::schema(&mut ctx);

        match schema {
            SchemaRef::Schema(s) => {
                assert_eq!(
                    s.schema_type,
                    Some(crate::schema::TypeArray::Single("string".to_string()))
                );
            }
            _ => panic!("Expected Schema"),
        }
    }

    #[test]
    fn test_spec_generation() {
        let spec = OpenApiSpec::new("Test", "1.0");
        let json = spec.to_json();
        assert_eq!(json["openapi"], "3.1.0");
        assert_eq!(json["info"]["title"], "Test");
    }

    #[derive(Schema)]
    #[allow(dead_code)]
    struct TestUser {
        id: i64,
        name: String,
        email: Option<String>,
    }

    #[test]
    fn test_derive_schema_struct() {
        let mut ctx = SchemaCtx::new();
        let schema_ref = TestUser::schema(&mut ctx);

        // Should be a reference
        match schema_ref {
            SchemaRef::Ref { reference } => {
                assert_eq!(reference, "#/components/schemas/TestUser");
            }
            _ => panic!("Expected Ref"),
        }

        // Component should be registered
        assert!(ctx.components.contains_key("TestUser"));
        let schema = ctx.components.get("TestUser").unwrap();

        // Verify properties
        assert!(schema.properties.is_some());
        let props = schema.properties.as_ref().unwrap();

        assert!(props.contains_key("id"));
        assert!(props.contains_key("name"));
        assert!(props.contains_key("email"));

        // Check required
        assert!(schema.required.is_some());
        let required = schema.required.as_ref().unwrap();
        assert!(required.contains(&"id".to_string()));
        assert!(required.contains(&"name".to_string()));
        assert!(!required.contains(&"email".to_string())); // Option should not be required
    }

    #[derive(Schema)]
    #[allow(dead_code)]
    enum Status {
        Active,
        Inactive,
    }

    #[test]
    fn test_derive_schema_enum_string() {
        let mut ctx = SchemaCtx::new();
        let _ = Status::schema(&mut ctx);

        let schema = ctx.components.get("Status").unwrap();
        assert_eq!(
            schema.schema_type,
            Some(crate::schema::TypeArray::Single("string".to_string()))
        );
        assert!(schema.enum_values.is_some());
        let enums = schema.enum_values.as_ref().unwrap();
        assert_eq!(enums.len(), 2);
    }

    #[derive(Schema)]
    #[allow(dead_code)]
    enum Event {
        Created { id: i64 },
        Deleted,
    }

    #[test]
    fn test_derive_schema_enum_complex() {
        let mut ctx = SchemaCtx::new();
        let _ = Event::schema(&mut ctx);

        let schema = ctx.components.get("Event").unwrap();
        // Should be oneOf
        assert!(schema.one_of.is_some());
    }

    #[derive(Schema)]
    #[allow(dead_code)]
    struct Wrapper<T: RustApiSchema> {
        value: T,
    }

    #[test]
    fn test_generic_collision() {
        let mut spec = OpenApiSpec::new("Test", "1.0");

        // Register Wrapper<String>
        spec.register_in_place::<Wrapper<String>>();

        // Register Wrapper<i32>
        spec.register_in_place::<Wrapper<i32>>();

        // Check components
        let components = spec.components.as_ref().unwrap();

        let has_string = components
            .schemas
            .keys()
            .any(|k| k.contains("Wrapper") && k.contains("String"));
        // Int32 because i32::name() returns "Int32"
        let has_int32 = components
            .schemas
            .keys()
            .any(|k| k.contains("Wrapper") && k.contains("Int32"));

        // If we only have "Wrapper", this will fail.
        assert!(has_string, "Missing Wrapper_String component");
        assert!(has_int32, "Missing Wrapper_Int32 component");
    }

    struct CollisionA;
    impl RustApiSchema for CollisionA {
        fn schema(ctx: &mut SchemaCtx) -> SchemaRef {
            ctx.components
                .insert("Collision".to_string(), JsonSchema2020::string());
            SchemaRef::Ref {
                reference: "#/components/schemas/Collision".to_string(),
            }
        }
        fn name() -> std::borrow::Cow<'static, str> {
            "Collision".into()
        }
    }

    struct CollisionB;
    impl RustApiSchema for CollisionB {
        fn schema(ctx: &mut SchemaCtx) -> SchemaRef {
            ctx.components
                .insert("Collision".to_string(), JsonSchema2020::integer());
            SchemaRef::Ref {
                reference: "#/components/schemas/Collision".to_string(),
            }
        }
        fn name() -> std::borrow::Cow<'static, str> {
            "Collision".into()
        }
    }

    #[test]
    #[should_panic(expected = "Schema collision detected")]
    fn test_collision_detection() {
        let mut spec = OpenApiSpec::new("Test", "1.0");
        spec.register_in_place::<CollisionA>();
        spec.register_in_place::<CollisionB>();
    }

    #[test]
    fn test_ref_integrity_valid() {
        let mut spec = OpenApiSpec::new("Test", "1.0");
        spec.register_in_place::<TestUser>();

        // TestUser references primitive types which are inline, and itself (registered)
        // Let's add a manual ref to ensure it works
        use crate::spec::Operation;
        use crate::spec::ResponseSpec;

        let mut op = Operation::new();
        op.responses
            .insert("200".to_string(), ResponseSpec::default());

        // This is valid because TestUser is registered
        let param = crate::spec::Parameter {
            name: "user".to_string(),
            location: "query".to_string(),
            required: false,
            description: None,
            deprecated: None,
            schema: Some(SchemaRef::Ref {
                reference: "#/components/schemas/TestUser".to_string(),
            }),
        };
        op.parameters.push(param);

        spec = spec.path("/user", "GET", op);

        assert!(spec.validate_integrity().is_ok());
    }

    #[test]
    fn test_ref_integrity_invalid() {
        let mut spec = OpenApiSpec::new("Test", "1.0");

        use crate::spec::Operation;

        let mut op = Operation::new();
        let param = crate::spec::Parameter {
            name: "user".to_string(),
            location: "query".to_string(),
            required: false,
            description: None,
            deprecated: None,
            schema: Some(SchemaRef::Ref {
                reference: "#/components/schemas/NonExistent".to_string(),
            }),
        };
        op.parameters.push(param);

        spec = spec.path("/user", "GET", op);

        let result = spec.validate_integrity();
        assert!(result.is_err());
        let missing = result.unwrap_err();
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], "#/components/schemas/NonExistent");
    }

    #[test]
    fn test_ref_integrity_components_invalid() {
        let mut spec = OpenApiSpec::new("Test", "1.0");
        let mut components = crate::spec::Components::default();

        components.parameters.insert(
            "badParam".to_string(),
            crate::spec::Parameter {
                name: "badParam".to_string(),
                location: "query".to_string(),
                required: false,
                description: None,
                deprecated: None,
                schema: Some(SchemaRef::Ref {
                    reference: "#/components/schemas/NonExistent".to_string(),
                }),
            },
        );

        spec.components = Some(components);

        let result = spec.validate_integrity();
        assert!(
            result.is_err(),
            "Should detect missing ref in components.parameters"
        );
        let missing = result.unwrap_err();
        assert!(missing.contains(&"#/components/schemas/NonExistent".to_string()));
    }

    #[test]
    fn test_ref_integrity_components_traverses_all_ref_bearing_members() {
        let mut spec = OpenApiSpec::new("Test", "1.0");
        let mut components = crate::spec::Components::default();

        components.responses.insert(
            "badResponse".to_string(),
            crate::spec::ResponseSpec {
                description: "bad".to_string(),
                content: std::collections::BTreeMap::from([(
                    "application/json".to_string(),
                    crate::spec::MediaType {
                        schema: Some(SchemaRef::Ref {
                            reference: "#/components/schemas/MissingFromResponse".to_string(),
                        }),
                        example: None,
                    },
                )]),
                headers: std::collections::BTreeMap::from([(
                    "X-Callback".to_string(),
                    crate::spec::Header {
                        description: None,
                        schema: Some(SchemaRef::Ref {
                            reference: "#/components/schemas/MissingFromResponseHeader".to_string(),
                        }),
                    },
                )]),
            },
        );

        components.request_bodies.insert(
            "badRequestBody".to_string(),
            crate::spec::RequestBody {
                description: None,
                required: Some(true),
                content: std::collections::BTreeMap::from([(
                    "application/json".to_string(),
                    crate::spec::MediaType {
                        schema: Some(SchemaRef::Ref {
                            reference: "#/components/schemas/MissingFromRequestBody".to_string(),
                        }),
                        example: None,
                    },
                )]),
            },
        );

        components.headers.insert(
            "badHeader".to_string(),
            crate::spec::Header {
                description: None,
                schema: Some(SchemaRef::Ref {
                    reference: "#/components/schemas/MissingFromHeader".to_string(),
                }),
            },
        );

        let mut callback_operation = crate::spec::Operation::new();
        callback_operation.request_body = Some(crate::spec::RequestBody {
            description: None,
            required: Some(true),
            content: std::collections::BTreeMap::from([(
                "application/json".to_string(),
                crate::spec::MediaType {
                    schema: Some(SchemaRef::Ref {
                        reference: "#/components/schemas/MissingFromCallback".to_string(),
                    }),
                    example: None,
                },
            )]),
        });

        let mut callback_path = crate::spec::PathItem::default();
        callback_path.post = Some(callback_operation);

        components.callbacks.insert(
            "badCallback".to_string(),
            std::collections::BTreeMap::from([(
                "{$request.body#/callbackUrl}".to_string(),
                callback_path,
            )]),
        );

        spec.components = Some(components);

        let missing = spec
            .validate_integrity()
            .expect_err("should report all missing schema refs");

        assert!(missing.contains(&"#/components/schemas/MissingFromResponse".to_string()));
        assert!(missing.contains(&"#/components/schemas/MissingFromResponseHeader".to_string()));
        assert!(missing.contains(&"#/components/schemas/MissingFromRequestBody".to_string()));
        assert!(missing.contains(&"#/components/schemas/MissingFromHeader".to_string()));
        assert!(missing.contains(&"#/components/schemas/MissingFromCallback".to_string()));
    }
}
