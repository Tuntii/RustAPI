//! Swagger UI HTML generation.

/// Generate Swagger UI HTML page.
///
/// # Arguments
///
/// * `spec_url` - URL to the OpenAPI JSON spec (usually "/openapi.json")
/// * `title` - Page title
///
/// # Example
///
/// ```rust
/// use rustapi_openapi::swagger_ui_html;
///
/// let html = swagger_ui_html("/openapi.json", "My API Docs");
/// assert!(html.contains("swagger-ui"));
/// ```
pub fn swagger_ui_html(spec_url: &str, title: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css">
    <style>
        html {{ box-sizing: border-box; overflow: -moz-scrollbars-vertical; overflow-y: scroll; }}
        *, *:before, *:after {{ box-sizing: inherit; }}
        body {{ margin: 0; background: #fafafa; }}
        .swagger-ui .topbar {{ display: none; }}
    </style>
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
    <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-standalone-preset.js"></script>
    <script>
        window.onload = function() {{
            window.ui = SwaggerUIBundle({{
                url: "{spec_url}",
                dom_id: '#swagger-ui',
                deepLinking: true,
                presets: [
                    SwaggerUIBundle.presets.apis,
                    SwaggerUIStandalonePreset
                ],
                plugins: [
                    SwaggerUIBundle.plugins.DownloadUrl
                ],
                layout: "StandaloneLayout",
                defaultModelsExpandDepth: 1,
                defaultModelExpandDepth: 1,
                docExpansion: "list",
                filter: true,
                showExtensions: true,
                showCommonExtensions: true,
                syntaxHighlight: {{
                    activate: true,
                    theme: "monokai"
                }}
            }});
        }};
    </script>
</body>
</html>"#,
        title = title,
        spec_url = spec_url
    )
}

/// Generate a minimal Swagger UI HTML page.
pub fn swagger_ui_minimal(spec_url: &str) -> String {
    swagger_ui_html(spec_url, "API Documentation")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swagger_html_contains_elements() {
        let html = swagger_ui_html("/openapi.json", "Test API");
        
        assert!(html.contains("swagger-ui"));
        assert!(html.contains("/openapi.json"));
        assert!(html.contains("Test API"));
        assert!(html.contains("SwaggerUIBundle"));
    }

    #[test]
    fn swagger_minimal_works() {
        let html = swagger_ui_minimal("/api/openapi.json");
        
        assert!(html.contains("/api/openapi.json"));
        assert!(html.contains("API Documentation"));
    }
}
