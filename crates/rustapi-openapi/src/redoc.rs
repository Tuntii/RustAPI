//! ReDoc UI HTML generation
//!
//! ReDoc is a modern, three-panel API documentation renderer.
//! It provides a clean, responsive interface for OpenAPI specifications.
//!
//! # Features
//! - Three-panel layout (navigation, content, code samples)
//! - Built-in search
//! - Markdown rendering
//! - Dark mode support
//! - Responsive design

/// Generate ReDoc HTML page
///
/// # Arguments
/// * `openapi_url` - URL to the OpenAPI JSON specification
/// * `title` - Optional custom title for the documentation page
///
/// # Example
/// ```rust,ignore
/// let html = generate_redoc_html("/openapi.json", Some("My API Docs"));
/// ```
pub fn generate_redoc_html(openapi_url: &str, title: Option<&str>) -> String {
    let page_title = title.unwrap_or("API Documentation - RustAPI");
    
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{title}</title>
    <meta name="description" content="API Documentation powered by RustAPI and ReDoc">
    <link href="https://fonts.googleapis.com/css?family=Montserrat:300,400,700|Roboto:300,400,700" rel="stylesheet">
    <style>
        body {{
            margin: 0;
            padding: 0;
        }}
        /* Custom styles for RustAPI branding */
        .menu-content {{
            background-color: #1a1a2e !important;
        }}
        .api-content {{
            background-color: #16213e !important;
        }}
    </style>
</head>
<body>
    <redoc 
        spec-url='{openapi_url}'
        expand-responses="200,201"
        hide-hostname
        theme='{{"colors":{{"primary":{{"main":"#e94560"}}}}}}'
    ></redoc>
    <script src="https://cdn.redoc.ly/redoc/latest/bundles/redoc.standalone.js"></script>
</body>
</html>"#,
        title = page_title,
        openapi_url = openapi_url
    )
}

/// ReDoc configuration options
#[derive(Debug, Clone, Default)]
pub struct RedocConfig {
    /// Hide the hostname from the server URL
    pub hide_hostname: bool,
    /// Expand responses by status code (e.g., "200,201")
    pub expand_responses: Option<String>,
    /// Enable native scrolling instead of perfect-scrollbar
    pub native_scrollbars: bool,
    /// Disable search functionality
    pub disable_search: bool,
    /// Hide the download button
    pub hide_download_button: bool,
    /// Hide the loading animation
    pub hide_loading: bool,
    /// Path prefix (for nested deployments)
    pub path_prefix: Option<String>,
    /// Custom theme colors
    pub theme: Option<RedocTheme>,
}

/// ReDoc theme configuration
#[derive(Debug, Clone)]
pub struct RedocTheme {
    /// Primary color (hex)
    pub primary_color: String,
    /// Success color for 2xx responses
    pub success_color: Option<String>,
    /// Warning color for 4xx responses
    pub warning_color: Option<String>,
    /// Error color for 5xx responses
    pub error_color: Option<String>,
}

impl Default for RedocTheme {
    fn default() -> Self {
        Self {
            primary_color: "#e94560".to_string(),
            success_color: Some("#00c853".to_string()),
            warning_color: Some("#ff9800".to_string()),
            error_color: Some("#f44336".to_string()),
        }
    }
}

impl RedocConfig {
    /// Create a new ReDoc configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Hide the hostname from server URLs
    pub fn hide_hostname(mut self) -> Self {
        self.hide_hostname = true;
        self
    }

    /// Set which response codes to expand by default
    pub fn expand_responses(mut self, codes: &str) -> Self {
        self.expand_responses = Some(codes.to_string());
        self
    }

    /// Use native scrollbars
    pub fn native_scrollbars(mut self) -> Self {
        self.native_scrollbars = true;
        self
    }

    /// Disable search
    pub fn disable_search(mut self) -> Self {
        self.disable_search = true;
        self
    }

    /// Set custom theme
    pub fn theme(mut self, theme: RedocTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Generate the HTML attributes for the redoc element
    fn to_attributes(&self) -> String {
        let mut attrs = Vec::new();

        if self.hide_hostname {
            attrs.push("hide-hostname".to_string());
        }
        if let Some(ref codes) = self.expand_responses {
            attrs.push(format!("expand-responses=\"{}\"", codes));
        }
        if self.native_scrollbars {
            attrs.push("native-scrollbars".to_string());
        }
        if self.disable_search {
            attrs.push("disable-search".to_string());
        }
        if self.hide_download_button {
            attrs.push("hide-download-button".to_string());
        }
        if self.hide_loading {
            attrs.push("hide-loading".to_string());
        }
        if let Some(ref prefix) = self.path_prefix {
            attrs.push(format!("path-in-middle-panel=\"{}\"", prefix));
        }
        if let Some(ref theme) = self.theme {
            let theme_json = format!(
                r#"{{"colors":{{"primary":{{"main":"{}"}}}}}}"#,
                theme.primary_color
            );
            attrs.push(format!("theme='{}'", theme_json));
        }

        attrs.join(" ")
    }
}

/// Generate ReDoc HTML with custom configuration
pub fn generate_redoc_html_with_config(
    openapi_url: &str,
    title: Option<&str>,
    config: &RedocConfig,
) -> String {
    let page_title = title.unwrap_or("API Documentation - RustAPI");
    let attributes = config.to_attributes();

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{title}</title>
    <meta name="description" content="API Documentation powered by RustAPI and ReDoc">
    <link href="https://fonts.googleapis.com/css?family=Montserrat:300,400,700|Roboto:300,400,700" rel="stylesheet">
    <style>
        body {{
            margin: 0;
            padding: 0;
        }}
    </style>
</head>
<body>
    <redoc spec-url='{openapi_url}' {attributes}></redoc>
    <script src="https://cdn.redoc.ly/redoc/latest/bundles/redoc.standalone.js"></script>
</body>
</html>"#,
        title = page_title,
        openapi_url = openapi_url,
        attributes = attributes
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_redoc_html() {
        let html = generate_redoc_html("/openapi.json", None);
        assert!(html.contains("redoc"));
        assert!(html.contains("/openapi.json"));
        assert!(html.contains("API Documentation - RustAPI"));
    }

    #[test]
    fn test_generate_redoc_html_custom_title() {
        let html = generate_redoc_html("/api/spec.json", Some("My Custom API"));
        assert!(html.contains("My Custom API"));
        assert!(html.contains("/api/spec.json"));
    }

    #[test]
    fn test_redoc_config() {
        let config = RedocConfig::new()
            .hide_hostname()
            .expand_responses("200,201")
            .native_scrollbars();

        let html = generate_redoc_html_with_config("/openapi.json", None, &config);
        assert!(html.contains("hide-hostname"));
        assert!(html.contains("expand-responses=\"200,201\""));
        assert!(html.contains("native-scrollbars"));
    }

    #[test]
    fn test_redoc_theme() {
        let theme = RedocTheme {
            primary_color: "#ff0000".to_string(),
            ..Default::default()
        };
        let config = RedocConfig::new().theme(theme);
        let html = generate_redoc_html_with_config("/openapi.json", None, &config);
        assert!(html.contains("#ff0000"));
    }
}
