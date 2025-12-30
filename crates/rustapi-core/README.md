# RustAPI Core

The core engine for the RustAPI framework.

> **Note**: This is an internal crate. You should depend on `rustapi-rs` instead.

## Responsibilities

- **HTTP Server**: Wraps `hyper` 1.0.
- **Routing**: Implements radix-tree routing via `matchit`.
- **Glue Code**: Connects extractors, handlers, and responses.
- **Middleware Integration**: Provides `tower` compatibility (internal).

## Architecture

This crate provides the `RustApi` builder and the `Handler` trait system that powers the framework's ergonomics.
