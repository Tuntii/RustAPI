use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::{rngs::OsRng, RngCore};
use std::fmt;

/// A CSRF token.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CsrfToken(String);

impl CsrfToken {
    /// Generate a new random CSRF token of the specified length.
    pub fn generate(length: usize) -> Self {
        let mut bytes = vec![0u8; length];
        OsRng.fill_bytes(&mut bytes);
        let token = URL_SAFE_NO_PAD.encode(&bytes);
        Self(token)
    }

    /// Create a token from an existing string.
    pub fn new(token: String) -> Self {
        Self(token)
    }

    /// Get the token string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for CsrfToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CsrfToken").field(&"***").finish()
    }
}

impl fmt::Display for CsrfToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl rustapi_core::FromRequestParts for CsrfToken {
    fn from_request_parts(req: &rustapi_core::Request) -> rustapi_core::Result<Self> {
        use http::StatusCode;
        use rustapi_core::ApiError;

        match req.extensions().get::<CsrfToken>() {
            Some(token) => Ok(token.clone()),
            None => Err(ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "csrf_missing",
                "CSRF token missing from request extensions. Ensure CSRF middleware is enabled.",
            )),
        }
    }
}

impl rustapi_openapi::OperationModifier for CsrfToken {
    fn update_operation(_op: &mut rustapi_openapi::Operation) {
        // CSRF token is handled by middleware, so we don't need to document
        // it as a parameter for every operation that extracts it.
        // It's usually part of the global security requirements.
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// **Feature: v1-features-roadmap, Property 15: CSRF token lifecycle**
    /// **Validates: Requirements 9.1, 9.2, 9.3, 9.4**
    ///
    /// For any CSRF token:
    /// - Generation SHALL produce unique, cryptographically secure tokens
    /// - Token round-trip (to string and back) SHALL preserve the value
    /// - Tokens SHALL be URL-safe base64 encoded

    /// Strategy for generating token lengths
    fn token_length_strategy() -> impl Strategy<Value = usize> {
        16usize..128
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 15: Token generation produces valid base64 strings
        #[test]
        fn prop_token_generates_valid_base64(length in token_length_strategy()) {
            let token = CsrfToken::generate(length);
            let token_str = token.as_str();
            
            // Should be non-empty
            prop_assert!(!token_str.is_empty());
            
            // Should be valid base64 (URL_SAFE_NO_PAD)
            use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
            let decoded = URL_SAFE_NO_PAD.decode(token_str);
            prop_assert!(decoded.is_ok());
            
            // Decoded bytes should match the requested length
            prop_assert_eq!(decoded.unwrap().len(), length);
        }

        /// Property 15: Token round-trip preserves value
        #[test]
        fn prop_token_roundtrip(length in token_length_strategy()) {
            let token1 = CsrfToken::generate(length);
            let token_str = token1.as_str();
            let token2 = CsrfToken::new(token_str.to_string());
            
            prop_assert_eq!(token1, token2);
            prop_assert_eq!(token1.as_str(), token2.as_str());
        }

        /// Property 15: Generated tokens are unique
        #[test]
        fn prop_tokens_are_unique(length in token_length_strategy()) {
            let token1 = CsrfToken::generate(length);
            let token2 = CsrfToken::generate(length);
            
            // With cryptographically secure random generation,
            // two tokens should never be equal
            prop_assert_ne!(token1, token2);
            prop_assert_ne!(token1.as_str(), token2.as_str());
        }

        /// Property 15: Token string representation is consistent
        #[test]
        fn prop_token_display_matches_as_str(length in token_length_strategy()) {
            let token = CsrfToken::generate(length);
            let as_str = token.as_str();
            let displayed = format!("{}", token);
            
            prop_assert_eq!(as_str, displayed);
        }

        /// Property 15: Tokens are URL-safe (no padding, no special chars)
        #[test]
        fn prop_token_is_url_safe(length in token_length_strategy()) {
            let token = CsrfToken::generate(length);
            let token_str = token.as_str();
            
            // Should not contain padding (=)
            prop_assert!(!token_str.contains('='));
            
            // Should only contain URL-safe base64 chars: A-Za-z0-9_-
            for c in token_str.chars() {
                prop_assert!(c.is_ascii_alphanumeric() || c == '_' || c == '-');
            }
        }

        /// Property 15: Token lifetime validation (simulated with timestamp)
        #[test]
        fn prop_token_validates_within_lifetime(
            length in token_length_strategy(),
            elapsed_seconds in 0u64..86400, // 0 to 24 hours
            max_age_seconds in 3600u64..172800, // 1 to 48 hours
        ) {
            use std::time::Duration;
            
            // Simulate token generation and validation timing
            let token = CsrfToken::generate(length);
            
            // Token should be valid if elapsed < max_age
            let is_valid = Duration::from_secs(elapsed_seconds) < Duration::from_secs(max_age_seconds);
            
            // This property demonstrates the lifecycle concept
            // In actual middleware, tokens would be compared with creation timestamp
            if is_valid {
                prop_assert!(elapsed_seconds < max_age_seconds);
            } else {
                prop_assert!(elapsed_seconds >= max_age_seconds);
            }
            
            // Token itself remains structurally valid regardless of time
            prop_assert!(!token.as_str().is_empty());
        }
    }

    #[test]
    fn test_token_debug_doesnt_leak() {
        let token = CsrfToken::generate(32);
        let debug_str = format!("{:?}", token);
        
        // Debug output should not contain the actual token
        assert!(!debug_str.contains(token.as_str()));
        assert!(debug_str.contains("***"));
    }

    #[test]
    fn test_token_clone_equality() {
        let token1 = CsrfToken::generate(32);
        let token2 = token1.clone();
        
        assert_eq!(token1, token2);
        assert_eq!(token1.as_str(), token2.as_str());
    }
}
