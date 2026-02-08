# RustAPI Release History

## v0.1.333 - Quick Wins + Must-Have Completion (2026-02-08)

This release combines dependency surface cleanup, runtime completions, and documentation alignment in one focused quick-wins iteration.

### Highlights

- **True dual-stack runtime completed**: `RustApi::run_dual_stack` now runs HTTP/1.1 (TCP) and HTTP/3 (QUIC/UDP) simultaneously.
- **WebSocket permessage-deflate negotiation completed**: real extension parsing and parameter negotiation added for `Sec-WebSocket-Extensions`.
- **OpenAPI ref integrity coverage strengthened**: component traversal validation now includes response/requestBody/header/callback paths with tests.
- **Async validation context from app state**: `AsyncValidatedJson` now uses state-provided `ValidationContext` behavior with verified coverage.
- **Native OpenAPI + validation documentation alignment**: architecture docs are synced to OpenAPI 3.1 and v2-native validation direction.
- **Dependency footprint reduced (quick wins)**: unused/overly broad dependencies and feature sets were tightened, reducing lockfile surface.

### Technical Details

- `crates/rustapi-core/src/app.rs`: `run_dual_stack` implementation
- `crates/rustapi-core/src/server.rs`: `Server::from_shared` for shared app state
- `crates/rustapi-ws/src/upgrade.rs`: permessage-deflate parsing/negotiation
- `crates/rustapi-ws/src/compression.rs`: negotiation test updates
- `crates/rustapi-openapi/src/tests.rs`: reference traversal coverage test
- `docs/ARCHITECTURE.md`, `docs/cookbook/src/architecture/system_overview.md`, `crates/rustapi-openapi/README.md`: architecture/docs alignment

### Validation

- `cargo test -p rustapi-openapi`
- `cargo test -p rustapi-ws`
- `cargo test -p rustapi-core test_async_validated_json_with_state_context`
- `cargo check -p rustapi-core --features http3`

### Commit References

- `ca238ac` chore(quick-wins): reduce dependency surface and align native OpenAPI docs
- `dcb0e8b` feat(core/ws/openapi): complete quick-wins must-haves

---

## v0.1.300 - Time-Travel Debugging (2026-02-06)

- Replay system (record/replay/diff)
- Admin API + CLI support
- Security (token auth, redaction, TTL)

## v0.1.202 - Performance Revolution (2026-01-26)

- Broad performance optimizations in server and JSON layers
- Benchmark improvements and release profile tuning
