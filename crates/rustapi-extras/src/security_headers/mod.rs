//! Security headers middleware
//!
//! This module provides automatic security headers for HTTP responses to protect
//! against common web vulnerabilities.
//!
//! # Example
//!
//! ```rust,no_run
//! use rustapi_core::RustApi;
//! use rustapi_extras::SecurityHeadersLayer;
//!
//! #[tokio::main]
//! async fn main() {
//!     let app = RustApi::new()
//!         .layer(SecurityHeadersLayer::new())
//!         .run("0.0.0.0:3000")
//!         .await
//!         .unwrap();
//! }
//! ```

use rustapi_core::{
    middleware::{BoxedNext, MiddlewareLayer},
    Request, Response,
};
use std::future::Future;
use std::pin::Pin;

/// Security headers configuration
#[derive(Clone)]
pub struct SecurityHeadersConfig {
    /// X-Content-Type-Options: Prevents MIME type sniffing
    pub x_content_type_options: bool,
    /// X-Frame-Options: Prevents clickjacking
    pub x_frame_options: Option<XFrameOptions>,
    /// X-XSS-Protection: Enables XSS filter in older browsers
    pub x_xss_protection: bool,
    /// Strict-Transport-Security: Enforces HTTPS
    pub hsts: Option<HstsConfig>,
    /// Content-Security-Policy: Prevents XSS and data injection
    pub csp: Option<String>,
    /// Referrer-Policy: Controls referrer information
    pub referrer_policy: Option<ReferrerPolicy>,
    /// Permissions-Policy: Controls browser features
    pub permissions_policy: Option<String>,
}

/// X-Frame-Options values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XFrameOptions {
    /// Deny all framing
    Deny,
    /// Allow framing from same origin
    SameOrigin,
}

impl XFrameOptions {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Deny => "DENY",
            Self::SameOrigin => "SAMEORIGIN",
        }
    }
}

/// HSTS (HTTP Strict Transport Security) configuration
#[derive(Debug, Clone)]
pub struct HstsConfig {
    /// Max age in seconds
    pub max_age: u32,
    /// Include subdomains
    pub include_subdomains: bool,
    /// Preload directive
    pub preload: bool,
}

impl HstsConfig {
    fn to_header_value(&self) -> String {
        let mut value = format!("max-age={}", self.max_age);
        if self.include_subdomains {
            value.push_str("; includeSubDomains");
        }
        if self.preload {
            value.push_str("; preload");
        }
        value
    }
}

/// Referrer-Policy values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferrerPolicy {
    /// No referrer information
    NoReferrer,
    /// Only send origin (no path)
    NoReferrerWhenDowngrade,
    /// Only send for same origin
    Origin,
    /// Send origin for cross-origin, full URL for same-origin
    OriginWhenCrossOrigin,
    /// Same origin only
    SameOrigin,
    /// Only send origin
    StrictOrigin,
    /// Origin for cross-origin requests when downgrading
    StrictOriginWhenCrossOrigin,
    /// Always send full URL
    UnsafeUrl,
}

impl ReferrerPolicy {
    fn as_str(&self) -> &'static str {
        match self {
            Self::NoReferrer => "no-referrer",
            Self::NoReferrerWhenDowngrade => "no-referrer-when-downgrade",
            Self::Origin => "origin",
            Self::OriginWhenCrossOrigin => "origin-when-cross-origin",
            Self::SameOrigin => "same-origin",
            Self::StrictOrigin => "strict-origin",
            Self::StrictOriginWhenCrossOrigin => "strict-origin-when-cross-origin",
            Self::UnsafeUrl => "unsafe-url",
        }
    }
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            x_content_type_options: true,
            x_frame_options: Some(XFrameOptions::Deny),
            x_xss_protection: true,
            hsts: Some(HstsConfig {
                max_age: 31536000, // 1 year
                include_subdomains: true,
                preload: false,
            }),
            csp: Some("default-src 'self'".to_string()),
            referrer_policy: Some(ReferrerPolicy::StrictOriginWhenCrossOrigin),
            permissions_policy: Some("geolocation=(), microphone=(), camera=()".to_string()),
        }
    }
}

/// Security headers middleware layer
#[derive(Clone)]
pub struct SecurityHeadersLayer {
    config: SecurityHeadersConfig,
}

impl SecurityHeadersLayer {
    /// Create a new security headers layer with default configuration
    pub fn new() -> Self {
        Self {
            config: SecurityHeadersConfig::default(),
        }
    }

    /// Create with strict security settings (recommended for production)
    pub fn strict() -> Self {
        Self {
            config: SecurityHeadersConfig {
                x_content_type_options: true,
                x_frame_options: Some(XFrameOptions::Deny),
                x_xss_protection: true,
                hsts: Some(HstsConfig {
                    max_age: 63072000, // 2 years
                    include_subdomains: true,
                    preload: true,
                }),
                csp: Some(
                    "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'"
                        .to_string(),
                ),
                referrer_policy: Some(ReferrerPolicy::NoReferrer),
                permissions_policy: Some(
                    "geolocation=(), microphone=(), camera=(), payment=(), usb=()".to_string(),
                ),
            },
        }
    }

    /// Disable X-Content-Type-Options
    pub fn without_content_type_options(mut self) -> Self {
        self.config.x_content_type_options = false;
        self
    }

    /// Set X-Frame-Options
    pub fn x_frame_options(mut self, options: XFrameOptions) -> Self {
        self.config.x_frame_options = Some(options);
        self
    }

    /// Disable X-Frame-Options
    pub fn without_x_frame_options(mut self) -> Self {
        self.config.x_frame_options = None;
        self
    }

    /// Set HSTS configuration
    pub fn hsts(mut self, config: HstsConfig) -> Self {
        self.config.hsts = Some(config);
        self
    }

    /// Disable HSTS (not recommended for production)
    pub fn without_hsts(mut self) -> Self {
        self.config.hsts = None;
        self
    }

    /// Set Content-Security-Policy
    pub fn csp(mut self, policy: impl Into<String>) -> Self {
        self.config.csp = Some(policy.into());
        self
    }

    /// Disable Content-Security-Policy
    pub fn without_csp(mut self) -> Self {
        self.config.csp = None;
        self
    }

    /// Set Referrer-Policy
    pub fn referrer_policy(mut self, policy: ReferrerPolicy) -> Self {
        self.config.referrer_policy = Some(policy);
        self
    }

    /// Set Permissions-Policy
    pub fn permissions_policy(mut self, policy: impl Into<String>) -> Self {
        self.config.permissions_policy = Some(policy.into());
        self
    }
}

impl Default for SecurityHeadersLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl MiddlewareLayer for SecurityHeadersLayer {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let config = self.config.clone();

        Box::pin(async move {
            let mut response = next(req).await;

            // Add security headers to response
            let headers = response.headers_mut();

            // X-Content-Type-Options
            if config.x_content_type_options {
                headers.insert(
                    http::header::HeaderName::from_static("x-content-type-options"),
                    http::header::HeaderValue::from_static("nosniff"),
                );
            }

            // X-Frame-Options
            if let Some(frame_options) = config.x_frame_options {
                headers.insert(
                    http::header::HeaderName::from_static("x-frame-options"),
                    http::header::HeaderValue::from_static(frame_options.as_str()),
                );
            }

            // X-XSS-Protection
            if config.x_xss_protection {
                headers.insert(
                    http::header::HeaderName::from_static("x-xss-protection"),
                    http::header::HeaderValue::from_static("1; mode=block"),
                );
            }

            // Strict-Transport-Security
            if let Some(hsts) = config.hsts {
                if let Ok(value) = http::header::HeaderValue::from_str(&hsts.to_header_value()) {
                    headers.insert(
                        http::header::HeaderName::from_static("strict-transport-security"),
                        value,
                    );
                }
            }

            // Content-Security-Policy
            if let Some(csp) = config.csp {
                if let Ok(value) = http::header::HeaderValue::from_str(&csp) {
                    headers.insert(
                        http::header::HeaderName::from_static("content-security-policy"),
                        value,
                    );
                }
            }

            // Referrer-Policy
            if let Some(referrer_policy) = config.referrer_policy {
                headers.insert(
                    http::header::HeaderName::from_static("referrer-policy"),
                    http::header::HeaderValue::from_static(referrer_policy.as_str()),
                );
            }

            // Permissions-Policy
            if let Some(permissions) = config.permissions_policy {
                if let Ok(value) = http::header::HeaderValue::from_str(&permissions) {
                    headers.insert(
                        http::header::HeaderName::from_static("permissions-policy"),
                        value,
                    );
                }
            }

            response
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use rustapi_core::ResponseBody;
    use std::sync::Arc;

    #[tokio::test]
    async fn security_headers_added_to_response() {
        let layer = SecurityHeadersLayer::new();

        let next: BoxedNext = Arc::new(|_req: Request| {
            Box::pin(async {
                http::Response::builder()
                    .status(200)
                    .body(ResponseBody::new(Bytes::from("OK")))
                    .unwrap()
            }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        });

        let req = Request::from_http_request(
            http::Request::builder()
                .method("GET")
                .uri("/")
                .body(())
                .unwrap(),
            Bytes::new(),
        );

        let response = layer.call(req, next).await;

        // Verify security headers are present
        assert!(response.headers().contains_key("x-content-type-options"));
        assert!(response.headers().contains_key("x-frame-options"));
        assert!(response.headers().contains_key("x-xss-protection"));
        assert!(response.headers().contains_key("strict-transport-security"));
        assert!(response.headers().contains_key("content-security-policy"));
        assert!(response.headers().contains_key("referrer-policy"));
    }

    #[tokio::test]
    async fn strict_mode_adds_all_headers() {
        let layer = SecurityHeadersLayer::strict();

        let next: BoxedNext = Arc::new(|_req: Request| {
            Box::pin(async {
                http::Response::builder()
                    .status(200)
                    .body(ResponseBody::new(Bytes::from("OK")))
                    .unwrap()
            }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        });

        let req = Request::from_http_request(
            http::Request::builder()
                .method("GET")
                .uri("/")
                .body(())
                .unwrap(),
            Bytes::new(),
        );

        let response = layer.call(req, next).await;

        // Verify HSTS includes preload
        let hsts = response
            .headers()
            .get("strict-transport-security")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(hsts.contains("preload"));
        assert!(hsts.contains("includeSubDomains"));
    }

    #[test]
    fn hsts_config_formats_correctly() {
        let hsts = HstsConfig {
            max_age: 31536000,
            include_subdomains: true,
            preload: true,
        };

        assert_eq!(
            hsts.to_header_value(),
            "max-age=31536000; includeSubDomains; preload"
        );
    }
}
