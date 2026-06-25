# Production Baseline

Recommended defaults for running RustAPI in production. This document describes what `.production_defaults()` enables and how to extend it for real workloads.

**Related:** [Production Checklist](PRODUCTION_CHECKLIST.md) · [Deployment recipe](cookbook/src/recipes/deployment.md) · [Observability recipe](cookbook/src/recipes/observability.md)

---

## One-call baseline

The fastest path to a production-shaped service:

```rust
use rustapi_rs::prelude::*;

#[rustapi_rs::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    RustApi::auto()
        .production_defaults("my-service")
        .run("0.0.0.0:8080")
        .await
}
```

`production_defaults(name)` enables:

| Capability | What it does |
|------------|--------------|
| **Request IDs** | `RequestIdLayer` — every response gets a correlatable ID |
| **Tracing** | `TracingLayer` — structured spans per request |
| **Health probes** | `/health`, `/ready`, `/live` without hand-written handlers |
| **Service metadata** | Service name attached to logs and health responses |

For full control, use `.production_defaults_with_config(ProductionDefaultsConfig::new()...)`.

---

## Environment variables

| Variable | Purpose | Production value |
|----------|---------|------------------|
| `RUSTAPI_ENV` | Error masking for 5xx responses | `production` |
| `RUST_LOG` | Log filter (when tracing subscriber is configured) | `info` or `warn` |
| `RUSTAPI_SERVICE` | Override service name in logs/health | Your service name |

In `production`, internal error details are masked to `"An internal error occurred"`. Validation errors (4xx) pass through unchanged. Every 5xx includes an `error_id` (`err_{uuid}`) for log correlation.

---

## Probe semantics

| Endpoint | Question it answers | Routing guidance |
|----------|---------------------|------------------|
| `/live` | Is the process alive? | Use for liveness and startup probes |
| `/ready` | Should this instance receive traffic? | Point load balancers here |
| `/health` | What is aggregate dependency health? | Dashboards and ops tooling |

**Rules of thumb:**

- Keep `/live` lightweight — no database calls.
- Make `/ready` fail when critical dependencies are down or during graceful drain.
- Use `/health` for richer dependency diagnostics.

Customize paths with `HealthEndpointConfig` if your platform requires different URLs.

---

## Recommended middleware stack

Beyond `production_defaults()`, most production APIs add:

```rust
use rustapi_rs::extras::cors::CorsLayer;
use rustapi_rs::extras::rate_limit::{RateLimitLayer, RateLimitStrategy};
use rustapi_rs::extras::security_headers::SecurityHeadersLayer;
use rustapi_rs::extras::timeout::TimeoutLayer;
use rustapi_rs::prelude::*;

RustApi::auto()
    .production_defaults("billing-api")
    .layer(CorsLayer::new().allow_any_origin()) // tighten for production
    .layer(SecurityHeadersLayer::new())
    .layer(TimeoutLayer::new(std::time::Duration::from_secs(30)))
    .layer(RateLimitLayer::new(100).strategy(RateLimitStrategy::SlidingWindow))
```

| Layer | Why |
|-------|-----|
| `CorsLayer` | Browser clients |
| `SecurityHeadersLayer` | HSTS, CSP, X-Frame-Options defaults |
| `TimeoutLayer` | Prevent hung handlers from tying up workers |
| `RateLimitLayer` | Abuse protection on public endpoints |
| `BodyLimitLayer` | Default 1 MB; tune per upload routes |

Use `cargo rustapi new my-api --preset prod-api` to scaffold a project with many of these features pre-selected.

---

## Dependency-aware readiness

When your service depends on a database or cache, wire checks into `HealthCheckBuilder`:

```rust
let health = HealthCheckBuilder::new(true)
    .add_check("database", || async {
        // ping your pool; return HealthStatus::unhealthy("...") on failure
        HealthStatus::healthy()
    })
    .build();

RustApi::auto()
    .with_health_check(health)
    .production_defaults("users-api")
    .run("0.0.0.0:8080")
    .await?;
```

`/ready` returns `503` when any registered check is unhealthy.

---

## Graceful shutdown

Register shutdown hooks for connection draining:

```rust
RustApi::auto()
    .production_defaults("users-api")
    .on_shutdown(|| async {
        // flush buffers, close pools, stop background workers
    })
    .run_with_shutdown("0.0.0.0:8080", shutdown_signal())
    .await?;
```

All `run*` entrypoints (`run`, `run_http3`, `run_dual_stack`, and `*_with_shutdown` variants) execute `on_shutdown` hooks after the server exits.

---

## Observability baseline

| Signal | RustAPI primitive |
|--------|-------------------|
| Request correlation | `RequestIdLayer` (in `production_defaults`) |
| Distributed tracing | `TracingLayer` + optional `extras-otel` |
| Metrics | `MetricsLayer` + `/metrics` when enabled |
| Structured logs | `extras-structured-logging` |
| Admin visibility | `core-dashboard` + optional `extras-replay` |

See the [Observability recipe](cookbook/src/recipes/observability.md) for OpenTelemetry wiring and dashboard setup.

---

## Security baseline

| Concern | Recommendation |
|---------|----------------|
| Secrets | Never commit `.env`; use your platform's secret manager |
| JWT | Rotate signing keys; use `JwtLayer::skip_paths` for public routes |
| Admin surfaces | Protect `/__rustapi/dashboard` and replay APIs with `admin_token` |
| CSRF | Enable for cookie-session apps — see [CSRF recipe](cookbook/src/recipes/csrf_protection.md) |
| Error leakage | Set `RUSTAPI_ENV=production` |

---

## Validate before deploy

Run the CLI doctor against your project:

```bash
cargo rustapi doctor --strict
```

Doctor checks toolchain availability and scans your workspace for production signals (`production_defaults`, health endpoints, shutdown hooks, rate limiting, etc.). See [Production Checklist](PRODUCTION_CHECKLIST.md) for the full manual list.

---

## Deployment options

| Path | When to use |
|------|-------------|
| [Self-hosted (Docker/K8s)](cookbook/src/recipes/deployment.md) | Full control, existing infra |
| [RustAPI Cloud](cookbook/src/recipes/rustapi_cloud.md) | Managed hosting via `cargo rustapi deploy cloud` |
| [Fly.io / Railway / Shuttle](cookbook/src/recipes/deployment.md) | Platform-specific generated configs |

---

## License

MIT OR Apache-2.0, at your option.