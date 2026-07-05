//! Multipart file upload example.
//!
//! Run:
//!   cargo run -p rustapi-rs --example file_upload
//!
//! Test:
//!   curl -X POST http://127.0.0.1:8080/upload -F "file=@./Cargo.toml"

use rustapi_rs::prelude::*;
use std::path::PathBuf;

#[derive(Serialize, Schema)]
struct UploadResponse {
    message: String,
    files: Vec<FileResult>,
}

#[derive(Serialize, Schema)]
struct FileResult {
    original_name: String,
    stored_name: String,
    content_type: String,
    size_bytes: usize,
}

#[rustapi_rs::post("/upload")]
#[rustapi_rs::summary("Upload one or more files")]
async fn upload_handler(mut multipart: Multipart) -> Result<Json<UploadResponse>> {
    let upload_dir = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string());
    tokio::fs::create_dir_all(&upload_dir)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let mut uploaded_files = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::bad_request("Invalid multipart payload"))?
    {
        if !field.is_file() {
            continue;
        }

        let original_name = field.file_name().unwrap_or("unknown.bin").to_string();
        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();
        let safe_filename = format!(
            "{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
            original_name
        );
        let data = field
            .bytes()
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?;
        let size_bytes = data.len();

        let path: PathBuf = PathBuf::from(&upload_dir).join(&safe_filename);
        tokio::fs::write(&path, &data)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?;

        uploaded_files.push(FileResult {
            original_name,
            stored_name: safe_filename,
            content_type,
            size_bytes,
        });
    }

    if uploaded_files.is_empty() {
        return Err(ApiError::bad_request("No file fields found in upload"));
    }

    Ok(Json(UploadResponse {
        message: "Upload successful".into(),
        files: uploaded_files,
    }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tokio::fs::create_dir_all("./uploads").await?;

    let addr = std::env::var("ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    println!("Upload server at http://{addr}/upload");

    RustApi::new()
        .body_limit(50 * 1024 * 1024)
        .layer(BodyLimitLayer::new(50 * 1024 * 1024))
        .route("/upload", post(upload_handler))
        .run(&addr)
        .await
}
