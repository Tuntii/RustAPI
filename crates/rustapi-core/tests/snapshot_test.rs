use rustapi_core::{get, RustApi};
use rustapi_openapi::Schema;
use serde_json::json;

#[derive(Schema)]
#[allow(dead_code)]
struct SnapshotUser {
    id: i64,
    username: String,
}

#[tokio::test]
async fn test_openapi_snapshot() {
    // 1. Setup App
    let app = RustApi::new()
        .openapi_info("Snapshot API", "1.0.0", Some("Test Description"))
        .register_schema::<SnapshotUser>()
        .route("/users", get(|| async { "users" }))
        .route("/users/{id}", get(|| async { "user" }));

    // 2. Generate Spec
    let spec = app.openapi_spec();
    let json = spec.to_json();

    // 3. Normalize/Pretty Print
    let output = serde_json::to_string_pretty(&json).expect("Failed to serialize");

    // 4. Expected Snapshot
    let expected = json!({
      "openapi": "3.1.0",
      "info": {
        "title": "Snapshot API",
        "version": "1.0.0",
        "description": "Test Description"
      },
      "jsonSchemaDialect": "https://json-schema.org/draft/2020-12/schema",
      "paths": {
        "/users": {
          "get": {
            "responses": {
              "200": {
                "description": "Successful response",
                "content": {
                  "text/plain": {
                    "schema": {
                      "type": "string"
                    }
                  }
                }
              }
            }
          }
        },
        "/users/{id}": {
          "get": {
            "parameters": [
              {
                "name": "id",
                "in": "path",
                "required": true,
                "schema": {
                  "type": "string"
                }
              }
            ],
            "responses": {
              "200": {
                "description": "Successful response",
                "content": {
                  "text/plain": {
                    "schema": {
                      "type": "string"
                    }
                  }
                }
              }
            }
          }
        }
      },
      "components": {
        "schemas": {
          "ErrorBodySchema": {
            "type": "object",
            "properties": {
              "error_type": {
                "type": "string"
              },
              "fields": {
                "type": [
                  "array",
                  "null"
                ],
                "items": {
                  "$ref": "#/components/schemas/FieldErrorSchema"
                }
              },
              "message": {
                "type": "string"
              }
            },
            "required": [
              "error_type",
              "message"
            ]
          },
          "ErrorSchema": {
            "type": "object",
            "properties": {
              "error": {
                "$ref": "#/components/schemas/ErrorBodySchema"
              },
              "request_id": {
                "type": [
                  "string",
                  "null"
                ]
              }
            },
            "required": [
              "error"
            ]
          },
          "FieldErrorSchema": {
            "type": "object",
            "properties": {
              "code": {
                "type": "string"
              },
              "field": {
                "type": "string"
              },
              "message": {
                "type": "string"
              }
            },
            "required": [
              "field",
              "code",
              "message"
            ]
          },
          "SnapshotUser": {
            "type": "object",
            "properties": {
              "id": {
                "type": "integer",
                "format": "int64"
              },
              "username": {
                "type": "string"
              }
            },
            "required": [
              "id",
              "username"
            ]
          },
          "ValidationErrorBodySchema": {
            "type": "object",
            "properties": {
              "error_type": {
                "type": "string"
              },
              "fields": {
                "type": "array",
                "items": {
                  "$ref": "#/components/schemas/FieldErrorSchema"
                }
              },
              "message": {
                "type": "string"
              }
            },
            "required": [
              "error_type",
              "message",
              "fields"
            ]
          },
          "ValidationErrorSchema": {
            "type": "object",
            "properties": {
              "error": {
                "$ref": "#/components/schemas/ValidationErrorBodySchema"
              }
            },
            "required": [
              "error"
            ]
          }
        }
      }
    });

    // Assert structural equality first (better error messages)
    assert_eq!(json, expected, "OpenAPI snapshot mismatch (structural)");

    // Assert string equality (ensures serialization determinism)
    let expected_str = serde_json::to_string_pretty(&expected).unwrap();
    assert_eq!(
        output, expected_str,
        "OpenAPI snapshot mismatch! output:\n{}\nexpected:\n{}",
        output, expected_str
    );

    // Also ensure determinism: generate again and match
    let json2 = app.openapi_spec().to_json();
    let output2 = serde_json::to_string_pretty(&json2).unwrap();
    assert_eq!(output, output2, "Nondeterministic output detected!");
}
