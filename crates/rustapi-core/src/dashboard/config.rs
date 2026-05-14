//! Configuration for the embedded dashboard.

/// Configuration for the embedded isometric system dashboard.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_core::dashboard::DashboardConfig;
///
/// let config = DashboardConfig::new()
///     .admin_token("my-secret")
///     .path("/__rustapi/dashboard")
///     .replay_api_path("/__rustapi/replays")
///     .title("My API Dashboard");
/// ```
#[derive(Debug, Clone)]
pub struct DashboardConfig {
    /// Bearer token required for the JSON API endpoints.
    /// `None` means open access (suitable for dev environments).
    pub admin_token: Option<String>,

    /// URL prefix for the dashboard. Default: `"/__rustapi/dashboard"`.
    pub path: String,

    /// Page title shown in the UI.
    pub title: String,

    /// Replay admin API path used by the dashboard replay browser.
    /// Default: `"/__rustapi/replays"`.
    pub replay_api_path: String,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl DashboardConfig {
    /// Create a dashboard configuration with secure defaults.
    pub fn new() -> Self {
        Self {
            admin_token: None,
            path: "/__rustapi/dashboard".to_string(),
            title: "RustAPI System Dashboard".to_string(),
            replay_api_path: "/__rustapi/replays".to_string(),
        }
    }

    /// Set the admin bearer token.
    ///
    /// When set, all `/__rustapi/dashboard/api/*` endpoints require
    /// `Authorization: Bearer <token>`.
    pub fn admin_token(mut self, token: impl Into<String>) -> Self {
        self.admin_token = Some(token.into());
        self
    }

    /// Override the URL prefix for the dashboard.
    ///
    /// Must start with `/`. Default: `"/__rustapi/dashboard"`.
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Set the page title shown in the browser and the dashboard header.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Override the replay admin API path used by the UI replay browser.
    ///
    /// This should match `ReplayConfig::admin_route_prefix(...)` when replay is enabled.
    pub fn replay_api_path(mut self, path: impl Into<String>) -> Self {
        self.replay_api_path = path.into();
        self
    }
}
