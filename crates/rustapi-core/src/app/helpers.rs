use std::collections::BTreeMap;
#[cfg(feature = "dashboard")]
use std::collections::BTreeSet;

#[cfg(feature = "dashboard")]
pub(super) fn openapi_tags_for_route(
    spec: &rustapi_openapi::OpenApiSpec,
    path: &str,
    methods: &[http::Method],
) -> Vec<String> {
    let Some(path_item) = spec.paths.get(path) else {
        return Vec::new();
    };

    let mut tags = BTreeSet::new();
    for method in methods {
        if let Some(operation) = operation_for_method(path_item, method) {
            tags.extend(operation.tags.iter().cloned());
        }
    }

    tags.into_iter().collect()
}

#[cfg(feature = "dashboard")]
pub(super) fn operation_for_method<'a>(
    path_item: &'a rustapi_openapi::PathItem,
    method: &http::Method,
) -> Option<&'a rustapi_openapi::Operation> {
    match *method {
        http::Method::GET => path_item.get.as_ref(),
        http::Method::POST => path_item.post.as_ref(),
        http::Method::PUT => path_item.put.as_ref(),
        http::Method::PATCH => path_item.patch.as_ref(),
        http::Method::DELETE => path_item.delete.as_ref(),
        http::Method::HEAD => path_item.head.as_ref(),
        http::Method::OPTIONS => path_item.options.as_ref(),
        http::Method::TRACE => path_item.trace.as_ref(),
        _ => None,
    }
}

#[cfg(feature = "dashboard")]
pub(super) fn infer_route_feature_gates(path: &str) -> Vec<String> {
    if path.contains("openapi") || path.contains("docs") {
        vec!["core-openapi".to_string()]
    } else if path.starts_with("/__rustapi/replays") {
        vec!["extras-replay".to_string()]
    } else {
        Vec::new()
    }
}

#[cfg(feature = "dashboard")]
pub(super) fn is_dashboard_replay_eligible(path: &str, health_eligible: bool) -> bool {
    !health_eligible && !path.starts_with("/__rustapi/")
}

pub(super) fn add_path_params_to_operation(
    path: &str,
    op: &mut rustapi_openapi::Operation,
    param_schemas: &BTreeMap<String, String>,
) {
    let mut params: Vec<String> = Vec::new();
    let mut in_brace = false;
    let mut current = String::new();

    for ch in path.chars() {
        match ch {
            '{' => {
                in_brace = true;
                current.clear();
            }
            '}' => {
                if in_brace {
                    in_brace = false;
                    if !current.is_empty() {
                        params.push(current.clone());
                    }
                }
            }
            _ => {
                if in_brace {
                    current.push(ch);
                }
            }
        }
    }

    if params.is_empty() {
        return;
    }

    let op_params = &mut op.parameters;

    for name in params {
        let already = op_params
            .iter()
            .any(|p| p.location == "path" && p.name == name);
        if already {
            continue;
        }

        // Use custom schema if provided, otherwise infer from name
        let schema = if let Some(schema_type) = param_schemas.get(&name) {
            schema_type_to_openapi_schema(schema_type)
        } else {
            infer_path_param_schema(&name)
        };

        op_params.push(rustapi_openapi::Parameter {
            name,
            location: "path".to_string(),
            required: true,
            description: None,
            deprecated: None,
            schema: Some(schema),
        });
    }
}

/// Convert a schema type string to an OpenAPI schema reference
pub(super) fn schema_type_to_openapi_schema(schema_type: &str) -> rustapi_openapi::SchemaRef {
    match schema_type.to_lowercase().as_str() {
        "uuid" => rustapi_openapi::SchemaRef::Inline(serde_json::json!({
            "type": "string",
            "format": "uuid"
        })),
        "integer" | "int" | "int64" | "i64" => {
            rustapi_openapi::SchemaRef::Inline(serde_json::json!({
                "type": "integer",
                "format": "int64"
            }))
        }
        "int32" | "i32" => rustapi_openapi::SchemaRef::Inline(serde_json::json!({
            "type": "integer",
            "format": "int32"
        })),
        "number" | "float" | "f64" | "f32" => {
            rustapi_openapi::SchemaRef::Inline(serde_json::json!({
                "type": "number"
            }))
        }
        "boolean" | "bool" => rustapi_openapi::SchemaRef::Inline(serde_json::json!({
            "type": "boolean"
        })),
        _ => rustapi_openapi::SchemaRef::Inline(serde_json::json!({
            "type": "string"
        })),
    }
}

/// Infer the OpenAPI schema type for a path parameter based on naming conventions.
///
/// Common patterns:
/// - `*_id`, `*Id`, `id` â†’ integer (but NOT *uuid)
/// - `*_count`, `*_num`, `page`, `limit`, `offset` â†’ integer  
/// - `*_uuid`, `uuid` â†’ string with uuid format
/// - `year`, `month`, `day` â†’ integer
/// - Everything else â†’ string
pub(super) fn infer_path_param_schema(name: &str) -> rustapi_openapi::SchemaRef {
    let lower = name.to_lowercase();

    // UUID patterns (check first to avoid false positive from "id" suffix)
    let is_uuid = lower == "uuid" || lower.ends_with("_uuid") || lower.ends_with("uuid");

    if is_uuid {
        return rustapi_openapi::SchemaRef::Inline(serde_json::json!({
            "type": "string",
            "format": "uuid"
        }));
    }

    // Integer patterns
    // Integer patterns
    let is_integer = lower == "page"
        || lower == "limit"
        || lower == "offset"
        || lower == "count"
        || lower.ends_with("_count")
        || lower.ends_with("_num")
        || lower == "year"
        || lower == "month"
        || lower == "day"
        || lower == "index"
        || lower == "position";

    if is_integer {
        rustapi_openapi::SchemaRef::Inline(serde_json::json!({
            "type": "integer",
            "format": "int64"
        }))
    } else {
        rustapi_openapi::SchemaRef::Inline(serde_json::json!({ "type": "string" }))
    }
}

/// Normalize a prefix for OpenAPI paths.
///
/// Ensures the prefix:
/// - Starts with exactly one leading slash
/// - Has no trailing slash (unless it's just "/")
/// - Has no double slashes
pub(super) fn normalize_prefix_for_openapi(prefix: &str) -> String {
    // Handle empty string
    if prefix.is_empty() {
        return "/".to_string();
    }

    // Split by slashes and filter out empty segments (handles multiple slashes)
    let segments: Vec<&str> = prefix.split('/').filter(|s| !s.is_empty()).collect();

    // If no segments after filtering, return root
    if segments.is_empty() {
        return "/".to_string();
    }

    // Build the normalized prefix with leading slash
    let mut result = String::with_capacity(prefix.len() + 1);
    for segment in segments {
        result.push('/');
        result.push_str(segment);
    }

    result
}

/// Check Basic Auth header against expected credentials
#[cfg(feature = "swagger-ui")]
pub(super) fn check_basic_auth(req: &crate::Request, expected: &str) -> bool {
    req.headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|auth| auth == expected)
        .unwrap_or(false)
}

/// Create 401 Unauthorized response with WWW-Authenticate header
#[cfg(feature = "swagger-ui")]
pub(super) fn unauthorized_response() -> crate::Response {
    http::Response::builder()
        .status(http::StatusCode::UNAUTHORIZED)
        .header(
            http::header::WWW_AUTHENTICATE,
            "Basic realm=\"API Documentation\"",
        )
        .header(http::header::CONTENT_TYPE, "text/plain")
        .body(crate::response::Body::from("Unauthorized"))
        .unwrap()
}
