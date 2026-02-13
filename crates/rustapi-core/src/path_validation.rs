//! Route path validation utilities
//!
//! This module provides compile-time and runtime validation for route paths.
//! The validation logic is shared between the proc-macro crate (for compile-time
//! validation) and the core crate (for runtime validation and testing).

/// Result of path validation
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PathValidationError {
    /// Path must start with '/'
    MustStartWithSlash { path: String },
    /// Path contains empty segment (double slash)
    EmptySegment { path: String },
    /// Nested braces are not allowed
    NestedBraces { path: String, position: usize },
    /// Unmatched closing brace
    UnmatchedClosingBrace { path: String, position: usize },
    /// Empty parameter name
    EmptyParameterName { path: String, position: usize },
    /// Invalid parameter name (contains invalid characters)
    InvalidParameterName {
        path: String,
        param_name: String,
        position: usize,
    },
    /// Parameter name starts with digit
    ParameterStartsWithDigit {
        path: String,
        param_name: String,
        position: usize,
    },
    /// Unclosed brace
    UnclosedBrace { path: String },
    /// Invalid character in path
    InvalidCharacter {
        path: String,
        character: char,
        position: usize,
    },
}

impl std::fmt::Display for PathValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathValidationError::MustStartWithSlash { path } => {
                write!(f, "route path must start with '/', got: \"{}\"", path)
            }
            PathValidationError::EmptySegment { path } => {
                write!(
                    f,
                    "route path contains empty segment (double slash): \"{}\"",
                    path
                )
            }
            PathValidationError::NestedBraces { path, position } => {
                write!(
                    f,
                    "nested braces are not allowed in route path at position {}: \"{}\"",
                    position, path
                )
            }
            PathValidationError::UnmatchedClosingBrace { path, position } => {
                write!(
                    f,
                    "unmatched closing brace '}}' at position {} in route path: \"{}\"",
                    position, path
                )
            }
            PathValidationError::EmptyParameterName { path, position } => {
                write!(
                    f,
                    "empty parameter name '{{}}' at position {} in route path: \"{}\"",
                    position, path
                )
            }
            PathValidationError::InvalidParameterName {
                path,
                param_name,
                position,
            } => {
                write!(f, "invalid parameter name '{{{}}}' at position {} - parameter names must contain only alphanumeric characters and underscores: \"{}\"", param_name, position, path)
            }
            PathValidationError::ParameterStartsWithDigit {
                path,
                param_name,
                position,
            } => {
                write!(
                    f,
                    "parameter name '{{{}}}' cannot start with a digit at position {}: \"{}\"",
                    param_name, position, path
                )
            }
            PathValidationError::UnclosedBrace { path } => {
                write!(
                    f,
                    "unclosed brace '{{' in route path (missing closing '}}'): \"{}\"",
                    path
                )
            }
            PathValidationError::InvalidCharacter {
                path,
                character,
                position,
            } => {
                write!(
                    f,
                    "invalid character '{}' at position {} in route path: \"{}\"",
                    character, position, path
                )
            }
        }
    }
}

impl std::error::Error for PathValidationError {}

/// Validate route path syntax
///
/// Returns Ok(()) if the path is valid, or Err with a descriptive error.
///
/// # Valid paths
/// - Must start with '/'
/// - Can contain alphanumeric characters, '-', '_', '.', '/'
/// - Can contain path parameters in the form `{param_name}`
/// - Parameter names must be valid identifiers (alphanumeric + underscore, not starting with digit)
///
/// # Invalid paths
/// - Paths not starting with '/'
/// - Paths with empty segments (double slashes like '//')
/// - Paths with unclosed or nested braces
/// - Paths with empty parameter names like '{}'
/// - Paths with invalid parameter names
/// - Paths with invalid characters
///
/// # Examples
///
/// ```
/// use rustapi_core::path_validation::validate_path;
///
/// // Valid paths
/// assert!(validate_path("/").is_ok());
/// assert!(validate_path("/users").is_ok());
/// assert!(validate_path("/users/{id}").is_ok());
/// assert!(validate_path("/users/{user_id}/posts/{post_id}").is_ok());
///
/// // Invalid paths
/// assert!(validate_path("users").is_err()); // Missing leading /
/// assert!(validate_path("/users//posts").is_err()); // Double slash
/// assert!(validate_path("/users/{").is_err()); // Unclosed brace
/// assert!(validate_path("/users/{}").is_err()); // Empty parameter
/// assert!(validate_path("/users/{123}").is_err()); // Parameter starts with digit
/// ```
#[allow(dead_code)]
pub fn validate_path(path: &str) -> Result<(), PathValidationError> {
    // Path must start with /
    if !path.starts_with('/') {
        return Err(PathValidationError::MustStartWithSlash {
            path: path.to_string(),
        });
    }

    // Check for empty path segments (double slashes)
    if path.contains("//") {
        return Err(PathValidationError::EmptySegment {
            path: path.to_string(),
        });
    }

    // Validate path parameter syntax
    let mut brace_depth = 0;
    let mut param_start = None;

    for (i, ch) in path.char_indices() {
        match ch {
            '{' => {
                if brace_depth > 0 {
                    return Err(PathValidationError::NestedBraces {
                        path: path.to_string(),
                        position: i,
                    });
                }
                brace_depth += 1;
                param_start = Some(i);
            }
            '}' => {
                if brace_depth == 0 {
                    return Err(PathValidationError::UnmatchedClosingBrace {
                        path: path.to_string(),
                        position: i,
                    });
                }
                brace_depth -= 1;

                // Check that parameter name is not empty
                if let Some(start) = param_start {
                    let param_name = &path[start + 1..i];
                    if param_name.is_empty() {
                        return Err(PathValidationError::EmptyParameterName {
                            path: path.to_string(),
                            position: start,
                        });
                    }
                    // Validate parameter name contains only valid identifier characters
                    if !param_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                        return Err(PathValidationError::InvalidParameterName {
                            path: path.to_string(),
                            param_name: param_name.to_string(),
                            position: start,
                        });
                    }
                    // Parameter name must not start with a digit
                    if param_name
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_digit())
                        .unwrap_or(false)
                    {
                        return Err(PathValidationError::ParameterStartsWithDigit {
                            path: path.to_string(),
                            param_name: param_name.to_string(),
                            position: start,
                        });
                    }
                }
                param_start = None;
            }
            // Check for invalid characters in path (outside of parameters)
            _ if brace_depth == 0 => {
                // Allow alphanumeric, -, _, ., /, and common URL characters
                if !ch.is_alphanumeric() && !"-_./*".contains(ch) {
                    return Err(PathValidationError::InvalidCharacter {
                        path: path.to_string(),
                        character: ch,
                        position: i,
                    });
                }
            }
            _ => {}
        }
    }

    // Check for unclosed braces
    if brace_depth > 0 {
        return Err(PathValidationError::UnclosedBrace {
            path: path.to_string(),
        });
    }

    Ok(())
}

/// Check if a path is valid (convenience function)
#[allow(dead_code)]
pub fn is_valid_path(path: &str) -> bool {
    validate_path(path).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Unit tests for specific cases
    #[test]
    fn test_valid_paths() {
        assert!(validate_path("/").is_ok());
        assert!(validate_path("/users").is_ok());
        assert!(validate_path("/users/{id}").is_ok());
        assert!(validate_path("/users/{user_id}").is_ok());
        assert!(validate_path("/users/{user_id}/posts").is_ok());
        assert!(validate_path("/users/{user_id}/posts/{post_id}").is_ok());
        assert!(validate_path("/api/v1/users").is_ok());
        assert!(validate_path("/api-v1/users").is_ok());
        assert!(validate_path("/api_v1/users").is_ok());
        assert!(validate_path("/api.v1/users").is_ok());
        assert!(validate_path("/users/*").is_ok()); // Wildcard
    }

    #[test]
    fn test_missing_leading_slash() {
        let result = validate_path("users");
        assert!(matches!(
            result,
            Err(PathValidationError::MustStartWithSlash { .. })
        ));

        let result = validate_path("users/{id}");
        assert!(matches!(
            result,
            Err(PathValidationError::MustStartWithSlash { .. })
        ));
    }

    #[test]
    fn test_double_slash() {
        let result = validate_path("/users//posts");
        assert!(matches!(
            result,
            Err(PathValidationError::EmptySegment { .. })
        ));

        let result = validate_path("//users");
        assert!(matches!(
            result,
            Err(PathValidationError::EmptySegment { .. })
        ));
    }

    #[test]
    fn test_unclosed_brace() {
        let result = validate_path("/users/{id");
        assert!(matches!(
            result,
            Err(PathValidationError::UnclosedBrace { .. })
        ));

        let result = validate_path("/users/{");
        assert!(matches!(
            result,
            Err(PathValidationError::UnclosedBrace { .. })
        ));
    }

    #[test]
    fn test_unmatched_closing_brace() {
        let result = validate_path("/users/id}");
        assert!(matches!(
            result,
            Err(PathValidationError::UnmatchedClosingBrace { .. })
        ));

        let result = validate_path("/users/}");
        assert!(matches!(
            result,
            Err(PathValidationError::UnmatchedClosingBrace { .. })
        ));
    }

    #[test]
    fn test_empty_parameter_name() {
        let result = validate_path("/users/{}");
        assert!(matches!(
            result,
            Err(PathValidationError::EmptyParameterName { .. })
        ));

        let result = validate_path("/users/{}/posts");
        assert!(matches!(
            result,
            Err(PathValidationError::EmptyParameterName { .. })
        ));
    }

    #[test]
    fn test_nested_braces() {
        let result = validate_path("/users/{{id}}");
        assert!(matches!(
            result,
            Err(PathValidationError::NestedBraces { .. })
        ));

        let result = validate_path("/users/{outer{inner}}");
        assert!(matches!(
            result,
            Err(PathValidationError::NestedBraces { .. })
        ));
    }

    #[test]
    fn test_parameter_starts_with_digit() {
        let result = validate_path("/users/{123}");
        assert!(matches!(
            result,
            Err(PathValidationError::ParameterStartsWithDigit { .. })
        ));

        let result = validate_path("/users/{1id}");
        assert!(matches!(
            result,
            Err(PathValidationError::ParameterStartsWithDigit { .. })
        ));
    }

    #[test]
    fn test_invalid_parameter_name() {
        let result = validate_path("/users/{id-name}");
        assert!(matches!(
            result,
            Err(PathValidationError::InvalidParameterName { .. })
        ));

        let result = validate_path("/users/{id.name}");
        assert!(matches!(
            result,
            Err(PathValidationError::InvalidParameterName { .. })
        ));

        let result = validate_path("/users/{id name}");
        assert!(matches!(
            result,
            Err(PathValidationError::InvalidParameterName { .. })
        ));
    }

    #[test]
    fn test_invalid_characters() {
        let result = validate_path("/users?query");
        assert!(matches!(
            result,
            Err(PathValidationError::InvalidCharacter { .. })
        ));

        let result = validate_path("/users#anchor");
        assert!(matches!(
            result,
            Err(PathValidationError::InvalidCharacter { .. })
        ));

        let result = validate_path("/users@domain");
        assert!(matches!(
            result,
            Err(PathValidationError::InvalidCharacter { .. })
        ));
    }

    // **Feature: phase4-ergonomics-v1, Property 2: Invalid Path Syntax Rejection**
    //
    // For any route path string that contains invalid syntax (e.g., unclosed braces,
    // invalid characters), the system should reject it with a clear error message.
    //
    // **Validates: Requirements 1.5**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property: Valid paths are accepted
        ///
        /// For any path that follows the valid path structure:
        /// - Starts with /
        /// - Contains only valid segments (alphanumeric, -, _, .)
        /// - Has properly formed parameters {name} where name is a valid identifier
        ///
        /// The validation should succeed.
        #[test]
        fn prop_valid_paths_accepted(
            // Generate valid path segments (non-empty to avoid double slashes)
            segments in prop::collection::vec("[a-zA-Z][a-zA-Z0-9_-]{0,10}", 0..5),
            // Generate valid parameter names (must start with letter or underscore)
            params in prop::collection::vec("[a-zA-Z_][a-zA-Z0-9_]{0,10}", 0..3),
        ) {
            // Build a valid path from segments and parameters
            let mut path = String::from("/");

            for (i, segment) in segments.iter().enumerate() {
                if i > 0 {
                    path.push('/');
                }
                path.push_str(segment);
            }

            // Add parameters at the end (only if we have segments or it's the root)
            for param in params.iter() {
                if path != "/" {
                    path.push('/');
                }
                path.push('{');
                path.push_str(param);
                path.push('}');
            }

            // If path is just "/", that's valid
            // Otherwise ensure we have a valid structure
            let result = validate_path(&path);
            prop_assert!(
                result.is_ok(),
                "Valid path '{}' should be accepted, but got error: {:?}",
                path,
                result.err()
            );
        }

        /// Property: Paths without leading slash are rejected
        ///
        /// For any path that doesn't start with '/', validation should fail
        /// with MustStartWithSlash error.
        #[test]
        fn prop_missing_leading_slash_rejected(
            // Generate path content that doesn't start with /
            content in "[a-zA-Z][a-zA-Z0-9/_-]{0,20}",
        ) {
            // Ensure the path doesn't start with /
            let path = if content.starts_with('/') {
                format!("x{}", content)
            } else {
                content
            };

            let result = validate_path(&path);
            prop_assert!(
                matches!(result, Err(PathValidationError::MustStartWithSlash { .. })),
                "Path '{}' without leading slash should be rejected with MustStartWithSlash, got: {:?}",
                path,
                result
            );
        }

        /// Property: Paths with unclosed braces are rejected
        ///
        /// For any path containing an unclosed '{', validation should fail.
        #[test]
        fn prop_unclosed_brace_rejected(
            // Use a valid prefix without double slashes
            prefix in "/[a-zA-Z][a-zA-Z0-9_-]{0,10}",
            param_start in "[a-zA-Z_][a-zA-Z0-9_]{0,5}",
        ) {
            // Create a path with an unclosed brace
            let path = format!("{}/{{{}", prefix, param_start);

            let result = validate_path(&path);
            prop_assert!(
                matches!(result, Err(PathValidationError::UnclosedBrace { .. })),
                "Path '{}' with unclosed brace should be rejected with UnclosedBrace, got: {:?}",
                path,
                result
            );
        }

        /// Property: Paths with unmatched closing braces are rejected
        ///
        /// For any path containing a '}' without a matching '{', validation should fail.
        #[test]
        fn prop_unmatched_closing_brace_rejected(
            // Use a valid prefix without double slashes
            prefix in "/[a-zA-Z][a-zA-Z0-9_-]{0,10}",
            suffix in "[a-zA-Z0-9_]{0,5}",
        ) {
            // Create a path with an unmatched closing brace
            let path = format!("{}/{}}}", prefix, suffix);

            let result = validate_path(&path);
            prop_assert!(
                matches!(result, Err(PathValidationError::UnmatchedClosingBrace { .. })),
                "Path '{}' with unmatched closing brace should be rejected, got: {:?}",
                path,
                result
            );
        }

        /// Property: Paths with empty parameter names are rejected
        ///
        /// For any path containing '{}', validation should fail with EmptyParameterName.
        #[test]
        fn prop_empty_parameter_rejected(
            // Use a valid prefix without double slashes
            prefix in "/[a-zA-Z][a-zA-Z0-9_-]{0,10}",
            has_suffix in proptest::bool::ANY,
            suffix_content in "[a-zA-Z][a-zA-Z0-9_-]{0,10}",
        ) {
            // Create a path with an empty parameter
            let suffix = if has_suffix {
                format!("/{}", suffix_content)
            } else {
                String::new()
            };
            let path = format!("{}/{{}}{}", prefix, suffix);

            let result = validate_path(&path);
            prop_assert!(
                matches!(result, Err(PathValidationError::EmptyParameterName { .. })),
                "Path '{}' with empty parameter should be rejected with EmptyParameterName, got: {:?}",
                path,
                result
            );
        }

        /// Property: Paths with parameters starting with digits are rejected
        ///
        /// For any path containing a parameter that starts with a digit,
        /// validation should fail with ParameterStartsWithDigit.
        #[test]
        fn prop_parameter_starting_with_digit_rejected(
            // Use a valid prefix without double slashes
            prefix in "/[a-zA-Z][a-zA-Z0-9_-]{0,10}",
            digit in "[0-9]",
            rest in "[a-zA-Z0-9_]{0,5}",
        ) {
            // Create a path with a parameter starting with a digit
            let path = format!("{}/{{{}{}}}", prefix, digit, rest);

            let result = validate_path(&path);
            prop_assert!(
                matches!(result, Err(PathValidationError::ParameterStartsWithDigit { .. })),
                "Path '{}' with parameter starting with digit should be rejected, got: {:?}",
                path,
                result
            );
        }

        /// Property: Paths with double slashes are rejected
        ///
        /// For any path containing '//', validation should fail with EmptySegment.
        #[test]
        fn prop_double_slash_rejected(
            prefix in "/[a-zA-Z0-9_-]{0,10}",
            suffix in "[a-zA-Z0-9/_-]{0,10}",
        ) {
            // Create a path with double slash
            let path = format!("{}//{}", prefix, suffix);

            let result = validate_path(&path);
            prop_assert!(
                matches!(result, Err(PathValidationError::EmptySegment { .. })),
                "Path '{}' with double slash should be rejected with EmptySegment, got: {:?}",
                path,
                result
            );
        }

        /// Property: Error messages contain the original path
        ///
        /// For any invalid path, the error message should contain the original path
        /// for debugging purposes.
        #[test]
        fn prop_error_contains_path(
            // Generate various invalid paths
            invalid_type in 0..5usize,
            content in "[a-zA-Z][a-zA-Z0-9_]{1,10}",
        ) {
            let path = match invalid_type {
                0 => content.clone(), // Missing leading slash
                1 => format!("/{}//test", content), // Double slash
                2 => format!("/{}/{{", content), // Unclosed brace
                3 => format!("/{}/{{}}", content), // Empty parameter
                4 => format!("/{content}/{{1{content}}}", content = content), // Parameter starts with digit
                _ => content.clone(),
            };

            let result = validate_path(&path);
            if let Err(err) = result {
                let error_message = err.to_string();
                prop_assert!(
                    error_message.contains(&path) || error_message.contains(&content),
                    "Error message '{}' should contain the path or content for debugging",
                    error_message
                );
            }
        }
    }
}
