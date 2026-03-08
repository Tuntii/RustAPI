# Middleware Debugging

Middleware bugs are rarely glamorous. They usually look like:

- a handler never running,
- a missing `x-request-id`,
- tracing spans without correlation,
- an extractor failing because middleware never inserted the expected extension,
- a response being transformed by the “wrong” layer.

This guide focuses on debugging the middleware you already have in your stack.

## Problem

Middleware wraps handlers from the outside in, so when something goes wrong the visible symptom is often far away from the actual cause.

## Solution

Start with a minimal, observable stack and verify one layer at a time.

### Understand execution order first

RustAPI executes layers in the order they are added:

- the **first** `.layer(...)` sees the request first,
- the **last** `.layer(...)` sees the response first on the way back out.

```rust
use rustapi_rs::prelude::*;

#[rustapi_rs::get("/")]
async fn index() -> &'static str {
    "ok"
}

#[rustapi_rs::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    RustApi::auto()
        .layer(RequestIdLayer::new())
        .layer(
            TracingLayer::new()
                .with_field("service", "debug-demo")
                .with_field("environment", "development"),
        )
        .run("127.0.0.1:8080")
        .await
}
```

For the request path, the order is:

1. `RequestIdLayer`
2. `TracingLayer`
3. handler

For the response path, it unwinds in reverse.

## A practical debugging workflow

### 1. Verify request correlation

Start by confirming `RequestIdLayer` is active.

```bash
curl -i http://127.0.0.1:8080/
```

If the response does not include `x-request-id`, either:

- `RequestIdLayer` is missing,
- the request never reached that layer, or
- another layer or proxy is mutating headers unexpectedly.

### 2. Verify tracing sees the request ID

`TracingLayer` reads the request ID from request extensions. If it runs without `RequestIdLayer`, the span records `request_id = "unknown"`.

That makes the pairing easy to diagnose:

- `x-request-id` present + trace has request ID → good
- no `x-request-id` + trace shows `unknown` → missing request ID layer

### 3. Reduce the stack

If a handler is not reached, strip the app down to the smallest reproducer:

```rust
RustApi::new()
    .layer(RequestIdLayer::new())
    .route("/", get(index));
```

Then add layers back one by one until the failure returns. It is boring, but boring debugging is usually the fastest debugging.

### 4. Watch for short-circuiting

Some middleware returns a response early and never calls downstream layers or the handler. Common examples include:

- auth failures,
- timeout layers,
- CORS preflight handling,
- rate limits,
- custom guards.

If a request fails before the handler runs, suspect an outer layer first.

## Common failure modes

### `RequestId` extractor fails inside a handler

Symptom:

- handler returns an internal error saying the request ID was not found.

Likely cause:

- `RequestIdLayer` was not added.

### `Extension<T>` extractor fails

Symptom:

- handler says an extension was not found.

Likely cause:

- the middleware that should insert that extension never ran,
- it short-circuited before insertion,
- or the inserted type does not match the extracted type exactly.

### Logs exist but are hard to correlate

Add `RequestIdLayer` and keep `TracingLayer` close to the edge of the stack so every request has a stable identifier early.

### Response looks modified “too late”

Remember response processing unwinds in reverse. The last layer added has the first chance to modify the outgoing response.

## Built-in tools that help

### Status page

The built-in status page helps answer whether traffic is reaching the service and which endpoints are hot.

```rust
RustApi::auto().status_page();
```

### Observability stack

If the issue spans multiple services, combine:

- `RequestIdLayer`
- `TracingLayer`
- `OtelLayer`
- `StructuredLoggingLayer`

See the [Observability](observability.md) recipe for the recommended baseline.

### TestClient

For reproducible debugging, build a small app and exercise it with the in-memory test client. That way you can inspect middleware behavior without involving a real network hop.

## Debug checklist

- [ ] Does the response include `x-request-id`?
- [ ] Does tracing log the same request ID instead of `unknown`?
- [ ] Is the handler actually being reached?
- [ ] Could an outer middleware be short-circuiting?
- [ ] Is layer order what you think it is?
- [ ] If using `Extension<T>`, does the inserted type exactly match the extracted type?
- [ ] Have you reproduced the issue with a minimal stack?

## Related reading

- [Custom Middleware](custom_middleware.md)
- [Observability](observability.md)
- [Graceful Shutdown](graceful_shutdown.md)
- [Troubleshooting](../troubleshooting.md)