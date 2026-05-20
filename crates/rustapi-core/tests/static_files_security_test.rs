use rustapi_core::static_files::serve_dir;
use rustapi_core::static_files::StaticFile;
use std::fs::File;

#[tokio::test]
async fn test_directory_traversal_blocked() {
    let config = serve_dir("/static", "./"); // root is crates/rustapi-core

    // We try to access something outside the root
    let relative_path = "../../etc/passwd";
    let res = StaticFile::serve(relative_path, &config).await;
    assert!(res.is_err(), "Standard traversal should be blocked");

    // Percent encoded payload
    let relative_path_encoded = "..%2F..%2Fetc%2Fpasswd";
    let res_encoded = StaticFile::serve(relative_path_encoded, &config).await;
    assert!(res_encoded.is_err(), "Encoded traversal should be blocked");

    // Double encoded
    let relative_path_double = "%2e%2e%2f%2e%2e%2fetc%2fpasswd";
    let res_double = StaticFile::serve(relative_path_double, &config).await;
    assert!(
        res_double.is_err(),
        "Double encoded traversal should be blocked"
    );
}

#[tokio::test]
async fn test_valid_file_served() {
    let config = serve_dir("/static", "./");

    // Valid file
    let relative_path = "src/lib.rs";
    let res = StaticFile::serve(relative_path, &config).await;
    assert!(res.is_ok(), "Valid file should be served");
}

#[tokio::test]
async fn test_valid_file_with_spaces_served() {
    let _ = std::fs::create_dir_all("./test_dir");
    let _ = File::create("./test_dir/file with spaces.txt");

    let config = serve_dir("/static", "./test_dir");

    let relative_path = "file%20with%20spaces.txt";
    let res = StaticFile::serve(relative_path, &config).await;
    assert!(
        res.is_ok(),
        "File with percent-encoded spaces should be served"
    );

    let _ = std::fs::remove_dir_all("./test_dir");
}
