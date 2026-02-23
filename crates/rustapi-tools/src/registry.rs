use crate::{FunctionDefinition, Tool, ToolError, ToolOutput};
use rustapi_context::RequestContext;
use std::collections::HashMap;
use std::sync::Arc;

/// Runtime registry for discovering and invoking tools.
///
/// Tools are registered by name and can be looked up for execution or
/// converted to LLM function definitions for function-calling APIs.
///
/// The registry is `Clone`-able (cheap, Arc-backed) and thread-safe.
#[derive(Clone)]
pub struct ToolRegistry {
    tools: Arc<HashMap<String, Arc<dyn Tool>>>,
}

impl ToolRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            tools: Arc::new(HashMap::new()),
        }
    }

    /// Register a tool. Overwrites any existing tool with the same name.
    pub fn register<T: Tool>(mut self, tool: T) -> Self {
        let name = tool.name().to_string();
        let map = Arc::make_mut(&mut self.tools);
        map.insert(name, Arc::new(tool));
        self
    }

    /// Register a pre-wrapped Arc<dyn Tool>.
    pub fn register_arc(mut self, tool: Arc<dyn Tool>) -> Self {
        let name = tool.name().to_string();
        let map = Arc::make_mut(&mut self.tools);
        map.insert(name, tool);
        self
    }

    /// Look up a tool by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// Check whether a tool is registered.
    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// List all registered tool names.
    pub fn names(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }

    /// Number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Execute a tool by name.
    pub async fn execute(
        &self,
        name: &str,
        ctx: &RequestContext,
        input: serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let tool = self
            .get(name)
            .ok_or_else(|| ToolError::not_found(name))?;

        tracing::debug!(tool = name, "Executing tool");
        let result = tool.execute(ctx, input).await;

        match &result {
            Ok(output) => {
                // Record cost if present.
                if let Some(ref cost) = output.cost {
                    let _ = ctx.cost().record(cost);
                }
                tracing::debug!(tool = name, "Tool executed successfully");
            }
            Err(e) => {
                tracing::warn!(tool = name, error = %e, "Tool execution failed");
            }
        }

        result
    }

    /// Convert all registered tools to LLM function definitions.
    pub fn to_function_definitions(&self) -> Vec<FunctionDefinition> {
        self.tools
            .values()
            .map(|t| t.to_function_definition())
            .collect()
    }

    /// Get descriptions for all tools (useful for system prompts).
    pub fn describe_all(&self) -> Vec<ToolDescription> {
        self.tools
            .values()
            .map(|t| ToolDescription {
                name: t.name().to_string(),
                description: t.description().to_string(),
            })
            .collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("tools", &self.names())
            .finish()
    }
}

/// Summary of a tool — name + description.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolDescription {
    pub name: String,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolOutput;
    use async_trait::async_trait;
    use rustapi_context::RequestContextBuilder;

    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn description(&self) -> &str {
            "Echoes the input"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object", "properties": {"text": {"type": "string"}}})
        }
        async fn execute(
            &self,
            _ctx: &RequestContext,
            input: serde_json::Value,
        ) -> Result<ToolOutput, ToolError> {
            Ok(ToolOutput::value(input))
        }
    }

    #[test]
    fn test_registry_register_and_get() {
        let reg = ToolRegistry::new().register(EchoTool);
        assert!(reg.contains("echo"));
        assert_eq!(reg.len(), 1);
        assert!(!reg.is_empty());
    }

    #[tokio::test]
    async fn test_registry_execute() {
        let reg = ToolRegistry::new().register(EchoTool);
        let ctx = RequestContextBuilder::new().build();
        let output = reg
            .execute("echo", &ctx, serde_json::json!({"text": "hello"}))
            .await
            .unwrap();
        assert_eq!(output.value, serde_json::json!({"text": "hello"}));
    }

    #[tokio::test]
    async fn test_registry_not_found() {
        let reg = ToolRegistry::new();
        let ctx = RequestContextBuilder::new().build();
        let result = reg
            .execute("nonexistent", &ctx, serde_json::json!({}))
            .await;
        assert!(matches!(result, Err(ToolError::NotFound { .. })));
    }

    #[test]
    fn test_function_definitions() {
        let reg = ToolRegistry::new().register(EchoTool);
        let defs = reg.to_function_definitions();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].name, "echo");
    }
}
