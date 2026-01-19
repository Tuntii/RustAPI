use rustapi_rs::prelude::StatusCode;
use rustapi_rs::prelude::*;

#[derive(Debug, ApiError)]
enum MyError {
    #[error(status = 404, message = "User not found")]
    UserNotFound,

    #[error(status = 400, code = "custom_error", message = "Custom error")]
    CustomError,

    #[error(status = 500)]
    Internal,

    #[error(status = 418, message = "I'm a teapot")]
    Teapot,
}

#[tokio::test]
async fn test_api_error_derive() {
    // Test Not Found
    let err = MyError::UserNotFound;
    let resp = err.into_response();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Test Custom
    let err = MyError::CustomError;
    let resp = err.into_response();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Test Internal
    let err = MyError::Internal;
    let resp = err.into_response();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

    // Test Teapot
    let err = MyError::Teapot;
    let resp = err.into_response();
    assert_eq!(resp.status(), StatusCode::IM_A_TEAPOT);
}
