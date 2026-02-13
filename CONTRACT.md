# RustAPI Contract

This document defines compatibility guarantees for the RustAPI workspace.

## 1. Scope

- Stable public contract:
  - `rustapi-rs` (facade crate)
  - `cargo-rustapi` (CLI surface)
- Internal implementation crates (best-effort, no stability guarantee):
  - `rustapi-core`, `rustapi-openapi`, `rustapi-validate`, `rustapi-macros`
  - `rustapi-extras`, `rustapi-ws`, `rustapi-toon`, `rustapi-view`, `rustapi-grpc`
  - `rustapi-testing`, `rustapi-jobs`

Do not depend on internal crate APIs for long-term compatibility.

## 2. SemVer Policy

- `rustapi-rs` follows strict SemVer:
  - Breaking changes: major
  - Additive changes: minor
  - Fixes/internal-only changes: patch
- Public API surface is tracked by committed snapshots:
  - `api/public/rustapi-rs.default.txt`
  - `api/public/rustapi-rs.all-features.txt`
- Pull requests that change snapshots must be labeled:
  - `breaking` (if compatibility breaks)
  - `feature` (if additive API surface change)

## 3. MSRV Policy

- Workspace MSRV is pinned to Rust `1.78`.
- MSRV increases are allowed only in minor or major releases.
- MSRV changes must be called out in changelog/release notes.
- Patch releases must not raise MSRV.

## 4. Deprecation Policy

- Deprecations are soft-first:
  - `#[deprecated]` attribute
  - explicit migration path in docs/release notes
- Minimum deprecation window before removal: 2 minor releases.
- Removals occur only in major releases.
- Current compatibility window for legacy aliases introduced in this cycle:
  - First eligible removal: `v0.3.0` (assuming deprecation introduced in `v0.1.x`).

## 5. Feature Flag Policy

- New facade feature names must follow taxonomy:
  - `core-*`
  - `protocol-*`
  - `extras-*`
- Meta features:
  - `core` (default)
  - `protocol-all`
  - `extras-all`
  - `full`
- Legacy aliases may exist temporarily for migration but must be treated as deprecated and eventually removed on a published timeline.
- Published timeline for this migration set:
  - `v0.1.x`: aliases available, deprecation warnings/documentation.
  - `v0.2.x`: aliases still available, migration reminders.
  - `v0.3.0+`: aliases may be removed.

## 6. Internal Leakage Rule

- Public facade signatures should not expose internal crate paths.
- Macro/runtime internals are allowed only via `rustapi_rs::__private` and are excluded from stability guarantees.
