//! HTTP admin route handlers for replay endpoints.
//!
//! Handles `/__rustapi/replays` admin API routes.

use super::auth::ReplayAdminAuth;
use super::client::ReplayClient;
use bytes::Bytes;
use http::StatusCode;
use http_body_util::Full;
use rustapi_core::replay::{compute_diff, ReplayConfig, ReplayQuery, ReplayStore};
use rustapi_core::Response;
use rustapi_core::ResponseBody;
use serde_json::json;

/// Dispatch a replay admin request based on path and method.
///
/// Returns `Some(Response)` if the path matches a replay admin route,
/// or `None` if it should be handled by the regular middleware chain.
pub async fn dispatch(
    headers: &http::HeaderMap,
    method: &str,
    uri: &http::Uri,
    store: &dyn ReplayStore,
    config: &ReplayConfig,
    path_suffix: &str,
) -> Option<Response> {
    // Check admin token
    if let Some(ref token) = config.admin_token {
        if let Err(resp) = ReplayAdminAuth::check(headers, token) {
            return Some(resp);
        }
    } else {
        // No token configured, refuse all admin requests
        return Some(json_response(
            StatusCode::FORBIDDEN,
            json!({"error": "forbidden", "message": "Admin token not configured"}),
        ));
    }

    // Trim leading slash
    let suffix = path_suffix.trim_start_matches('/');

    match (method, suffix) {
        // GET /__rustapi/replays - list entries
        ("GET", "") => Some(handle_list(uri, store).await),

        // GET /__rustapi/replays/{id} - show entry
        ("GET", id) if !id.contains('/') => Some(handle_show(id, store).await),

        // POST /__rustapi/replays/{id}/run?target=URL - replay
        ("POST", path) if path.ends_with("/run") => {
            let id = path.trim_end_matches("/run");
            let target = extract_query_param(uri, "target");
            match target {
                Some(target_url) => Some(handle_run(id, &target_url, store).await),
                None => Some(json_response(
                    StatusCode::BAD_REQUEST,
                    json!({"error": "bad_request", "message": "Missing 'target' query parameter"}),
                )),
            }
        }

        // POST /__rustapi/replays/{id}/diff?target=URL - replay & diff
        ("POST", path) if path.ends_with("/diff") => {
            let id = path.trim_end_matches("/diff");
            let target = extract_query_param(uri, "target");
            match target {
                Some(target_url) => Some(handle_diff(id, &target_url, store).await),
                None => Some(json_response(
                    StatusCode::BAD_REQUEST,
                    json!({"error": "bad_request", "message": "Missing 'target' query parameter"}),
                )),
            }
        }

        // DELETE /__rustapi/replays/{id} - delete entry
        ("DELETE", id) if !id.contains('/') => Some(handle_delete(id, store).await),

        _ => Some(json_response(
            StatusCode::NOT_FOUND,
            json!({"error": "not_found", "message": "Unknown replay endpoint"}),
        )),
    }
}

async fn handle_list(uri: &http::Uri, store: &dyn ReplayStore) -> Response {
    let query = replay_query_from_uri(uri);

    match store.list(&query).await {
        Ok(entries) => {
            let count = entries.len();
            let total = store.count().await.unwrap_or(0);
            json_response(
                StatusCode::OK,
                json!({
                    "entries": entries,
                    "count": count,
                    "total": total,
                }),
            )
        }
        Err(e) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": "store_error", "message": e.to_string()}),
        ),
    }
}

fn replay_query_from_uri(uri: &http::Uri) -> ReplayQuery {
    let mut query = ReplayQuery::new();

    if let Some(limit) = extract_query_param(uri, "limit") {
        if let Ok(n) = limit.parse::<usize>() {
            query = query.limit(n);
        }
    }
    if let Some(offset) = extract_query_param(uri, "offset") {
        if let Ok(n) = offset.parse::<usize>() {
            query = query.offset(n);
        }
    }
    if let Some(method) = extract_query_param(uri, "method") {
        if !method.is_empty() {
            query = query.method(method.to_ascii_uppercase());
        }
    }
    if let Some(path) = extract_query_param(uri, "path") {
        if !path.is_empty() {
            query = query.path_contains(path);
        }
    }
    if let Some(status_min) = extract_query_param(uri, "status_min") {
        if let Ok(s) = status_min.parse::<u16>() {
            query = query.status_min(s);
        }
    }
    if let Some(status_max) = extract_query_param(uri, "status_max") {
        if let Ok(s) = status_max.parse::<u16>() {
            query = query.status_max(s);
        }
    }
    if let Some(from) = extract_query_param(uri, "from") {
        if let Ok(ts) = from.parse::<u64>() {
            query = query.from_timestamp(ts);
        }
    }
    if let Some(to) = extract_query_param(uri, "to") {
        if let Ok(ts) = to.parse::<u64>() {
            query = query.to_timestamp(ts);
        }
    }
    if let Some(tag) = extract_query_param(uri, "tag") {
        if let Some((key, value)) = tag.split_once('=') {
            if !key.is_empty() {
                query = query.tag(key.to_string(), value.to_string());
            }
        }
    }
    if let Some(order) = extract_query_param(uri, "order") {
        query = query.newest_first(!matches!(order.as_str(), "asc" | "oldest" | "oldest_first"));
    }

    query
}

async fn handle_show(id: &str, store: &dyn ReplayStore) -> Response {
    match store.get(id).await {
        Ok(Some(entry)) => json_response(StatusCode::OK, serde_json::to_value(&entry).unwrap()),
        Ok(None) => json_response(
            StatusCode::NOT_FOUND,
            json!({"error": "not_found", "message": format!("Entry {} not found", id)}),
        ),
        Err(e) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": "store_error", "message": e.to_string()}),
        ),
    }
}

async fn handle_run(id: &str, target_url: &str, store: &dyn ReplayStore) -> Response {
    let entry = match store.get(id).await {
        Ok(Some(entry)) => entry,
        Ok(None) => {
            return json_response(
                StatusCode::NOT_FOUND,
                json!({"error": "not_found", "message": format!("Entry {} not found", id)}),
            );
        }
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error": "store_error", "message": e.to_string()}),
            );
        }
    };

    let client = ReplayClient::new();
    match client.replay(&entry, target_url).await {
        Ok(replayed) => json_response(
            StatusCode::OK,
            json!({
                "original_response": entry.response,
                "replayed_response": replayed,
                "target": target_url,
            }),
        ),
        Err(e) => json_response(
            StatusCode::BAD_GATEWAY,
            json!({"error": "replay_failed", "message": e.to_string()}),
        ),
    }
}

async fn handle_diff(id: &str, target_url: &str, store: &dyn ReplayStore) -> Response {
    let entry = match store.get(id).await {
        Ok(Some(entry)) => entry,
        Ok(None) => {
            return json_response(
                StatusCode::NOT_FOUND,
                json!({"error": "not_found", "message": format!("Entry {} not found", id)}),
            );
        }
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error": "store_error", "message": e.to_string()}),
            );
        }
    };

    let client = ReplayClient::new();
    match client.replay(&entry, target_url).await {
        Ok(replayed) => {
            let ignore_headers = vec![
                "date".to_string(),
                "x-request-id".to_string(),
                "x-correlation-id".to_string(),
                "server".to_string(),
            ];
            let diff = compute_diff(&entry.response, &replayed, &ignore_headers);

            json_response(
                StatusCode::OK,
                json!({
                    "diff": diff,
                    "original_response": entry.response,
                    "replayed_response": replayed,
                    "target": target_url,
                }),
            )
        }
        Err(e) => json_response(
            StatusCode::BAD_GATEWAY,
            json!({"error": "replay_failed", "message": e.to_string()}),
        ),
    }
}

async fn handle_delete(id: &str, store: &dyn ReplayStore) -> Response {
    match store.delete(id).await {
        Ok(true) => json_response(
            StatusCode::OK,
            json!({"message": format!("Entry {} deleted", id)}),
        ),
        Ok(false) => json_response(
            StatusCode::NOT_FOUND,
            json!({"error": "not_found", "message": format!("Entry {} not found", id)}),
        ),
        Err(e) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": "store_error", "message": e.to_string()}),
        ),
    }
}

/// Helper to extract a query parameter value from a URI.
fn extract_query_param(uri: &http::Uri, key: &str) -> Option<String> {
    uri.query().and_then(|q| {
        q.split('&').find_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            if k == key {
                Some(v.to_string())
            } else {
                None
            }
        })
    })
}

/// Helper to create a JSON response.
fn json_response(status: StatusCode, body: serde_json::Value) -> Response {
    let body_bytes = serde_json::to_vec(&body).unwrap_or_default();
    http::Response::builder()
        .status(status)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(ResponseBody::Full(Full::new(Bytes::from(body_bytes))))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::replay_query_from_uri;

    #[test]
    fn replay_query_from_uri_supports_ui_filters() {
        let uri: http::Uri = "/__rustapi/replays?limit=20&offset=40&method=get&path=/api&status_min=400&status_max=499&from=1000&to=2000&tag=tenant=acme&order=asc"
            .parse()
            .unwrap();

        let query = replay_query_from_uri(&uri);

        assert_eq!(query.limit, Some(20));
        assert_eq!(query.offset, Some(40));
        assert_eq!(query.method.as_deref(), Some("GET"));
        assert_eq!(query.path_contains.as_deref(), Some("/api"));
        assert_eq!(query.status_min, Some(400));
        assert_eq!(query.status_max, Some(499));
        assert_eq!(query.from_timestamp, Some(1000));
        assert_eq!(query.to_timestamp, Some(2000));
        assert_eq!(
            query.tag.as_ref().map(|(k, v)| (k.as_str(), v.as_str())),
            Some(("tenant", "acme"))
        );
        assert!(!query.newest_first);
    }
}
