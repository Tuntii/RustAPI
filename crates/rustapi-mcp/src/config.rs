//! Configuration for the MCP server / integration.

use std::collections::HashSet;

/// Configuration for the native MCP server.
///
/// This is the primary way users control what gets exposed as tools,
/// authentication for MCP clients, transport behavior, etc.
#[derive(Debug, Clone)]
pub struct McpConfig {
    /// Human-friendly name of this MCP server (shown to agents).
    pub name: String,
    /// Version string.
    pub version: String,
    /// Optional description.
    pub description: Option<String>,

    /// Whether tool discovery and calling is enabled.
    pub tools_enabled: bool,

    /// Explicitly allowed tags. Only routes that have at least one of these tags
    /// (via OpenAPI `tags` or future route metadata) will be exposed as tools.
    ///
    /// Empty set + no other allow rules = nothing is exposed (safe default).
    pub allowed_tags: HashSet<String>,

    /// Explicit path prefixes that are allowed to become tools.
    /// Example: `["/api/public", "/agent"]`
    pub allowed_path_prefixes: Vec<String>,

    /// Admin / MCP client token.
    ///
    /// When set, MCP clients must present this (via header or query param,
    /// transport dependent) to use discovery or invocation.
    pub admin_token: Option<String>,

    /// Whether to include detailed error information in tool responses.
    /// In production you usually want this `false` (similar to RUSTAPI_ENV=production).
    pub expose_detailed_errors: bool,

    /// Maximum number of tools to advertise in one `tools/list` response.
    /// Helps protect against very large route sets.
    pub max_tools: usize,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            name: "rustapi-mcp".to_string(),
            version: "0.0.0".to_string(),
            description: None,
            tools_enabled: true,
            allowed_tags: HashSet::new(),
            allowed_path_prefixes: vec![],
            admin_token: None,
            expose_detailed_errors: false,
            max_tools: 256,
        }
    }
}

impl McpConfig {
    /// Create a new config with reasonable defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the name advertised to MCP clients.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the version advertised to MCP clients.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set a human description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Enable or disable the tools capability entirely.
    pub fn enable_tools(mut self, enabled: bool) -> Self {
        self.tools_enabled = enabled;
        self
    }

    /// Allow tools only for routes that carry at least one of the given tags.
    ///
    /// This is the recommended way to safely expose a curated surface to agents.
    pub fn allowed_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allowed_tags = tags.into_iter().map(Into::into).collect();
        self
    }

    /// Add a path prefix that is allowed to be exposed as tools.
    pub fn allow_path_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.allowed_path_prefixes.push(prefix.into());
        self
    }

    /// Require this token for MCP clients (discovery + calls).
    pub fn admin_token(mut self, token: impl Into<String>) -> Self {
        self.admin_token = Some(token.into());
        self
    }

    /// Control whether tool responses include full internal error details.
    pub fn expose_detailed_errors(mut self, expose: bool) -> Self {
        self.expose_detailed_errors = expose;
        self
    }

    /// Set the maximum number of tools to list.
    pub fn max_tools(mut self, max: usize) -> Self {
        self.max_tools = max;
        self
    }
}
