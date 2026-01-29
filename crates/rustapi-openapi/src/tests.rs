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
}
