# Production Checklist

Use this checklist before sending real traffic to a RustAPI service. Items marked **(auto)** are checked by `cargo rustapi doctor`.

**Related:** [Production Baseline](PRODUCTION_BASELINE.md) · [Deployment recipe](cookbook/src/recipes/deployment.md)

---

## Pre-deploy

### Toolchain

- [ ] Rust >= MSRV 1.85 (`rustc --version`)
- [ ] Release build tested locally (`cargo build --release`)
- [ ] `cargo test --workspace` passes in CI configuration
- [ ] `cargo clippy --workspace -- -D warnings` clean

### Configuration

- [ ] `RUSTAPI_ENV=production` set in runtime environment
- [ ] Secrets loaded from platform secret manager (not baked into image)
- [ ] `.env` files excluded from container images and git
- [ ] Service name set (`production_defaults("...")` or `RUSTAPI_SERVICE`)

### Health & readiness **(auto)**

- [ ] `/live` returns `200` under normal operation
- [ ] `/ready` returns `200` only when dependencies are healthy
- [ ] `/health` reflects expected dependency state
- [ ] Probe paths are **not** blocked by auth middleware
- [ ] Kubernetes/load-balancer probes point at `/ready`, not `/live`

### Production baseline **(auto)**

- [ ] `.production_defaults(...)` or equivalent manual setup:
  - [ ] Request ID middleware
  - [ ] Tracing middleware
  - [ ] Health/readiness/liveness endpoints
- [ ] Graceful shutdown configured (`on_shutdown` hooks or `run_with_shutdown`)
- [ ] Body size limits appropriate for your largest upload route

### Security

- [ ] 5xx errors masked in production (verify with `RUSTAPI_ENV=production`)
- [ ] JWT signing keys rotated and stored securely
- [ ] CORS origins restricted (not `allow_any_origin` in production)
- [ ] Rate limiting on authentication and public write endpoints
- [ ] Security headers enabled (`SecurityHeadersLayer`)
- [ ] Admin/dashboard/replay endpoints protected with `admin_token`
- [ ] Sensitive headers redacted in replay/audit logs

### Observability

- [ ] Structured logging configured (`extras-structured-logging` or platform agent)
- [ ] Request IDs visible in access logs
- [ ] Tracing exporter configured if using `extras-otel`
- [ ] Alerting on `/ready` failures and elevated 5xx rate
- [ ] Log retention and PII policy documented

### Resilience

- [ ] Timeouts on outbound calls and handler execution
- [ ] Circuit breaker on flaky downstream dependencies (if applicable)
- [ ] Retry policy defined for background jobs (not unbounded on user-facing paths)
- [ ] Connection pool limits sized for expected concurrency

---

## Deploy day

1. [ ] `GET /live` → `200`
2. [ ] `GET /ready` → `200`
3. [ ] `GET /health` → expected dependency status
4. [ ] At least one business endpoint succeeds end-to-end
5. [ ] Smoke test with production auth/CORS configuration
6. [ ] Rollback plan documented (previous image tag or blue/green switch)
7. [ ] Termination grace period ≥ drain window for in-flight requests

---

## Post-deploy

- [ ] Monitor error rate and latency for 30+ minutes
- [ ] Verify probes stay green under load
- [ ] Confirm logs/traces contain `error_id` on 5xx for correlation
- [ ] Run `cargo rustapi doctor --strict` against release branch (optional CI gate)

---

## RustAPI Cloud (managed hosting)

If deploying via RustAPI Cloud:

- [ ] `cargo rustapi login` completed
- [ ] `cargo rustapi whoami` shows expected account
- [ ] `cargo rustapi deploy cloud` succeeded
- [ ] `cargo rustapi deploy status <id>` shows `running` or `healthy`
- [ ] Public URL responds (pattern: `https://{project}-{user8}.rustapi.{domain}`)
- [ ] Cloud backend repo pinned to compatible version — see [RustAPI-Cloud](https://github.com/Tuntii/RustAPI-Cloud)

Full guide: [RustAPI Cloud recipe](cookbook/src/recipes/rustapi_cloud.md)

---

## CLI validation

```bash
# Toolchain + workspace signal scan
cargo rustapi doctor

# Fail CI on warnings
cargo rustapi doctor --strict
```

Doctor scans for: `production_defaults`, `RUSTAPI_ENV=production`, health endpoints, shutdown hooks, request IDs, tracing, structured logging, OpenTelemetry, rate limiting, security headers, timeouts, CORS, and body limits.

---

## License

MIT OR Apache-2.0, at your option.