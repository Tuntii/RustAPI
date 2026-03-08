# Deployment

RustAPI includes built-in deployment tooling to help you ship applications, but production deployment is more than generating a config file. This guide covers both the CLI-assisted setup and the operational recommendations for health, readiness, liveness, and rollout safety.

## Supported Platforms

- **Docker**: Generate a production-ready `Dockerfile`.
- **Fly.io**: Generate `fly.toml` and deploy instructions.
- **Railway**: Generate `railway.toml` and project setup.
- **Shuttle.rs**: Generate `Shuttle.toml` and setup instructions.

## Usage

### Docker

Generate a `Dockerfile` optimized for RustAPI applications:

```bash
cargo rustapi deploy docker
```

Options:
- `--output <path>`: Output path (default: `./Dockerfile`)
- `--rust-version <ver>`: Rust version (default: 1.78)
- `--port <port>`: Port to expose (default: 8080)
- `--binary <name>`: Binary name (default: package name)

### Fly.io

Prepare your application for Fly.io:

```bash
cargo rustapi deploy fly
```

Options:
- `--app <name>`: Application name
- `--region <region>`: Fly.io region (default: iad)
- `--init_only`: Only generate config, don't show deployment steps

### Railway

Prepare your application for Railway:

```bash
cargo rustapi deploy railway
```

Options:
- `--project <name>`: Project name
- `--environment <env>`: Environment name (default: production)

### Shuttle.rs

Prepare your application for Shuttle.rs serverless deployment:

```bash
cargo rustapi deploy shuttle
```

Options:
- `--project <name>`: Project name
- `--init_only`: Only generate config

> **Note**: Shuttle.rs requires some code changes to use their runtime macro `#[shuttle_runtime::main]`. The deploy command generates the configuration but you will need to adjust your `main.rs` to use their attributes if you are deploying to their platform.

## Probe recommendations

RustAPI has first-class built-in probe endpoints:

- `/health` — aggregate service and dependency health
- `/ready` — readiness for load balancers and orchestrators
- `/live` — lightweight liveness probe

You can enable them via:

- `.health_endpoints()`
- `.with_health_check(...)`
- `.production_defaults("service-name")`

### Recommended semantics

- **Liveness** should answer: “Is the process alive?”
- **Readiness** should answer: “Should this instance receive traffic right now?”
- **Health** should answer: “What is the aggregate state of the service and its dependencies?”

In practice:

- let `/live` stay lightweight,
- let `/ready` fail when critical dependencies fail,
- let `/ready` also fail during drain/shutdown windows,
- use `/health` for richer diagnostics and dashboards.

## Kubernetes example

```yaml
livenessProbe:
	httpGet:
		path: /live
		port: 8080
	initialDelaySeconds: 5
	periodSeconds: 10

readinessProbe:
	httpGet:
		path: /ready
		port: 8080
	initialDelaySeconds: 2
	periodSeconds: 5

startupProbe:
	httpGet:
		path: /live
		port: 8080
	failureThreshold: 30
	periodSeconds: 2
```

If you customize the paths with `HealthEndpointConfig`, update the probe configuration to match.

## Load balancer and ingress guidance

- Point traffic-routing health checks at `/ready`, not `/live`.
- Keep the drain window consistent with your termination grace period.
- Avoid routing public traffic to admin/debug surfaces such as `/status`, `/docs`, or `/admin/insights` unless intentionally protected.
- If auth middleware protects most routes, make sure probe routes remain reachable.

## Minimal production bootstrap

```rust
use rustapi_rs::prelude::*;

#[rustapi_rs::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
		RustApi::auto()
				.production_defaults("users-api")
				.run("0.0.0.0:8080")
				.await
}
```

If you need dependency-aware readiness, supply your own `HealthCheck`:

```rust
use rustapi_rs::prelude::*;

let health = HealthCheckBuilder::new(true)
		.add_check("database", || async {
				HealthStatus::healthy()
		})
		.build();

let app = RustApi::new().with_health_check(health);
```

## Rollout checklist

Before sending real traffic:

1. `GET /live` returns `200`.
2. `GET /ready` returns `200`.
3. `GET /health` shows expected dependency state.
4. At least one business endpoint succeeds.
5. Logs and traces contain request IDs and service metadata.

For the full operational list, see [Production Checklist](../../../PRODUCTION_CHECKLIST.md).

