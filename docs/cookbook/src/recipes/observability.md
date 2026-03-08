# Observability

Production services need more than “logs exist somewhere”. A healthy RustAPI observability setup should let you answer three questions quickly:

1. **What failed?**
2. **Which request or trace did it belong to?**
3. **Is this isolated or systemic?**

This recipe shows a pragmatic observability stack using:

- `production_defaults(...)` for request IDs and request tracing,
- `OtelLayer` for distributed traces,
- `StructuredLoggingLayer` for machine-readable logs, and
- `InsightLayer` for in-process traffic analytics.

## Prerequisites

Enable the relevant features:

```toml
[dependencies]
rustapi-rs = { version = "0.1.335", features = [
  "core",
  "extras-otel",
  "extras-structured-logging",
  "extras-insight"
] }
```

## Basic Usage

```rust
use rustapi_rs::prelude::*;
use rustapi_rs::extras::{
    insight::{InsightConfig, InsightLayer},
    otel::{OtelConfig, OtelLayer},
    structured_logging::{LogOutputFormat, StructuredLoggingConfig, StructuredLoggingLayer},
};

#[rustapi_rs::get("/")]
async fn hello() -> &'static str {
    "hello"
}

#[rustapi_rs::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let environment = std::env::var("RUSTAPI_ENV")
        .unwrap_or_else(|_| "development".to_string());

    RustApi::auto()
        .production_defaults("billing-api")
        .layer(OtelLayer::new(
            OtelConfig::builder()
                .service_name("billing-api")
                .service_version(env!("CARGO_PKG_VERSION"))
                .deployment_environment(environment.clone())
                .endpoint("http://otel-collector:4317")
                .exclude_paths(vec![
                    "/health".to_string(),
                    "/ready".to_string(),
                    "/live".to_string(),
                ])
                .build(),
        ))
        .layer(StructuredLoggingLayer::new(
            StructuredLoggingConfig::builder()
                .format(LogOutputFormat::Json)
                .service_name("billing-api")
                .service_version(env!("CARGO_PKG_VERSION"))
                .environment(environment)
                .correlation_id_header("x-request-id")
                .exclude_paths(vec![
                    "/health".to_string(),
                    "/ready".to_string(),
                    "/live".to_string(),
                ])
                .build(),
        ))
        .layer(InsightLayer::with_config(
            InsightConfig::new()
                .sample_rate(0.20)
                .skip_paths(["/health", "/ready", "/live"])
                .header_whitelist(["content-type", "user-agent", "x-request-id"])
                .response_header_whitelist(["content-type", "x-request-id"])
                .dashboard_path(Some("/admin/insights"))
                .stats_path(Some("/admin/insights/stats")),
        ))
        .run("0.0.0.0:8080")
        .await
}
```

## The recommended “golden config”

For most APIs, the following defaults work well:

### 1. Request correlation everywhere

Use the production preset so every request already carries a request ID and tracing span. This gives you a stable correlation key before you add any external observability backend.

### 2. JSON logs in production

Prefer `StructuredLoggingLayer` with:

- `LogOutputFormat::Json`
- `service_name`
- `service_version`
- `environment`
- `correlation_id_header("x-request-id")`

That makes it easy to join app logs with request IDs emitted by the built-in preset.

### 3. OTel for distributed traces

Use `OtelLayer` when your service participates in a larger system. Set:

- service name,
- service version,
- deployment environment,
- collector endpoint,
- excluded probe paths.

### 4. Insight for local traffic intelligence

`InsightLayer` is useful for:

- endpoint hot spots,
- latency outliers,
- lightweight internal dashboards,
- short-term debugging without a full external analytics platform.

Use sampling in production and keep the dashboard on a private/admin route.

## What each layer is responsible for

| Layer | Purpose |
|-------|---------|
| `TracingLayer` (via production preset) | Request-scoped tracing spans with service metadata |
| `OtelLayer` | Distributed trace export and propagation |
| `StructuredLoggingLayer` | Machine-readable application/request logs |
| `InsightLayer` | In-process request analytics and dashboards |

These tools complement each other rather than replace each other.

## Noise control

Probe routes can dominate dashboards and logs in busy clusters. A good default is to exclude `/health`, `/ready`, and `/live` from:

- OTel export,
- structured logs, and
- insight capture.

If you need probe telemetry for a specific incident, re-enable it deliberately rather than keeping it on all the time.

## Sensitive data guidance

- Leave request/response body capture off unless debugging requires it.
- Whitelist only the headers you actually need.
- Keep `authorization`, `cookie`, and API-key style headers redacted.
- Treat admin insight endpoints as internal surfaces.

## Operational tips

1. Include `env!("CARGO_PKG_VERSION")` in logs and traces.
2. Make dashboards searchable by `x-request-id`, `trace_id`, and `error_id`.
3. Keep observability config close to your app bootstrap, not hidden in scattered helpers.
4. Validate the full path with one real request before rollout:
   - response has `X-Request-ID`,
   - logs include the correlation ID,
   - traces reach the collector,
   - insight dashboard records traffic if enabled.

## Related guides

- [Recommended Production Baseline](../../../PRODUCTION_BASELINE.md)
- [Production Checklist](../../../PRODUCTION_CHECKLIST.md)
- [Adaptive Execution Debug Plan](../../../ADAPTIVE_EXECUTION_DEBUG_PLAN.md)
- [Graceful Shutdown](graceful_shutdown.md)