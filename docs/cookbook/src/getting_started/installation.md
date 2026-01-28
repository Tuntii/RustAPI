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

## Editor Setup

For the best experience, we recommend **VS Code** with the **rust-analyzer** extension. This provides:
- Real-time error checking
- Intelligent code completion
- In-editor documentation
