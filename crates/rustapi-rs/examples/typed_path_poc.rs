use rustapi_rs::prelude::*;
use serde::{Deserialize, Serialize};

// --- User Code (This is how users would use it) ---

#[derive(Debug, Serialize, Deserialize, TypedPath)]
#[typed_path("/users/{id}")]
struct UserPath {
    id: u64,
}

#[derive(Debug, Serialize, Deserialize, TypedPath)]
#[typed_path("/users/{user_id}/posts/{post_id}")]
struct PostPath {
    user_id: u64,
    post_id: String,
}

// Handler using the typed path
async fn get_user(Typed(params): Typed<UserPath>) -> String {
    format!("Get user {}", params.id)
}

async fn get_post(Typed(params): Typed<PostPath>) -> String {
    format!("Get post {} for user {}", params.post_id, params.user_id)
}

#[tokio::main]
async fn main() {
    println!("Running Typed Path Example...");

    let _app = RustApi::new()
        // Type-safe registration!
        // The path string is derived from UserParam::PATH
        .typed::<UserPath>(get(get_user))
        .typed::<PostPath>(get(get_post));

    println!("Routes registered:");
    // In a real app we'd print registered routes from router,
    // but here we just demonstrate the API structure compiles.
    println!(" - {}", UserPath::PATH);
    println!(" - {}", PostPath::PATH);

    // Type-safe URL generation!
    let user_link = UserPath { id: 42 }.to_uri();
    println!("Generated Link: {}", user_link);
    assert_eq!(user_link, "/users/42");

    let post_link = PostPath {
        user_id: 42,
        post_id: "hello".to_string(),
    }
    .to_uri();
    println!("Generated Link: {}", post_link);
    assert_eq!(post_link, "/users/42/posts/hello");
}
