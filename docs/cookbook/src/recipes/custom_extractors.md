# Custom Extractors

Custom extractors let you move repetitive request parsing out of handlers and into reusable, typed building blocks.

Use them when a handler keeps repeating logic like:

- reading a required header,
- validating a tenant or region identifier,
- parsing a plain-text or binary body,
- loading middleware-injected context from request extensions.

## Problem

Inline parsing works for one endpoint, but quickly becomes noisy when multiple handlers repeat the same header/body checks.

## Solution

RustAPI exposes two traits for custom extraction:

- `FromRequestParts` for headers, path params, query params, extensions, and state
- `FromRequest` for extractors that must consume the request body

If the extractor does **not** need the body, prefer `FromRequestParts`.

### Example 1: Header-backed tenant extractor

```rust
use rustapi_rs::prelude::*;

#[derive(Debug, Clone)]
struct TenantId(String);

impl TenantId {
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromRequestParts for TenantId {
    fn from_request_parts(req: &Request) -> Result<Self> {
        let header = HeaderValue::extract(req, "x-tenant-id")
            .map_err(|_| ApiError::bad_request("Missing x-tenant-id header"))?;

        let tenant = header.value().trim();
        if tenant.is_empty() {
            return Err(ApiError::bad_request("x-tenant-id cannot be empty"));
        }

        Ok(TenantId(tenant.to_string()))
    }
}

#[derive(Serialize, Schema)]
struct ProjectList {
    tenant: String,
    items: Vec<String>,
}

#[rustapi_rs::get("/projects")]
async fn list_projects(tenant: TenantId) -> Json<ProjectList> {
    Json(ProjectList {
        tenant: tenant.as_str().to_string(),
        items: vec!["alpha".into(), "beta".into()],
    })
}
```

### Example 2: Plain-text body extractor

When you need to consume the request body yourself, implement `FromRequest` instead.

```rust
use rustapi_rs::prelude::*;

#[derive(Debug)]
struct PlainTextBody(String);

impl PlainTextBody {
    fn into_inner(self) -> String {
        self.0
    }
}

impl FromRequest for PlainTextBody {
    async fn from_request(req: &mut Request) -> Result<Self> {
        req.load_body().await?;

        let body = req
            .take_body()
            .ok_or_else(|| ApiError::internal("Body already consumed"))?;

        let text = String::from_utf8(body.to_vec())
            .map_err(|_| ApiError::bad_request("Request body must be valid UTF-8"))?;

        Ok(PlainTextBody(text))
    }
}

#[derive(Serialize, Schema)]
struct EchoResponse {
    content: String,
}

#[rustapi_rs::post("/echo-text")]
async fn echo_text(body: PlainTextBody) -> Json<EchoResponse> {
    Json(EchoResponse {
        content: body.into_inner(),
    })
}
```

## Discussion

### Pick the right trait

Use `FromRequestParts` when you only need request metadata:

- headers,
- query string,
- path parameters,
- request extensions,
- shared state.

Use `FromRequest` only when you must consume the body.

### Body-consuming extractors still must come last

This rule applies to your custom body extractors too.

```rust
async fn create_note(
    State(app): State<AppState>,
    tenant: TenantId,
    body: PlainTextBody, // body-consuming extractor goes last
) -> Result<Json<NoteResponse>> {
    # let _ = (&app, tenant, body);
    # todo!()
}
```

### Middleware + extractors fit together nicely

If middleware inserts typed data into request extensions, a custom extractor can read it back using the same `FromRequestParts` pattern. That keeps handlers clean and avoids repeated extension lookups.

### Error style

Return `ApiError` from your extractor when extraction fails. That keeps rejection behavior consistent with built-in extractors.

## Testing

Quick manual checks:

```bash
curl -i http://127.0.0.1:8080/projects
curl -i -H "x-tenant-id: acme" http://127.0.0.1:8080/projects
curl -i -X POST http://127.0.0.1:8080/echo-text -H "content-type: text/plain" --data "hello"
```

Expected outcomes:

- missing `x-tenant-id` returns `400`,
- valid header returns a JSON payload containing the tenant,
- plain-text echo returns the posted content as JSON.

## Related reading

- [Handlers & Extractors](../concepts/handlers.md)
- [Troubleshooting](../troubleshooting.md)
- [JWT Authentication](jwt_auth.md)