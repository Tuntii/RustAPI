# RustAPI Cloud

Deploy RustAPI applications to managed hosting with a single CLI workflow. The **framework and CLI** live in this repository; the **cloud backend** (OAuth, storage, nginx routing, deploy pipeline) lives in the separate [RustAPI-Cloud](https://github.com/Tuntii/RustAPI-Cloud) repository.

---

## Overview

| Component | Repository | Role |
|-----------|------------|------|
| `rustapi-rs` + `cargo-rustapi` | [Tuntii/RustAPI](https://github.com/Tuntii/RustAPI) | Framework, CLI, `deploy cloud` client |
| RustAPI Cloud backend | [Tuntii/RustAPI-Cloud](https://github.com/Tuntii/RustAPI-Cloud) | Auth, build pipeline, HTTPS routing, app hosting |

**Default cloud API:** `https://api.rustapi.cloud`

Self-hosted operators can run their own cloud backend from RustAPI-Cloud and point the CLI with `--cloud-url`.

---

## Prerequisites

- Rust 1.85+ and a working `cargo` toolchain
- `cargo-rustapi` installed (`cargo install cargo-rustapi`)
- A RustAPI project with a release binary (the CLI builds `cargo build --release` automatically)
- Cloud feature enabled (on by default in `cargo-rustapi`)

---

## Quick start

```bash
# 1. Authenticate (device-code OAuth — opens browser)
cargo rustapi login

# 2. Verify session
cargo rustapi whoami

# 3. Deploy from project root (builds, packages, uploads)
cargo rustapi deploy cloud

# 4. Poll deploy progress (use ID from deploy output)
cargo rustapi deploy status <deploy-id>
```

After a successful deploy, your app is available at a public HTTPS URL:

```
https://{project}-{user8}.rustapi.{domain}
```

- `{project}` — Cargo package name or `--name` override
- `{user8}` — first 8 characters of your GitHub user id
- `{domain}` — configured on the cloud backend (managed default: `rustapi.cloud`)

---

## Authentication

### `cargo rustapi login`

Device-code OAuth flow (RFC 8628):

```bash
cargo rustapi login
cargo rustapi login --cloud-url https://api.rustapi.cloud
cargo rustapi login --no-browser   # print URL only, no auto-open
```

**What happens:**

1. CLI requests a device code from `{cloud_url}/auth/device`
2. You open the verification URL and enter the user code
3. CLI polls `{cloud_url}/auth/token` until authorized
4. Access + refresh tokens saved to `~/.rustapi/config.json` (or `RUSTAPI_CONFIG_PATH`)

### `cargo rustapi whoami`

Prints the logged-in GitHub username, tier, and cloud URL from local config.

### `cargo rustapi logout`

Removes local credentials. Does not revoke tokens server-side.

---

## Deploy

### `cargo rustapi deploy cloud`

Builds a release binary, packages it, and uploads to RustAPI Cloud.

```bash
cargo rustapi deploy cloud
cargo rustapi deploy cloud --name my-api
cargo rustapi deploy cloud --no-wait   # return immediately after upload
```

**Pipeline steps (server-side, in RustAPI-Cloud):**

1. Receive upload and queue deploy job
2. Extract binary and configure runtime
3. Route traffic via nginx wildcard vhost
4. Expose HTTPS URL

### `cargo rustapi deploy status <deploy-id>`

Poll deploy job state:

```bash
cargo rustapi deploy status abc123-def456
```

Typical states: `queued` → `building` → `deploying` → `running` / `failed`.

Use this when `--no-wait` was passed or when checking a long-running deploy from CI.

---

## Configuration

### Local credentials

Default path:

| OS | Path |
|----|------|
| Linux/macOS | `~/.rustapi/config.json` |
| Windows | `%USERPROFILE%\.rustapi\config.json` |

Schema:

```json
{
  "token": "<access-token>",
  "refresh_token": "<refresh-token>",
  "user": {
    "login": "your-github-username",
    "tier": "hobby",
    "avatar_url": "https://..."
  },
  "last_login": "2026-06-25T12:00:00Z",
  "cloud_url": "https://api.rustapi.cloud"
}
```

### `RUSTAPI_CONFIG_PATH`

Override config file location — useful for CI, multiple profiles, or isolated tests:

```bash
export RUSTAPI_CONFIG_PATH=/tmp/rustapi-ci-config.json
cargo rustapi deploy cloud
```

The CLI creates parent directories when saving config.

---

## Cloud feature flag

Cloud HTTP commands are gated behind the `cloud` feature on `cargo-rustapi` (enabled by default):

```toml
# Cargo.toml — disable cloud commands
[dependencies]
cargo-rustapi = { version = "0.1.550", default-features = false }
```

When disabled, `login`, `deploy cloud`, and `deploy status` are not compiled. Workspace builds with `--no-default-features` remain supported.

---

## Self-hosted cloud backend

To run your own RustAPI Cloud instance:

1. Clone [Tuntii/RustAPI-Cloud](https://github.com/Tuntii/RustAPI-Cloud)
2. Follow its `install.sh` / docker-compose setup
3. Point CLI at your API:

```bash
cargo rustapi login --cloud-url https://cloud.example.com
cargo rustapi deploy cloud
```

Backend configuration (ports, Postgres, wildcard TLS, nginx templates) is documented in the RustAPI-Cloud repository.

---

## Production recommendations

Before deploying to cloud:

1. Enable `.production_defaults("your-service")` — see [Production Baseline](../../PRODUCTION_BASELINE.md)
2. Set `RUSTAPI_ENV=production`
3. Run `cargo rustapi doctor --strict`
4. Complete the [Production Checklist](../../PRODUCTION_CHECKLIST.md)

For self-managed infra (Docker, K8s, Fly.io), see [Deployment](deployment.md) instead.

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `Not logged in` | Run `cargo rustapi login` |
| Device code expired | Re-run `cargo rustapi login` |
| Deploy times out | Check `cargo rustapi deploy status <id>`; verify cloud backend health |
| Wrong cloud instance | `logout`, then `login --cloud-url <correct-url>` |
| CI needs isolated creds | Set `RUSTAPI_CONFIG_PATH` to a temp file with test tokens |
| `cloud` commands missing | Reinstall with default features: `cargo install cargo-rustapi` |

---

## Architecture (high level)

```text
Developer machine                    RustAPI Cloud backend
┌─────────────────────┐             ┌──────────────────────────┐
│ cargo rustapi login │──OAuth─────▶│ /auth/device, /auth/token│
│ cargo rustapi       │             │                          │
│   deploy cloud      │──upload────▶│ /deploy                  │
│ cargo rustapi       │             │   → build queue          │
│   deploy status     │◀──poll──────│   → nginx wildcard route │
└─────────────────────┘             │   → HTTPS public URL     │
                                    └──────────────────────────┘
```

---

## Related

- [cargo-rustapi deep dive](../crates/cargo_rustapi.md)
- [Deployment (self-hosted)](deployment.md)
- [Production Baseline](../../PRODUCTION_BASELINE.md)
- [RustAPI-Cloud repository](https://github.com/Tuntii/RustAPI-Cloud)