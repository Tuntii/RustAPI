//! ReDoc UI HTML generation
//!
//! ReDoc is a modern, three-panel API documentation renderer.

/// Generate ReDoc HTML page
pub fn generate_redoc_html(openapi_url: &str, title: Option<&str>) -> String {
    let page_title = title.unwrap_or("API Documentation - RustAPI");

    let mut html = String::with_capacity(2000);
    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("    <meta charset=\"utf-8\"/>\n");
    html.push_str("    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    html.push_str("    <title>");
    html.push_str(page_title);
    html.push_str("</title>\n");
    html.push_str("    <link href=\"https://fonts.googleapis.com/css?family=Montserrat:300,400,700|Roboto:300,400,700\" rel=\"stylesheet\">\n");
    html.push_str("    <style>body { margin: 0; padding: 0; }</style>\n");
    html.push_str("</head>\n<body>\n");
    html.push_str("    <redoc spec-url='");
    html.push_str(openapi_url);
    html.push_str("' expand-responses=\"200,201\" hide-hostname></redoc>\n");
    html.push_str("    <script src=\"https://cdn.redoc.ly/redoc/latest/bundles/redoc.standalone.js\"></script>\n");
    html.push_str("</body>\n</html>");

    html
}

/// ReDoc configuration options
#[derive(Debug, Clone, Default)]
pub struct RedocConfig {
    /// Hide the hostname from the server URL
    pub hide_hostname: bool,
    /// Expand responses by status code
    pub expand_responses: Option<String>,
    /// Enable native scrolling
    pub native_scrollbars: bool,
    /// Disable search functionality
    pub disable_search: bool,
    /// Hide the download button
    pub hide_download_button: bool,
    /// Custom theme primary color
    pub primary_color: Option<String>,
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

    /// Set primary theme color
    pub fn primary_color(mut self, color: &str) -> Self {
        self.primary_color = Some(color.to_string());
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

        attrs.join(" ")
    }
}

/// ReDoc theme configuration  
#[derive(Debug, Clone)]
pub struct RedocTheme {
    /// Primary color (hex)
    pub primary_color: String,
}

impl Default for RedocTheme {
    fn default() -> Self {
        Self {
            primary_color: "#e94560".to_string(),
        }
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

    let mut html = String::with_capacity(2000);
    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("    <meta charset=\"utf-8\"/>\n");
    html.push_str("    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    html.push_str("    <title>");
    html.push_str(page_title);
    html.push_str("</title>\n");
    html.push_str("    <link href=\"https://fonts.googleapis.com/css?family=Montserrat:300,400,700|Roboto:300,400,700\" rel=\"stylesheet\">\n");
    html.push_str("    <style>body { margin: 0; padding: 0; }</style>\n");
    html.push_str("</head>\n<body>\n");
    html.push_str("    <redoc spec-url='");
    html.push_str(openapi_url);
    html.push_str("' ");
    html.push_str(&attributes);
    html.push_str("></redoc>\n");
    html.push_str("    <script src=\"https://cdn.redoc.ly/redoc/latest/bundles/redoc.standalone.js\"></script>\n");
    html.push_str("</body>\n</html>");

    html
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
        assert!(html.contains("native-scrollbars"));
    }
}
