# Replay workflow: time-travel debugging

Record HTTP request/response pairs in a controlled environment, inspect a captured request, replay it against another target, and diff the result before promoting a fix.

> **Security notice**
> Replay is intended for **development, staging, canary, and incident-response environments**. Do not expose the admin endpoints publicly on the open internet.

## When to use it

Replay is most useful when:

- behavior differs between staging and local
- you need to reproduce a regression using a real traffic sample
- you want to rerun critical requests before promoting a new version to canary
- you are asking, “why did this request work yesterday but break today?” and want a time-machine-style answer

## Prerequisites

Enable the canonical replay feature in your application:

```toml
[dependencies]
rustapi-rs = { version = "0.1.335", features = ["extras-replay"] }
```

On the CLI side, `cargo-rustapi` is enough; replay commands are part of the default installation:

```bash
cargo install cargo-rustapi
```

## 1) Enable replay recording

For the smallest practical setup, start with an in-memory store:

```rust,ignore
use rustapi_rs::extras::replay::{InMemoryReplayStore, ReplayConfig, ReplayLayer};
use rustapi_rs::prelude::*;

#[rustapi_rs::get("/api/users")]
async fn list_users() -> Json<Vec<&'static str>> {
    Json(vec!["Alice", "Bob"])
}

#[rustapi_rs::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let replay = ReplayLayer::new(
        ReplayConfig::new()
            .enabled(true)
            .admin_token("local-replay-token")
            .ttl_secs(900)
            .skip_path("/health")
            .skip_path("/ready")
            .skip_path("/live"),
    )
    .with_store(InMemoryReplayStore::new(200));

    RustApi::auto()
        .layer(replay)
        .run("127.0.0.1:8080")
        .await
}
```

This setup:

- enables replay recording
- protects the admin endpoints with a bearer token
- excludes probe endpoints from recording
- keeps entries for 15 minutes
- stores at most 200 records in memory

## 2) Generate target traffic

Now send requests to the application as usual. The replay middleware captures request/response pairs without changing your application code.

The recording flow looks like this:

1. the request passes through
2. request metadata and eligible body fields are stored
3. response status, headers, and capturable body content are stored
4. the record becomes accessible through the admin API and CLI

## 3) List recordings and find the right entry

For a first look, the CLI is the easiest path:

```bash
# List recent replay entries
cargo rustapi replay list -s http://localhost:8080 -t local-replay-token

# Filter to a specific endpoint only
cargo rustapi replay list -s http://localhost:8080 -t local-replay-token --method GET --path /api/users --limit 20
```

The list output shows these fields:

- replay ID
- HTTP method
- path
- original response status code
- total duration

## 4) Inspect a single entry

Once you find the suspicious request, open the full record:

```bash
cargo rustapi replay show <id> -s http://localhost:8080 -t local-replay-token
```

This command typically shows:

- the original request method and URI
- stored headers
- the captured request body
- the original response status/body
- metadata such as duration, client IP, and request ID

## 5) Replay the same request against another environment

You can now run the same request against your local fix, staging, or canary environment:

```bash
cargo rustapi replay run <id> -s http://localhost:8080 -t local-replay-token -T http://localhost:3000
```

Practical uses include:

- verifying that the local fix really resolves the incident
- checking whether staging still matches the previous production behavior
- replaying critical endpoints as a pre-deploy smoke test

## 6) Generate diffs automatically

This is where the real magic happens: compare the replayed response with the original response.

```bash
cargo rustapi replay diff <id> -s http://localhost:8080 -t local-replay-token -T http://staging:8080
```

The `diff` output looks for differences in:

- status code
- response headers
- JSON body fields

That lets you catch subtler regressions too, such as “it still returned 200, but the payload changed.”

## Recommended workflow

During an incident or regression, the recommended flow is:

1. **Start recording**: enable replay in staging/canary with a short TTL.
2. **Capture the example**: replay the real request that triggers the problem.
3. **List**: find the right entry with `cargo rustapi replay list`.
4. **Inspect**: validate the request/response pair with `cargo rustapi replay show`.
5. **Try the fix**: rerun the entry against your local build or release candidate with `run`.
6. **Diff it**: use `diff` to confirm the behavior changed as expected.
7. **Turn it off**: disable replay recording after the incident or keep the TTL short.

In short: **capture → inspect → replay → diff → promote**.

## Admin API reference

All admin endpoints require this header:

```text
Authorization: Bearer <admin_token>
```

| Method | Path | Description |
|--------|------|-------------|
| GET | `/__rustapi/replays` | List recordings |
| GET | `/__rustapi/replays/{id}` | Show a single entry |
| POST | `/__rustapi/replays/{id}/run?target=URL` | Replay the request against another target |
| POST | `/__rustapi/replays/{id}/diff?target=URL` | Replay the request and generate a diff |
| DELETE | `/__rustapi/replays/{id}` | Delete an entry |

### cURL examples

```bash
curl -H "Authorization: Bearer local-replay-token" \
     "http://localhost:8080/__rustapi/replays?limit=10"

curl -H "Authorization: Bearer local-replay-token" \
     "http://localhost:8080/__rustapi/replays/<id>"

curl -X POST -H "Authorization: Bearer local-replay-token" \
     "http://localhost:8080/__rustapi/replays/<id>/run?target=http://staging:8080"

curl -X POST -H "Authorization: Bearer local-replay-token" \
     "http://localhost:8080/__rustapi/replays/<id>/diff?target=http://staging:8080"
```

## Configuration notes

These are the `ReplayConfig` options you will adjust most often:

```rust,ignore
use rustapi_rs::extras::replay::ReplayConfig;

let config = ReplayConfig::new()
    .enabled(true)
    .admin_token("local-replay-token")
    .store_capacity(1_000)
    .ttl_secs(7_200)
    .sample_rate(0.5)
    .max_request_body(131_072)
    .max_response_body(524_288)
    .record_path("/api/orders")
    .record_path("/api/users")
    .skip_path("/health")
    .skip_path("/metrics")
    .redact_header("x-custom-secret")
    .redact_body_field("password")
    .redact_body_field("credit_card")
    .admin_route_prefix("/__admin/replays");
```

By default, these headers are stored as `[REDACTED]`:

- `authorization`
- `cookie`
- `x-api-key`
- `x-auth-token`

JSON body redaction works recursively; for example, a `password` field is masked even inside nested objects.

## Filesystem store for persistent retention

If you want the records to survive a developer-machine restart, use the filesystem store:

```rust,ignore
use rustapi_rs::extras::replay::{
    FsReplayStore, FsReplayStoreConfig, ReplayConfig, ReplayLayer,
};

let config = ReplayConfig::new()
    .enabled(true)
    .admin_token("local-replay-token");

let fs_store = FsReplayStore::new(FsReplayStoreConfig {
    directory: "./replay-data".into(),
    max_file_size: Some(10 * 1024 * 1024),
    create_if_missing: true,
});

let replay = ReplayLayer::new(config).with_store(fs_store);
```

## If you want to write a custom backend

If you want to use Redis, object storage, or an enterprise audit backend, implement the `ReplayStore` trait:

```rust,ignore
use async_trait::async_trait;
use rustapi_rs::extras::replay::{
    ReplayEntry, ReplayQuery, ReplayStore, ReplayStoreResult,
};

#[derive(Clone)]
struct MyCustomStore;

#[async_trait]
impl ReplayStore for MyCustomStore {
    async fn store(&self, entry: ReplayEntry) -> ReplayStoreResult<()> {
        let _ = entry;
        Ok(())
    }

    async fn get(&self, id: &str) -> ReplayStoreResult<Option<ReplayEntry>> {
        let _ = id;
        Ok(None)
    }

    async fn list(&self, query: &ReplayQuery) -> ReplayStoreResult<Vec<ReplayEntry>> {
        let _ = query;
        Ok(vec![])
    }

    async fn delete(&self, id: &str) -> ReplayStoreResult<bool> {
        let _ = id;
        Ok(false)
    }

    async fn count(&self) -> ReplayStoreResult<usize> {
        Ok(0)
    }

    async fn clear(&self) -> ReplayStoreResult<()> {
        Ok(())
    }

    async fn delete_before(&self, timestamp_ms: u64) -> ReplayStoreResult<usize> {
        let _ = timestamp_ms;
        Ok(0)
    }

    fn clone_store(&self) -> Box<dyn ReplayStore> {
        Box::new(self.clone())
    }
}
```

## Verification checklist

After setting up replay, run this short check:

1. send a request to the application
2. use `cargo rustapi replay list -t <token>` to confirm the entry appears
3. use `cargo rustapi replay show <id> -t <token>` to verify the stored body/header data
4. use `cargo rustapi replay diff <id> -t <token> -T <target>` to compare the results

If these four steps succeed, the workflow is ready.

## Security summary

The replay system includes several safeguards:

1. **Disabled by default**: it starts with `enabled(false)`.
2. **Admin token required**: admin endpoints require a bearer token.
3. **Header redaction**: sensitive headers are masked.
4. **Body field redaction**: JSON fields can be selectively masked.
5. **TTL enforced**: old records are cleaned up automatically.
6. **Body size limits**: request/response capture is size-limited.
7. **Bounded storage**: the in-memory store is limited with FIFO eviction.

Recommendations:

- do not enable replay behind a publicly exposed production ingress
- use a short TTL
- add application-specific secret fields to the redaction list
- monitor memory usage if you use a large-capacity in-memory store
- consider turning replay recording off after the incident
