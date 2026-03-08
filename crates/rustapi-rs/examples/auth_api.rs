#[cfg(not(any(feature = "extras-session", feature = "session")))]
fn main() {
    eprintln!(
        "Run this example with session support enabled:\n  cargo run -p rustapi-rs --example auth_api --features extras-session"
    );
}

#[cfg(any(feature = "extras-session", feature = "session"))]
use rustapi_rs::extras::session::{MemorySessionStore, Session, SessionConfig, SessionLayer};
#[cfg(any(feature = "extras-session", feature = "session"))]
use rustapi_rs::prelude::*;
#[cfg(any(feature = "extras-session", feature = "session"))]
use std::time::Duration;

#[cfg(any(feature = "extras-session", feature = "session"))]
#[derive(Debug, Deserialize, Schema)]
struct LoginRequest {
    user_id: String,
}

#[cfg(any(feature = "extras-session", feature = "session"))]
#[derive(Debug, Serialize, Schema)]
struct SessionView {
    authenticated: bool,
    user_id: Option<String>,
    refreshed: bool,
    session_id: Option<String>,
}

#[cfg(any(feature = "extras-session", feature = "session"))]
async fn session_view(session: &Session) -> SessionView {
    let user_id = session.get::<String>("user_id").await.ok().flatten();
    let refreshed = session
        .get::<bool>("refreshed")
        .await
        .ok()
        .flatten()
        .unwrap_or(false);
    let session_id = session.id().await;

    SessionView {
        authenticated: user_id.is_some(),
        user_id,
        refreshed,
        session_id,
    }
}

#[cfg(any(feature = "extras-session", feature = "session"))]
async fn login(session: Session, Json(payload): Json<LoginRequest>) -> Json<SessionView> {
    session.cycle_id().await;
    session
        .insert("user_id", &payload.user_id)
        .await
        .expect("serializing user_id into the session should succeed");
    session
        .insert("refreshed", false)
        .await
        .expect("serializing refreshed flag into the session should succeed");

    Json(session_view(&session).await)
}

#[cfg(any(feature = "extras-session", feature = "session"))]
async fn me(session: Session) -> Json<SessionView> {
    Json(session_view(&session).await)
}

#[cfg(any(feature = "extras-session", feature = "session"))]
async fn refresh(session: Session) -> Json<SessionView> {
    if session.contains("user_id").await {
        session.cycle_id().await;
        session
            .insert("refreshed", true)
            .await
            .expect("serializing refreshed flag into the session should succeed");
    }

    Json(session_view(&session).await)
}

#[cfg(any(feature = "extras-session", feature = "session"))]
async fn logout(session: Session) -> NoContent {
    session.destroy().await;
    NoContent
}

#[cfg(any(feature = "extras-session", feature = "session"))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Starting session auth example...");
    println!(" -> POST   http://127.0.0.1:3000/auth/login   {{\"user_id\":\"demo-user\"}}");
    println!(" -> GET    http://127.0.0.1:3000/auth/me");
    println!(" -> POST   http://127.0.0.1:3000/auth/refresh");
    println!(" -> POST   http://127.0.0.1:3000/auth/logout");

    RustApi::new()
        .layer(SessionLayer::new(
            MemorySessionStore::new(),
            SessionConfig::new()
                .cookie_name("rustapi_auth")
                .secure(false)
                .ttl(Duration::from_secs(60 * 30)),
        ))
        .route("/auth/login", post(login))
        .route("/auth/me", get(me))
        .route("/auth/refresh", post(refresh))
        .route("/auth/logout", post(logout))
        .run("127.0.0.1:3000")
        .await
}
