//! Tool discovery: converting RustAPI OpenAPI metadata into MCP tools.
//!
//! This module walks the OpenAPI spec produced by RustApi and turns
//! HTTP operations into MCP `McpTool` definitions, applying exposure
//! filters from `McpConfig`.

use crate::config::McpConfig;
use crate::types::McpTool;
use rustapi_openapi::{Components, OpenApiSpec, Operation, Parameter, RequestBody, SchemaRef};
use std::collections::BTreeMap;

/// Main entry point: extract a filtered list of MCP tools from an OpenAPI spec.
pub fn extract_tools_from_spec(spec: &OpenApiSpec, config: &McpConfig) -> Vec<McpTool> {
    if !config.tools_enabled {
        return vec![];
    }

    let mut tools = Vec::new();
    let components = spec.components.as_ref();

    for (path, path_item) in &spec.paths {
        // Apply path prefix filter if configured
        if !path_matches_prefixes(path, &config.allowed_path_prefixes) {
            continue;
        }

        // Check each HTTP method
        if let Some(op) = &path_item.get {
            if let Some(tool) = operation_to_tool("GET", path, op, components, config) {
                tools.push(tool);
            }
        }
        if let Some(op) = &path_item.post {
            if let Some(tool) = operation_to_tool("POST", path, op, components, config) {
                tools.push(tool);
            }
        }
        if let Some(op) = &path_item.put {
            if let Some(tool) = operation_to_tool("PUT", path, op, components, config) {
                tools.push(tool);
            }
        }
        if let Some(op) = &path_item.patch {
            if let Some(tool) = operation_to_tool("PATCH", path, op, components, config) {
                tools.push(tool);
            }
        }
        if let Some(op) = &path_item.delete {
            if let Some(tool) = operation_to_tool("DELETE", path, op, components, config) {
                tools.push(tool);
            }
        }

        // We can add more methods later if needed (HEAD, OPTIONS...)

        if tools.len() >= config.max_tools {
            break;
        }
    }

    // Enforce max_tools
    if tools.len() > config.max_tools {
        tools.truncate(config.max_tools);
    }

    tools
}

fn path_matches_prefixes(path: &str, prefixes: &[String]) -> bool {
    if prefixes.is_empty() {
        return true;
    }
    prefixes.iter().any(|p| path.starts_with(p))
}

fn operation_to_tool(
    method: &str,
    path: &str,
    op: &Operation,
    components: Option<&Components>,
    config: &McpConfig,
) -> Option<McpTool> {
    // Tag filtering (if allowed_tags configured, operation must have at least one matching tag)
    if !config.allowed_tags.is_empty() {
        let has_match = op.tags.iter().any(|t| config.allowed_tags.contains(t));
        if !has_match {
            return None;
        }
    }

    let name = generate_tool_name(method, path, op);
    let description = op
        .summary
        .clone()
        .or_else(|| op.description.clone());

    let input_schema = build_input_schema(op, components);

    Some(McpTool {
        name,
        description,
        input_schema,
        output_schema: None, // Future: extract from success responses
        tags: op.tags.clone(),
    })
}

/// Generate a stable, agent-friendly tool name.
fn generate_tool_name(method: &str, path: &str, op: &Operation) -> String {
    if let Some(oid) = &op.operation_id {
        return sanitize_name(oid);
    }

    // Fallback: method + sanitized path
    let mut slug = path
        .trim_start_matches('/')
        .replace(['/', '{', '}', ':'], "_")
        .replace(['-', '.', ' '], "_");

    // Collapse multiple underscores
    while slug.contains("__") {
        slug = slug.replace("__", "_");
    }

    let slug = slug.trim_matches('_').to_string();
    let method_lower = method.to_lowercase();

    if slug.is_empty() {
        method_lower
    } else {
        format!("{}_{}", method_lower, slug)
    }
}

fn sanitize_name(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
        .to_lowercase()
}

/// Build a JSON Schema for the tool input.
///
/// Strategy (MVP):
/// - If there is a JSON request body, use its schema (attempt simple $ref resolution).
/// - Otherwise, synthesize an object schema from the operation's parameters.
fn build_input_schema(op: &Operation, components: Option<&Components>) -> serde_json::Value {
    // 1. Try request body first (most common for "tool call with data")
    if let Some(body) = &op.request_body {
        if let Some(schema_val) = extract_json_schema_from_body(body, components) {
            return schema_val;
        }
    }

    // 2. Fallback: build from parameters (path + query + header)
    build_schema_from_parameters(&op.parameters, components)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustapi_openapi::{OpenApiSpec, Operation};

    fn make_minimal_spec() -> OpenApiSpec {
        let mut spec = OpenApiSpec::new("Test API", "1.0.0");

        // Public tool
        let mut get_user = Operation::new();
        get_user.summary = Some("Get user by ID".to_string());
        get_user.tags = vec!["users".to_string(), "public".to_string()];
        get_user.operation_id = Some("getUser".to_string());

        // Body tool
        let mut create_user = Operation::new();
        create_user.summary = Some("Create a user".to_string());
        create_user.tags = vec!["users".to_string()];
        create_user.operation_id = Some("createUser".to_string());

        // Internal only (no public tag)
        let mut admin = Operation::new();
        admin.summary = Some("Admin only".to_string());
        admin.tags = vec!["admin".to_string()];

        spec = spec
            .path("/users/{id}", "GET", get_user)
            .path("/users", "POST", create_user)
            .path("/admin/users", "GET", admin);

        spec
    }

    #[test]
    fn extracts_tools_with_operation_id_as_name() {
        let spec = make_minimal_spec();
        let config = McpConfig::new();

        let tools = extract_tools_from_spec(&spec, &config);
        assert!(!tools.is_empty());

        let names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"getuser"));
        assert!(names.contains(&"createuser"));
    }

    #[test]
    fn respects_allowed_tags_filter() {
        let spec = make_minimal_spec();

        let config = McpConfig::new().allowed_tags(["public"]);

        let tools = extract_tools_from_spec(&spec, &config);
        let _tags: Vec<Vec<String>> = tools.iter().map(|t| t.tags.clone()).collect();

        // Only the GET /users/{id} should survive (has "public")
        assert_eq!(tools.len(), 1);
        assert!(tools[0].name.contains("getuser") || tools[0].tags.contains(&"public".to_string()));
    }

    #[test]
    fn respects_path_prefix_filter() {
        let spec = make_minimal_spec();

        let config = McpConfig::new().allow_path_prefix("/users");

        let tools = extract_tools_from_spec(&spec, &config);
        // admin path should be filtered out
        assert!(tools.iter().all(|t| !t.name.contains("admin")));
    }

    #[test]
    fn max_tools_limit_is_respected() {
        let spec = make_minimal_spec();
        let config = McpConfig::new().max_tools(1);

        let tools = extract_tools_from_spec(&spec, &config);
        assert!(tools.len() <= 1);
    }
}

fn extract_json_schema_from_body(
    body: &RequestBody,
    components: Option<&Components>,
) -> Option<serde_json::Value> {
    // Prefer application/json
    let media = body
        .content
        .get("application/json")
        .or_else(|| body.content.values().next())?;

    if let Some(schema_ref) = &media.schema {
        return Some(schema_ref_to_json(schema_ref, components));
    }
    None
}

fn build_schema_from_parameters(
    params: &[Parameter],
    components: Option<&Components>,
) -> serde_json::Value {
    if params.is_empty() {
        return serde_json::json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        });
    }

    let mut properties = BTreeMap::new();
    let mut required = Vec::new();

    for param in params {
        let name = param.name.clone();
        let schema = if let Some(s) = &param.schema {
            schema_ref_to_json(s, components)
        } else {
            // Default to string if no schema
            serde_json::json!({"type": "string"})
        };

        if param.required {
            required.push(name.clone());
        }

        properties.insert(name, schema);
    }

    let mut schema = serde_json::json!({
        "type": "object",
        "properties": properties,
    });

    if !required.is_empty() {
        schema["required"] = serde_json::to_value(required).unwrap();
    }
    schema["additionalProperties"] = serde_json::json!(false);

    schema
}

/// Convert a SchemaRef into a plain JSON value suitable for MCP tool inputSchema.
/// For $ref we attempt a shallow resolution from components.schemas when available.
fn schema_ref_to_json(
    schema_ref: &SchemaRef,
    components: Option<&Components>,
) -> serde_json::Value {
    match schema_ref {
        SchemaRef::Ref { reference } => {
            // Try to resolve simple "#/components/schemas/Name"
            if let Some(name) = reference.strip_prefix("#/components/schemas/") {
                if let Some(components) = components {
                    if let Some(schema) = components.schemas.get(name) {
                        // Serialize the JsonSchema2020 as value (it will be a valid schema)
                        return serde_json::to_value(schema).unwrap_or_else(|_| {
                            serde_json::json!({ "$ref": reference })
                        });
                    }
                }
            }
            // Can't resolve — emit the ref (MCP clients / LLMs can sometimes handle it, or we improve later)
            serde_json::json!({ "$ref": reference })
        }
        SchemaRef::Schema(boxed) => {
            serde_json::to_value(boxed.as_ref()).unwrap_or(serde_json::json!({}))
        }
        SchemaRef::Inline(val) => val.clone(),
    }
}
