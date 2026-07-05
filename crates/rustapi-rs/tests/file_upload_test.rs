//! Integration test for multipart upload handling (no network bind).

use bytes::Bytes;
use rustapi_rs::prelude::*;
use rustapi_testing::{TestClient, TestRequest};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Serialize, Deserialize, Schema)]
struct UploadResponse {
    message: String,
    files: Vec<FileResult>,
}

#[derive(Debug, Serialize, Deserialize, Schema)]
struct FileResult {
    original_name: String,
    stored_name: String,
    content_type: String,
    size_bytes: usize,
}

#[derive(Clone, Default)]
struct UploadState(Arc<Mutex<Vec<(String, Bytes)>>>);

#[rustapi_rs::post("/upload")]
async fn upload_handler(
    State(state): State<UploadState>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>> {
    let mut uploaded = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::bad_request("Invalid multipart"))?
    {
        if !field.is_file() {
            continue;
        }

        let original_name = field.file_name().unwrap_or("file.bin").to_string();
        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();
        let data = field
            .bytes()
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?;
        let size_bytes = data.len();
        let stored_name = format!("stored-{original_name}");

        state.0.lock().await.push((stored_name.clone(), data));

        uploaded.push(FileResult {
            original_name,
            stored_name,
            content_type,
            size_bytes,
        });
    }

    Ok(Json(UploadResponse {
        message: "Upload successful".into(),
        files: uploaded,
    }))
}

fn multipart_body(boundary: &str, filename: &str, payload: &[u8]) -> Bytes {
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n\
         Content-Type: text/plain\r\n\
         \r\n\
         {content}\r\n\
         --{boundary}--\r\n",
        content = String::from_utf8_lossy(payload),
    );
    Bytes::from(body.into_bytes())
}

#[tokio::test]
async fn multipart_upload_returns_success_json() {
    let state = UploadState::default();
    let app = RustApi::new()
        .state(state.clone())
        .route("/upload", post(upload_handler));

    let client = TestClient::with_body_limit(app, 1024 * 1024);
    let boundary = "rustapi-upload-test";
    let payload = b"hello upload";
    let body = multipart_body(boundary, "demo.txt", payload);

    let response = client
        .request(
            TestRequest::post("/upload")
                .header(
                    "content-type",
                    &format!("multipart/form-data; boundary={boundary}"),
                )
                .body(body),
        )
        .await;

    response.assert_status(StatusCode::OK);
    let json: UploadResponse = response.json().expect("valid upload json");
    assert_eq!(json.message, "Upload successful");
    assert_eq!(json.files.len(), 1);
    assert_eq!(json.files[0].original_name, "demo.txt");
    assert_eq!(json.files[0].size_bytes, payload.len());

    let stored = state.0.lock().await;
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].1.as_ref(), payload);
}
