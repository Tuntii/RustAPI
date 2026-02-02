# Installation

> [!NOTE]
> RustAPI is designed for Rust 1.75 or later.

## Prerequisites

Before we begin, ensure you have the Rust toolchain installed. If you haven't, the best way is via [rustup.rs](https://rustup.rs).

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Installing the CLI

RustAPI comes with a powerful CLI to scaffold projects. Install it directly from crates.io:

```bash
cargo install cargo-rustapi
```

Verify your installation:

```bash
cargo-rustapi --version
```

## Adding to an Existing Project

If you prefer not to use the CLI, you can add RustAPI to your `Cargo.toml` manually:

```bash
cargo add rustapi-rs@0.1.233
```

Or add this to your `Cargo.toml`:

```toml
[dependencies]
rustapi-rs = "0.1.233"
```

## Editor Setup

For the best experience, we recommend **VS Code** with the **rust-analyzer** extension. This provides:
- Real-time error checking
- Intelligent code completion
- In-editor documentation
