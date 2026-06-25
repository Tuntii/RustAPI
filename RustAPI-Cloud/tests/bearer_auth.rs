use http::HeaderMap;
use rustapi_cloud::auth::bearer::BearerAuth;

const JWT_SECRET: &str = "test-jwt-secret-bearer";

fn auth_headers(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "authorization",
        format!("Bearer {}", token).parse().unwrap(),
    );
    headers
}

#[test]
fn bearer_auth_rejects_missing_and_invalid_tokens() {
    let headers = HeaderMap::new();
    let err = BearerAuth::from_headers(&headers, JWT_SECRET).unwrap_err();
    assert!(err.to_string().contains("Missing Authorization"));

    let headers = auth_headers("not-a-real-token");
    let err = BearerAuth::from_headers(&headers, JWT_SECRET).unwrap_err();
    assert!(err.to_string().contains("Invalid") || err.to_string().contains("expired"));
}

#[test]
fn bearer_auth_accepts_valid_token() {
    let (token, _) = rustapi_cloud::auth::jwt::create_token(
        "user-1",
        "alice",
        None,
        "hobby",
        JWT_SECRET,
        1,
    )
    .expect("token");

    let auth = BearerAuth::from_headers(&auth_headers(&token), JWT_SECRET).expect("valid");
    assert_eq!(auth.user_id(), "user-1");
}