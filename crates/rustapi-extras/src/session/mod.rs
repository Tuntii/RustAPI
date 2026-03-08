//! Session middleware, extractors, and storage backends.
//!
//! This module provides a cookie-backed session flow that stores session state in
//! pluggable backends. It is intentionally small and framework-native: handlers
//! receive a [`Session`] extractor, mutate typed values, and the middleware
//! persists those changes after the response is produced.
//!
//! # Example
//!
//! ```rust,ignore
//! use rustapi_rs::prelude::*;
//! use rustapi_rs::extras::session::{MemorySessionStore, Session, SessionConfig, SessionLayer};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Deserialize)]
//! struct LoginPayload {
//!     user_id: String,
//! }
//!
//! #[derive(Serialize)]
//! struct MeResponse {
//!     user_id: String,
//! }
//!
//! async fn login(session: Session, Json(payload): Json<LoginPayload>) -> Result<Json<MeResponse>> {
//!     session.cycle_id().await;
//!     session.insert("user_id", &payload.user_id).await?;
//!     Ok(Json(MeResponse { user_id: payload.user_id }))
//! }
//!
//! async fn me(session: Session) -> Result<Json<MeResponse>> {
//!     let user_id = session
//!         .get::<String>("user_id")
//!         .await?
//!         .ok_or_else(|| ApiError::unauthorized("Not logged in"))?;
//!
//!     Ok(Json(MeResponse { user_id }))
//! }
//!
//! let app = RustApi::new()
//!     .layer(SessionLayer::new(
//!         MemorySessionStore::new(),
//!         SessionConfig::new().cookie_name("rustapi_session"),
//!     ))
//!     .route("/login", post(login))
//!     .route("/me", get(me));
//! ```

use async_trait::async_trait;
use cookie::{Cookie, SameSite};
use http::{header, HeaderValue};
use rustapi_core::middleware::{BoxedNext, MiddlewareLayer};
use rustapi_core::{ApiError, FromRequestParts, IntoResponse, Request, Response, Result};
use rustapi_openapi::{Operation, OperationModifier};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, RwLock};
use tracing::warn;
use uuid::Uuid;

/// Result type for session operations.
pub type SessionResult<T> = std::result::Result<T, SessionError>;

/// Arbitrary JSON-backed session data.
pub type SessionData = BTreeMap<String, Value>;

/// Errors that can occur when loading, mutating, or persisting sessions.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    /// The configured store failed to read data.
    #[error("Failed to read session data: {0}")]
    Read(String),

    /// The configured store failed to persist data.
    #[error("Failed to persist session data: {0}")]
    Write(String),

    /// A serialized value could not be converted to JSON.
    #[error("Failed to serialize session value: {0}")]
    Serialize(String),

    /// A JSON value could not be converted back to the requested type.
    #[error("Failed to deserialize session value: {0}")]
    Deserialize(String),

    /// A store-specific configuration error occurred.
    #[error("Invalid session store configuration: {0}")]
    Config(String),
}

impl From<SessionError> for ApiError {
    fn from(error: SessionError) -> Self {
        ApiError::internal(error.to_string())
    }
}

/// Persistent representation of a session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionRecord {
    /// Stable session identifier stored in the cookie.
    pub id: String,
    /// Arbitrary JSON data for the session.
    #[serde(default)]
    pub data: SessionData,
    /// UNIX timestamp in seconds when the session expires.
    pub expires_at: u64,
}

impl SessionRecord {
    /// Create a new record with the given TTL.
    pub fn new(id: impl Into<String>, data: SessionData, ttl: Duration) -> Self {
        let expires_at = current_unix_seconds().saturating_add(ttl.as_secs());
        Self {
            id: id.into(),
            data,
            expires_at,
        }
    }

    /// Returns true when the record should be treated as expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at <= current_unix_seconds()
    }

    /// Returns the remaining TTL in seconds, saturating at zero.
    pub fn ttl_seconds(&self) -> u64 {
        self.expires_at.saturating_sub(current_unix_seconds())
    }
}

/// Storage backend contract for sessions.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Load a session by identifier.
    async fn load(&self, session_id: &str) -> SessionResult<Option<SessionRecord>>;

    /// Persist a session record.
    async fn save(&self, record: SessionRecord) -> SessionResult<()>;

    /// Delete a session by identifier.
    async fn delete(&self, session_id: &str) -> SessionResult<()>;
}

/// In-memory session store suitable for tests, examples, and single-node deployments.
#[derive(Debug, Clone, Default)]
pub struct MemorySessionStore {
    sessions: Arc<RwLock<HashMap<String, SessionRecord>>>,
}

impl MemorySessionStore {
    /// Create a new empty in-memory session store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of currently retained sessions, excluding entries cleaned on access.
    pub async fn len(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Returns true if the store currently has no retained sessions.
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }
}

#[async_trait]
impl SessionStore for MemorySessionStore {
    async fn load(&self, session_id: &str) -> SessionResult<Option<SessionRecord>> {
        if let Some(record) = self.sessions.read().await.get(session_id).cloned() {
            if record.is_expired() {
                self.sessions.write().await.remove(session_id);
                return Ok(None);
            }

            return Ok(Some(record));
        }

        Ok(None)
    }

    async fn save(&self, record: SessionRecord) -> SessionResult<()> {
        self.sessions
            .write()
            .await
            .insert(record.id.clone(), record);
        Ok(())
    }

    async fn delete(&self, session_id: &str) -> SessionResult<()> {
        self.sessions.write().await.remove(session_id);
        Ok(())
    }
}

/// Configuration for cookie-backed sessions.
#[derive(Clone, Debug)]
pub struct SessionConfig {
    /// Cookie name used to transport the session identifier.
    pub cookie_name: String,
    /// Cookie path.
    pub cookie_path: String,
    /// Optional cookie domain.
    pub cookie_domain: Option<String>,
    /// Whether the cookie should be sent over HTTPS only.
    pub cookie_secure: bool,
    /// Whether JavaScript should be denied access to the cookie.
    pub cookie_http_only: bool,
    /// SameSite value for the session cookie.
    pub cookie_same_site: SameSite,
    /// Logical TTL for stored session records.
    pub ttl: Duration,
    /// If enabled, every successfully loaded session is re-saved with a fresh expiry.
    pub rolling: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            cookie_name: "rustapi_session".to_string(),
            cookie_path: "/".to_string(),
            cookie_domain: None,
            cookie_secure: true,
            cookie_http_only: true,
            cookie_same_site: SameSite::Lax,
            ttl: Duration::from_secs(60 * 60 * 24),
            rolling: true,
        }
    }
}

impl SessionConfig {
    /// Create a default session configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Override the cookie name.
    pub fn cookie_name(mut self, value: impl Into<String>) -> Self {
        self.cookie_name = value.into();
        self
    }

    /// Override the cookie path.
    pub fn cookie_path(mut self, value: impl Into<String>) -> Self {
        self.cookie_path = value.into();
        self
    }

    /// Override the cookie domain.
    pub fn cookie_domain(mut self, value: impl Into<String>) -> Self {
        self.cookie_domain = Some(value.into());
        self
    }

    /// Toggle the secure flag.
    pub fn secure(mut self, secure: bool) -> Self {
        self.cookie_secure = secure;
        self
    }

    /// Toggle the HTTP only flag.
    pub fn http_only(mut self, value: bool) -> Self {
        self.cookie_http_only = value;
        self
    }

    /// Override the SameSite setting.
    pub fn same_site(mut self, value: SameSite) -> Self {
        self.cookie_same_site = value;
        self
    }

    /// Override the session TTL.
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Toggle rolling expiration.
    pub fn rolling(mut self, rolling: bool) -> Self {
        self.rolling = rolling;
        self
    }
}

#[derive(Debug, Clone)]
struct SessionState {
    id: Option<String>,
    data: SessionData,
    loaded: bool,
    dirty: bool,
    destroyed: bool,
    rotate_id: bool,
}

impl SessionState {
    fn from_record(record: Option<SessionRecord>) -> Self {
        match record {
            Some(record) => Self {
                id: Some(record.id),
                data: record.data,
                loaded: true,
                dirty: false,
                destroyed: false,
                rotate_id: false,
            },
            None => Self {
                id: None,
                data: SessionData::new(),
                loaded: false,
                dirty: false,
                destroyed: false,
                rotate_id: false,
            },
        }
    }

    fn ensure_id(&mut self) -> String {
        if self.id.is_none() {
            self.id = Some(Uuid::new_v4().to_string());
        }

        self.id.clone().expect("session id should be present")
    }
}

/// Request extractor for reading and mutating the current session.
#[derive(Clone)]
pub struct Session {
    inner: Arc<Mutex<SessionState>>,
}

impl Session {
    fn new(inner: Arc<Mutex<SessionState>>) -> Self {
        Self { inner }
    }

    /// Get the current session identifier, if already assigned.
    pub async fn id(&self) -> Option<String> {
        self.inner.lock().await.id.clone()
    }

    /// Returns true if the session currently holds the given key.
    pub async fn contains(&self, key: &str) -> bool {
        self.inner.lock().await.data.contains_key(key)
    }

    /// Returns a full copy of the current session data.
    pub async fn entries(&self) -> SessionData {
        self.inner.lock().await.data.clone()
    }

    /// Returns the number of stored keys.
    pub async fn len(&self) -> usize {
        self.inner.lock().await.data.len()
    }

    /// Returns true if the current session contains no keys.
    pub async fn is_empty(&self) -> bool {
        self.inner.lock().await.data.is_empty()
    }

    /// Read a typed value from the session.
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> SessionResult<Option<T>> {
        let guard = self.inner.lock().await;
        guard
            .data
            .get(key)
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|error| SessionError::Deserialize(error.to_string()))
    }

    /// Read the raw JSON value stored for a key.
    pub async fn get_value(&self, key: &str) -> Option<Value> {
        self.inner.lock().await.data.get(key).cloned()
    }

    /// Insert or replace a typed value.
    pub async fn insert<T: Serialize>(
        &self,
        key: impl Into<String>,
        value: T,
    ) -> SessionResult<()> {
        let mut guard = self.inner.lock().await;
        let value = serde_json::to_value(value)
            .map_err(|error| SessionError::Serialize(error.to_string()))?;
        guard.ensure_id();
        guard.data.insert(key.into(), value);
        guard.dirty = true;
        guard.destroyed = false;
        Ok(())
    }

    /// Remove a value from the session, returning the raw JSON if present.
    pub async fn remove(&self, key: &str) -> Option<Value> {
        let mut guard = self.inner.lock().await;
        let removed = guard.data.remove(key);
        if removed.is_some() {
            guard.dirty = true;
        }
        removed
    }

    /// Clear all values while keeping the session container alive.
    pub async fn clear(&self) {
        let mut guard = self.inner.lock().await;
        if !guard.data.is_empty() || guard.loaded {
            guard.dirty = true;
        }
        guard.data.clear();
        guard.destroyed = false;
    }

    /// Mark the session for deletion and clear all values.
    pub async fn destroy(&self) {
        let mut guard = self.inner.lock().await;
        guard.data.clear();
        guard.dirty = true;
        guard.destroyed = true;
    }

    /// Rotate the session identifier on the next persistence cycle.
    pub async fn cycle_id(&self) {
        let mut guard = self.inner.lock().await;
        guard.ensure_id();
        guard.rotate_id = true;
        guard.dirty = true;
        guard.destroyed = false;
    }
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session").finish_non_exhaustive()
    }
}

impl FromRequestParts for Session {
    fn from_request_parts(req: &Request) -> Result<Self> {
        req.extensions().get::<Session>().cloned().ok_or_else(|| {
            ApiError::internal("Session middleware is missing. Add SessionLayer first.")
        })
    }
}

impl OperationModifier for Session {
    fn update_operation(_op: &mut Operation) {}
}

/// Middleware that loads session state before the handler and persists it afterwards.
#[derive(Clone)]
pub struct SessionLayer<S> {
    store: Arc<S>,
    config: Arc<SessionConfig>,
}

impl<S> SessionLayer<S>
where
    S: SessionStore + 'static,
{
    /// Create a new session layer.
    pub fn new(store: S, config: SessionConfig) -> Self {
        Self {
            store: Arc::new(store),
            config: Arc::new(config),
        }
    }

    /// Create a new session layer from a shared store instance.
    pub fn from_arc(store: Arc<S>, config: SessionConfig) -> Self {
        Self {
            store,
            config: Arc::new(config),
        }
    }
}

impl<S> MiddlewareLayer for SessionLayer<S>
where
    S: SessionStore + 'static,
{
    fn call(
        &self,
        mut req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let store = self.store.clone();
        let config = self.config.clone();

        Box::pin(async move {
            let incoming_session_id = cookie_value(req.headers(), &config.cookie_name);
            let mut clear_stale_cookie = false;

            let record = if let Some(session_id) = incoming_session_id.as_deref() {
                match store.load(session_id).await {
                    Ok(Some(record)) if !record.is_expired() => Some(record),
                    Ok(Some(_)) => {
                        if let Err(error) = store.delete(session_id).await {
                            warn!(error = %error, session_id, "failed to delete expired session record");
                        }
                        clear_stale_cookie = true;
                        None
                    }
                    Ok(None) => {
                        clear_stale_cookie = true;
                        None
                    }
                    Err(error) => return ApiError::from(error).into_response(),
                }
            } else {
                None
            };

            let previous_id = record.as_ref().map(|record| record.id.clone());
            let state = Arc::new(Mutex::new(SessionState::from_record(record)));
            req.extensions_mut().insert(Session::new(state.clone()));

            let mut response = next(req).await;

            let snapshot = state.lock().await.clone();

            if snapshot.destroyed {
                if let Some(session_id) = snapshot.id.as_deref().or(previous_id.as_deref()) {
                    if let Err(error) = store.delete(session_id).await {
                        return ApiError::from(error).into_response();
                    }
                }

                append_clear_cookie(&mut response, &config);
                return response;
            }

            let should_persist = if snapshot.loaded {
                snapshot.dirty || config.rolling
            } else {
                snapshot.dirty && !snapshot.data.is_empty()
            };

            if should_persist {
                let mut session_id = snapshot
                    .id
                    .clone()
                    .unwrap_or_else(|| Uuid::new_v4().to_string());

                if snapshot.rotate_id {
                    let rotated_id = Uuid::new_v4().to_string();
                    if let Some(previous_id) = snapshot.id.as_deref() {
                        if previous_id != rotated_id {
                            if let Err(error) = store.delete(previous_id).await {
                                return ApiError::from(error).into_response();
                            }
                        }
                    }
                    session_id = rotated_id;
                }

                let record =
                    SessionRecord::new(session_id.clone(), snapshot.data.clone(), config.ttl);

                if let Err(error) = store.save(record).await {
                    return ApiError::from(error).into_response();
                }

                append_session_cookie(&mut response, &config, &session_id);
                return response;
            }

            if clear_stale_cookie {
                append_clear_cookie(&mut response, &config);
            }

            response
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(Self {
            store: self.store.clone(),
            config: self.config.clone(),
        })
    }
}

#[cfg(feature = "session-redis")]
use redis::AsyncCommands;

/// Redis-backed session storage.
#[cfg(feature = "session-redis")]
#[derive(Clone)]
pub struct RedisSessionStore {
    client: redis::Client,
    key_prefix: String,
}

#[cfg(feature = "session-redis")]
impl std::fmt::Debug for RedisSessionStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisSessionStore")
            .field("key_prefix", &self.key_prefix)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "session-redis")]
impl RedisSessionStore {
    /// Create a Redis session store from an existing client.
    pub fn new(client: redis::Client) -> Self {
        Self {
            client,
            key_prefix: "rustapi:session:".to_string(),
        }
    }

    /// Create a Redis session store from a connection URL.
    pub fn from_url(url: &str) -> SessionResult<Self> {
        let client =
            redis::Client::open(url).map_err(|error| SessionError::Config(error.to_string()))?;
        Ok(Self::new(client))
    }

    /// Override the key prefix used for session records.
    pub fn key_prefix(mut self, value: impl Into<String>) -> Self {
        self.key_prefix = value.into();
        self
    }

    fn key(&self, session_id: &str) -> String {
        format!("{}{}", self.key_prefix, session_id)
    }
}

#[cfg(feature = "session-redis")]
#[async_trait]
impl SessionStore for RedisSessionStore {
    async fn load(&self, session_id: &str) -> SessionResult<Option<SessionRecord>> {
        let mut connection = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|error| SessionError::Read(error.to_string()))?;

        let payload: Option<String> = connection
            .get(self.key(session_id))
            .await
            .map_err(|error| SessionError::Read(error.to_string()))?;

        payload
            .map(|payload| {
                serde_json::from_str(&payload)
                    .map_err(|error| SessionError::Deserialize(error.to_string()))
            })
            .transpose()
    }

    async fn save(&self, record: SessionRecord) -> SessionResult<()> {
        let ttl = record.ttl_seconds().max(1);
        let payload = serde_json::to_string(&record)
            .map_err(|error| SessionError::Serialize(error.to_string()))?;

        let mut connection = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|error| SessionError::Write(error.to_string()))?;

        connection
            .set_ex(self.key(&record.id), payload, ttl)
            .await
            .map_err(|error| SessionError::Write(error.to_string()))
    }

    async fn delete(&self, session_id: &str) -> SessionResult<()> {
        let mut connection = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|error| SessionError::Write(error.to_string()))?;

        let _: usize = connection
            .del(self.key(session_id))
            .await
            .map_err(|error| SessionError::Write(error.to_string()))?;

        Ok(())
    }
}

fn current_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn cookie_value(headers: &http::HeaderMap, cookie_name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            Cookie::split_parse(value)
                .filter_map(|cookie| cookie.ok())
                .find(|cookie| cookie.name() == cookie_name)
                .map(|cookie| cookie.value().to_string())
        })
}

fn append_session_cookie(response: &mut Response, config: &SessionConfig, session_id: &str) {
    let mut cookie = Cookie::build((config.cookie_name.clone(), session_id.to_string()))
        .path(config.cookie_path.clone())
        .secure(config.cookie_secure)
        .http_only(config.cookie_http_only)
        .same_site(config.cookie_same_site)
        .max_age(cookie::time::Duration::seconds(config.ttl.as_secs() as i64));

    if let Some(domain) = &config.cookie_domain {
        cookie = cookie.domain(domain.clone());
    }

    response
        .headers_mut()
        .append(header::SET_COOKIE, cookie_header_value(cookie.build()));
}

fn append_clear_cookie(response: &mut Response, config: &SessionConfig) {
    let mut cookie = Cookie::build((config.cookie_name.clone(), String::new()))
        .path(config.cookie_path.clone())
        .secure(config.cookie_secure)
        .http_only(config.cookie_http_only)
        .same_site(config.cookie_same_site)
        .max_age(cookie::time::Duration::seconds(0));

    if let Some(domain) = &config.cookie_domain {
        cookie = cookie.domain(domain.clone());
    }

    response
        .headers_mut()
        .append(header::SET_COOKIE, cookie_header_value(cookie.build()));
}

fn cookie_header_value(cookie: Cookie<'static>) -> HeaderValue {
    HeaderValue::from_str(&cookie.to_string()).unwrap_or_else(|_| HeaderValue::from_static(""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::StatusCode;
    use rustapi_core::{get, post, Body, Json, NoContent, RustApi};
    use rustapi_openapi::ResponseModifier;
    use rustapi_testing::{TestClient, TestRequest};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, Serialize)]
    struct LoginPayload {
        user_id: String,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct SessionUser {
        user_id: String,
        refreshed: bool,
    }

    enum TestSessionResponse {
        User(SessionUser),
        Empty,
        Error(ApiError),
    }

    impl IntoResponse for TestSessionResponse {
        fn into_response(self) -> Response {
            match self {
                Self::User(body) => Json(body).into_response(),
                Self::Empty => NoContent.into_response(),
                Self::Error(error) => error.into_response(),
            }
        }
    }

    impl ResponseModifier for TestSessionResponse {
        fn update_response(_op: &mut Operation) {}
    }

    async fn login(session: Session, body: Body) -> TestSessionResponse {
        let payload: LoginPayload = match serde_json::from_slice(&body) {
            Ok(payload) => payload,
            Err(error) => {
                return TestSessionResponse::Error(ApiError::bad_request(error.to_string()))
            }
        };

        session.cycle_id().await;
        if let Err(error) = session.insert("user_id", &payload.user_id).await {
            return TestSessionResponse::Error(ApiError::from(error));
        }
        if let Err(error) = session.insert("refreshed", false).await {
            return TestSessionResponse::Error(ApiError::from(error));
        }

        TestSessionResponse::User(SessionUser {
            user_id: payload.user_id,
            refreshed: false,
        })
    }

    async fn me(session: Session) -> TestSessionResponse {
        let user_id = match session.get::<String>("user_id").await {
            Ok(Some(user_id)) => user_id,
            Ok(None) => return TestSessionResponse::Error(ApiError::unauthorized("Not logged in")),
            Err(error) => return TestSessionResponse::Error(ApiError::from(error)),
        };

        let refreshed = match session.get::<bool>("refreshed").await {
            Ok(Some(refreshed)) => refreshed,
            Ok(None) => false,
            Err(error) => return TestSessionResponse::Error(ApiError::from(error)),
        };

        TestSessionResponse::User(SessionUser { user_id, refreshed })
    }

    async fn refresh(session: Session) -> TestSessionResponse {
        session.cycle_id().await;
        if let Err(error) = session.insert("refreshed", true).await {
            return TestSessionResponse::Error(ApiError::from(error));
        }

        let user_id = match session.get::<String>("user_id").await {
            Ok(Some(user_id)) => user_id,
            Ok(None) => return TestSessionResponse::Error(ApiError::unauthorized("Not logged in")),
            Err(error) => return TestSessionResponse::Error(ApiError::from(error)),
        };

        TestSessionResponse::User(SessionUser {
            user_id,
            refreshed: true,
        })
    }

    async fn logout(session: Session) -> TestSessionResponse {
        session.destroy().await;
        TestSessionResponse::Empty
    }

    fn set_cookie_value(response: &rustapi_testing::TestResponse) -> String {
        response
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|value| value.to_str().ok())
            .expect("missing set-cookie header")
            .to_string()
    }

    fn cookie_pair(set_cookie: &str) -> String {
        set_cookie
            .split(';')
            .next()
            .expect("cookie pair should exist")
            .to_string()
    }

    #[tokio::test]
    async fn login_refresh_logout_flow_works() {
        let store = MemorySessionStore::new();
        let app = RustApi::new()
            .layer(SessionLayer::new(
                store.clone(),
                SessionConfig::new().cookie_name("sid").secure(false),
            ))
            .route("/login", post(login))
            .route("/me", get(me))
            .route("/refresh", post(refresh))
            .route("/logout", post(logout));

        let client = TestClient::new(app);

        let login_response = client
            .request(TestRequest::post("/login").json(&LoginPayload {
                user_id: "user-42".to_string(),
            }))
            .await;

        login_response.assert_status(StatusCode::OK);
        let login_cookie = set_cookie_value(&login_response);
        let login_pair = cookie_pair(&login_cookie);
        assert!(login_pair.starts_with("sid="));

        let me_response = client
            .request(TestRequest::get("/me").header("Cookie", &login_pair))
            .await;

        me_response.assert_status(StatusCode::OK);
        me_response.assert_json(&SessionUser {
            user_id: "user-42".to_string(),
            refreshed: false,
        });

        let first_session_id = login_pair.trim_start_matches("sid=").to_string();
        assert!(store.load(&first_session_id).await.unwrap().is_some());

        let refresh_response = client
            .request(TestRequest::post("/refresh").header("Cookie", &login_pair))
            .await;

        refresh_response.assert_status(StatusCode::OK);
        let refreshed_cookie = set_cookie_value(&refresh_response);
        let refreshed_pair = cookie_pair(&refreshed_cookie);
        assert_ne!(login_pair, refreshed_pair);

        let refreshed_me = client
            .request(TestRequest::get("/me").header("Cookie", &refreshed_pair))
            .await;

        refreshed_me.assert_status(StatusCode::OK);
        refreshed_me.assert_json(&SessionUser {
            user_id: "user-42".to_string(),
            refreshed: true,
        });

        let logout_response = client
            .request(TestRequest::post("/logout").header("Cookie", &refreshed_pair))
            .await;

        logout_response.assert_status(StatusCode::NO_CONTENT);
        let cleared_cookie = set_cookie_value(&logout_response);
        assert!(cleared_cookie.contains("Max-Age=0") || cleared_cookie.contains("Max-Age=0;"));

        let after_logout = client
            .request(TestRequest::get("/me").header("Cookie", &refreshed_pair))
            .await;

        after_logout.assert_status(StatusCode::UNAUTHORIZED);
        assert!(store.is_empty().await);
    }

    #[tokio::test]
    async fn stale_cookie_is_cleared() {
        let app = RustApi::new()
            .layer(SessionLayer::new(
                MemorySessionStore::new(),
                SessionConfig::new().cookie_name("sid").secure(false),
            ))
            .route("/me", get(me));

        let client = TestClient::new(app);
        let response = client
            .request(TestRequest::get("/me").header("Cookie", "sid=missing"))
            .await;

        response.assert_status(StatusCode::UNAUTHORIZED);
        let cleared_cookie = set_cookie_value(&response);
        assert!(cleared_cookie.contains("sid="));
        assert!(cleared_cookie.contains("Max-Age=0"));
    }

    #[cfg(feature = "session-redis")]
    #[test]
    fn redis_store_uses_configurable_key_prefix() {
        let store = RedisSessionStore::from_url("redis://127.0.0.1/")
            .unwrap()
            .key_prefix("custom:sessions:");

        assert_eq!(store.key("abc"), "custom:sessions:abc");
    }
}
