//! Input Sanitization Utilities
//!
//! Provides functions to sanitize user input against XSS and injection attacks.
//! NOTE: This is a basic implementation. For production high-risk apps, use a dedicated crate like `ammonia`.

/// Sanitizes a string by escaping HTML special characters.
///
/// Replaces:
/// - `&` -> `&amp;`
/// - `<` -> `&lt;`
/// - `>` -> `&gt;`
/// - `"` -> `&quot;`
/// - `'` -> `&#x27;`
pub fn sanitize_html(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#x27;"),
            _ => output.push(c),
        }
    }
    output
}

/// Strip all HTML tags from a string.
pub fn strip_tags(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut inside_tag = false;

    for c in input.chars() {
        if c == '<' {
            inside_tag = true;
        } else if c == '>' {
            inside_tag = false;
        } else if !inside_tag {
            output.push(c);
        }
    }

    output
}

/// Recursively sanitizes string fields in a JSON value.
pub fn sanitize_json(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(s) => *s = sanitize_html(s),
        serde_json::Value::Array(arr) => {
            for v in arr {
                sanitize_json(v);
            }
        }
        serde_json::Value::Object(map) => {
            for (_, v) in map {
                sanitize_json(v);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_sanitize_html() {
        let input = "<script>alert('XSS')</script>";
        let expected = "&lt;script&gt;alert(&#x27;XSS&#x27;)&lt;/script&gt;";
        assert_eq!(sanitize_html(input), expected);
    }

    #[test]
    fn test_strip_tags() {
        let input = "<p>Hello <b>World</b></p>";
        let expected = "Hello World";
        assert_eq!(strip_tags(input), expected);
    }

    #[test]
    fn test_sanitize_json() {
        let mut data = json!({
            "name": "<b>John</b>",
            "age": 30,
            "tags": ["<script>", "normal"]
        });

        sanitize_json(&mut data);

        assert_eq!(data["name"], "&lt;b&gt;John&lt;/b&gt;");
        assert_eq!(data["tags"][0], "&lt;script&gt;");
        assert_eq!(data["tags"][1], "normal");
    }
}
