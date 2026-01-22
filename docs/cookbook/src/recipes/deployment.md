# Deployment

RustAPI includes built-in deployment tooling to helping you ship your applications to production with ease. The `cargo rustapi deploy` command generates configuration files and provides instructions for various platforms.

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

