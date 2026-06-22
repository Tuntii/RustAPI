use crate::handler::BoxedHandler;
use crate::path_params::PathParams;
use http::Method;

/// Result of route matching
pub enum RouteMatch<'a> {
    Found {
        handler: &'a BoxedHandler,
        params: PathParams,
    },
    NotFound,
    MethodNotAllowed {
        allowed: Vec<Method>,
    },
}

/// Convert {param} style to :param for matchit
pub(crate) fn convert_path_params(path: &str) -> String {
    let mut result = String::with_capacity(path.len());

    for ch in path.chars() {
        match ch {
            '{' => {
                result.push(':');
            }
            '}' => {
                // Skip closing brace
            }
            _ => {
                result.push(ch);
            }
        }
    }

    result
}

/// Normalize a path for conflict comparison by replacing parameter names with a placeholder
pub(crate) fn normalize_path_for_comparison(path: &str) -> String {
    let mut result = String::with_capacity(path.len());
    let mut in_param = false;

    for ch in path.chars() {
        match ch {
            ':' => {
                in_param = true;
                result.push_str(":_");
            }
            '/' => {
                in_param = false;
                result.push('/');
            }
            _ if in_param => {
                // Skip parameter name characters
            }
            _ => {
                result.push(ch);
            }
        }
    }

    result
}

/// Normalize a prefix for router nesting.
///
/// Ensures the prefix:
/// - Starts with exactly one leading slash
/// - Has no trailing slash (unless it's just "/")
/// - Has no double slashes
///
/// # Examples
///
/// ```ignore
/// assert_eq!(normalize_prefix("api"), "/api");
/// assert_eq!(normalize_prefix("/api"), "/api");
/// assert_eq!(normalize_prefix("/api/"), "/api");
/// assert_eq!(normalize_prefix("//api//"), "/api");
/// assert_eq!(normalize_prefix(""), "/");
/// ```
pub(crate) fn normalize_prefix(prefix: &str) -> String {
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
