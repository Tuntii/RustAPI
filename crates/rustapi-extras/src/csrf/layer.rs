use super::config::CsrfConfig;
use super::token::CsrfToken;
use cookie::Cookie;
use http::{Method, StatusCode};
use rustapi_core::middleware::{BoxedNext, MiddlewareLayer};
use rustapi_core::{ApiError, IntoResponse, Request, Response};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Middleware for CSRF protection using the Double-Submit Cookie pattern.
#[derive(Clone, Debug)]
pub struct CsrfLayer {
    config: Arc<CsrfConfig>,
}

impl CsrfLayer {
    /// Create a new CSRF middleware layer.
    pub fn new(config: CsrfConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }
}

impl MiddlewareLayer for CsrfLayer {
    fn call(
        &self,
        mut req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let config = self.config.clone();

        Box::pin(async move {
            // 1. Extract existing token from cookie
            let existing_token = req
                .headers()
                .get(http::header::COOKIE)
                .and_then(|h| h.to_str().ok())
                .and_then(|cookie_str| {
                    cookie::Cookie::split_parse(cookie_str)
                        .filter_map(|c| c.ok())
                        .find(|c| c.name() == config.cookie_name)
                        .map(|c| c.value().to_string())
                })
                .map(CsrfToken::new);

            // 2. Determine the token to use for this request context
            // If existing, use it. If not, generate new.
            let (token, is_new) = match existing_token {
                Some(t) => (t, false),
                None => (CsrfToken::generate(config.token_length), true),
            };

            // 3. Store token in request extensions so handlers/templates can access it
            req.extensions_mut().insert(token.clone());

            // 4. Validate if unsafe method
            let method = req.method();
            let is_safe = matches!(
                *method,
                Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
            );

            if !is_safe {
                // For unsafe methods, we MUST have received a matching token in the header
                let header_value = req
                    .headers()
                    .get(&config.header_name)
                    .and_then(|v| v.to_str().ok());

                let valid = match header_value {
                    Some(h_token) => h_token == token.as_str(),
                    None => false,
                };

                if !valid {
                    // Mismatch or missing header -> Forbidden
                    // If cookie was missing (is_new=true), it fails here too as header can't match.
                    // We return JSON error for consistency
                    return ApiError::new(
                        StatusCode::FORBIDDEN,
                        "csrf_forbidden",
                        "CSRF token validation failed",
                    )
                    .into_response();
                }
            }

            // 5. Proceed
            let mut response = next(req).await;

            // 6. Set cookie if new
            if is_new {
                let mut cookie =
                    Cookie::build((config.cookie_name.clone(), token.as_str().to_owned()))
                        .path(config.cookie_path.clone())
                        .secure(config.cookie_secure)
                        .http_only(config.cookie_http_only)
                        .same_site(config.cookie_same_site);

                if let Some(domain) = &config.cookie_domain {
                    cookie = cookie.domain(domain.clone());
                }

                // Note: Not setting max-age strictly to avoid dependency complexity in this snippets,
                // but usually recommended.

                let c = cookie.build();
                let header_value = c.to_string();

                response.headers_mut().append(
                    http::header::SET_COOKIE,
                    header_value
                        .parse()
                        .unwrap_or(http::header::HeaderValue::from_static("")),
                );
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
    use http::StatusCode;
    use rustapi_core::{get, post, RustApi};
    use rustapi_testing::{TestClient, TestRequest, TestResponse};

    async fn handler() -> &'static str {
        "ok"
    }

    #[tokio::test]
    async fn test_safe_method_generates_cookie() {
        let config = CsrfConfig::new().cookie_name("csrf_id");
        let app = RustApi::new()
            .layer(CsrfLayer::new(config))
            .route("/", get(handler));

        let client = TestClient::new(app);
        let res: TestResponse = client.get("/").await;

        assert_eq!(res.status(), StatusCode::OK);
        let cookies = res
            .headers()
            .get("set-cookie")
            .expect("No cookie set")
            .to_str()
            .unwrap();
        assert!(cookies.contains("csrf_id="));
    }

    #[tokio::test]
    async fn test_unsafe_method_without_cookie_fails() {
        let config = CsrfConfig::new();
        let app = RustApi::new()
            .layer(CsrfLayer::new(config))
            .route("/", post(handler));

        let client = TestClient::new(app);
        // POST without cookie or header
        let res: TestResponse = client.request(TestRequest::post("/")).await;

        assert_eq!(res.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_unsafe_method_valid_passes() {
        let config = CsrfConfig::new().cookie_name("ID").header_name("X-ID");
        let app = RustApi::new()
            .layer(CsrfLayer::new(config))
            .route("/", post(handler));

        let client = TestClient::new(app);
        let res: TestResponse = client
            .request(
                TestRequest::post("/")
                    .header("Cookie", "ID=token123")
                    .header("X-ID", "token123"),
            )
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_unsafe_method_mismatch_fails() {
        let config = CsrfConfig::new().cookie_name("ID").header_name("X-ID");
        let app = RustApi::new()
            .layer(CsrfLayer::new(config))
            .route("/", post(handler));

        let client = TestClient::new(app);
        let res: TestResponse = client
            .request(
                TestRequest::post("/")
                    .header("Cookie", "ID=token123")
                    .header("X-ID", "wrongtoken"),
            )
            .await;

        assert_eq!(res.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_csrf_lifecycle() {
        let config = CsrfConfig::new()
            .cookie_name("token")
            .header_name("x-token");
        // Chain handlers on same route to avoid conflict
        let app = RustApi::new()
            .layer(CsrfLayer::new(config))
            .route("/", get(handler).post(handler));

        let client = TestClient::new(app);

        // 1. Initial GET to get token
        let res: TestResponse = client.get("/").await;
        assert_eq!(res.status(), StatusCode::OK);
        let set_cookie = res
            .headers()
            .get("set-cookie")
            .expect("No cookie set")
            .to_str()
            .unwrap();

        // Parse cookie value (simple parse for "token=VALUE; ...")
        let token_part = set_cookie.split(';').next().unwrap(); // "token=VALUE"
        let token_val = token_part.split('=').nth(1).unwrap();

        // 2. Unsafe POST with valid token
        let res: TestResponse = client
            .request(
                TestRequest::post("/")
                    .header("Cookie", token_part)
                    .header("x-token", token_val),
            )
            .await;
        assert_eq!(res.status(), StatusCode::OK);

        // 3. Unsafe POST with invalid token (Mismatch)
        let res: TestResponse = client
            .request(
                TestRequest::post("/")
                    .header("Cookie", token_part)
                    .header("x-token", "bad"),
            )
            .await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_token_extraction() {
        use crate::csrf::CsrfToken;

        async fn token_handler(token: CsrfToken) -> String {
            token.as_str().to_string()
        }

        let config = CsrfConfig::new().cookie_name("csrf_id");
        let app = RustApi::new()
            .layer(CsrfLayer::new(config))
            .route("/", get(token_handler));

        let client = TestClient::new(app);
        let res: TestResponse = client.get("/").await;

        assert_eq!(res.status(), StatusCode::OK);
        let body = res.text();
        assert!(!body.is_empty());

        // Verify token matches cookie
        let cookie_val = res.headers().get("set-cookie").unwrap().to_str().unwrap();
        assert!(cookie_val.contains(&body));
    }
}
