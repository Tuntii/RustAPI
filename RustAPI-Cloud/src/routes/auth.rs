use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use rustapi_rs::prelude::*;

use crate::auth::github;
use crate::auth::jwt;
use crate::config::Config;
use crate::db::DbPool;
use crate::models::{NewOauthDevice, NewUser, User};
use crate::schema::{oauth_devices, users};

#[derive(Debug, Deserialize, Schema)]
pub struct TokenRequest {
    pub grant_type: String,
    pub device_code: String,
}

#[derive(Debug, Deserialize, Schema)]
pub struct ActivateQuery {
    pub code: String,
}

#[derive(Debug, Deserialize, Schema)]
pub struct CallbackQuery {
    pub code: String,
    #[serde(default)]
    pub state: String,
}

pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "rustapi-cloud"
    }))
}

pub async fn auth_device(State(pool): State<DbPool>) -> Json<serde_json::Value> {
    let mut conn = pool.get().await.expect("DB connection failed");

    let device = NewOauthDevice::new();

    diesel::insert_into(oauth_devices::table)
        .values(&device)
        .execute(&mut conn)
        .await
        .expect("DB insert failed");

    Json(serde_json::json!({
        "device_code": device.device_code(),
        "user_code": device.user_code(),
        "verification_uri": "https://rustapi.rs/activate",
        "verification_uri_complete": format!("https://rustapi.rs/activate?code={}", device.user_code()),
        "expires_in": 900,
        "interval": 3,
    }))
}

pub async fn auth_activate(
    Query(query): Query<ActivateQuery>,
    State(config): State<Config>,
) -> Redirect {
    let url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=user:read&state={}",
        config.github_client_id,
        config.github_redirect_uri,
        query.code,
    );
    Redirect::temporary(&url)
}

pub async fn auth_callback(
    Query(query): Query<CallbackQuery>,
    State(pool): State<DbPool>,
    State(config): State<Config>,
) -> Html<String> {
    let token_resp = github::exchange_code(
        &query.code,
        &config.github_client_id,
        &config.github_client_secret,
        &config.github_redirect_uri,
    )
    .await
    .expect("GitHub OAuth failed");

    let gh_user = github::get_user(&token_resp.access_token)
        .await
        .expect("GitHub API failed");

    let mut conn = pool.get().await.expect("DB connection failed");

    let existing = users::table
        .filter(users::github_id.eq(gh_user.id))
        .select(User::as_select())
        .first(&mut conn)
        .await
        .optional()
        .expect("DB query failed");

    let user = match existing {
        Some(u) => u,
        None => {
            let new_user = NewUser::from_github(
                gh_user.id,
                gh_user.login.clone(),
                gh_user.avatar_url.clone(),
                gh_user.email.clone(),
            );
            diesel::insert_into(users::table)
                .values(&new_user)
                .returning(User::as_select())
                .get_result(&mut conn)
                .await
                .expect("DB insert failed")
        }
    };

    if !query.state.is_empty() {
        let _ = diesel::update(oauth_devices::table)
            .filter(oauth_devices::user_code.eq(&query.state))
            .filter(oauth_devices::expires_at.gt(Utc::now().naive_utc()))
            .set(oauth_devices::user_id.eq(Some(user.id.clone())))
            .execute(&mut conn)
            .await;
    }

    Html(format!(
        r#"<!DOCTYPE html>
<html>
<head><title>RustAPI Cloud — Login Successful</title>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<style>
  body {{ font-family: system-ui, -apple-system, sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; background: #0d1117; color: #c9d1d9; }}
  .box {{ text-align: center; padding: 40px; max-width: 400px; }}
  h1 {{ color: #58a6ff; font-size: 24px; }}
  p {{ color: #8b949e; font-size: 16px; line-height: 1.5; }}
  strong {{ color: #f0f6fc; }}
</style>
</head>
<body>
  <div class="box">
    <h1>&#x2705; Login Successful</h1>
    <p>Welcome, <strong>{}</strong>!</p>
    <p>You can close this window and return to your terminal.</p>
  </div>
</body>
</html>"#,
        gh_user.login
    ))
}

pub async fn auth_token(
    State(pool): State<DbPool>,
    State(config): State<Config>,
    Json(body): Json<TokenRequest>,
) -> Json<serde_json::Value> {
    let mut conn = pool.get().await.expect("DB connection");

    let device = oauth_devices::table
        .filter(oauth_devices::device_code.eq(&body.device_code))
        .filter(oauth_devices::expires_at.gt(Utc::now().naive_utc()))
        .select(crate::models::OauthDevice::as_select())
        .first(&mut conn)
        .await
        .optional()
        .expect("DB query");

    match device {
        Some(device) if device.user_id.is_some() => {
            let user = users::table
                .filter(users::id.eq(device.user_id.unwrap()))
                .select(User::as_select())
                .first(&mut conn)
                .await
                .expect("DB user query");

            let (access_token, refresh_token) = jwt::create_token(
                &user.id,
                &user.login,
                user.avatar_url.as_deref(),
                &user.tier,
                &config.jwt_secret,
                720,
            )
            .expect("JWT creation");

            let _ = diesel::delete(oauth_devices::table)
                .filter(oauth_devices::id.eq(device.id))
                .execute(&mut conn)
                .await;

            Json(serde_json::json!({
                "access_token": access_token,
                "refresh_token": refresh_token,
                "token_type": "Bearer",
                "expires_in": 3600,
            }))
        }
        Some(_) => Json(serde_json::json!({
            "error": "authorization_pending",
            "error_description": "User has not yet authorized"
        })),
        None => Json(serde_json::json!({
            "error": "expired_token",
            "error_description": "Device code has expired"
        })),
    }
}

pub async fn auth_whoami(
    State(_pool): State<DbPool>,
    State(config): State<Config>,
    Json(body): Json<TokenRequest>,
) -> Json<serde_json::Value> {
    let token = body.device_code.trim();

    if token.is_empty() {
        return Json(serde_json::json!({
            "error": "Missing token"
        }));
    }

    match crate::auth::jwt::verify_token(token, &config.jwt_secret) {
        Ok(claims) => Json(serde_json::json!({
            "sub": claims.sub,
            "login": claims.login,
            "avatar_url": claims.avatar_url,
            "tier": claims.tier,
        })),
        Err(_) => Json(serde_json::json!({
            "error": "Invalid or expired token"
        })),
    }
}
