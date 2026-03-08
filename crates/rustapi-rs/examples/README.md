# RustAPI Examples

This directory contains the in-repository examples for the `rustapi-rs` facade crate.

## Available examples

### `auth_api`

Shows cookie-backed login, session refresh, logout, and session inspection using the built-in session middleware.

Run it with:

```sh
cargo run -p rustapi-rs --example auth_api --features extras-session
```

Then try:

- `POST http://127.0.0.1:3000/auth/login` with `{"user_id":"demo-user"}`
- `GET http://127.0.0.1:3000/auth/me`
- `POST http://127.0.0.1:3000/auth/refresh`
- `POST http://127.0.0.1:3000/auth/logout`

### `full_crud_api`

Shows a compact in-memory CRUD API with list/create/read/update/delete routes.

Run it with:

```sh
cargo run -p rustapi-rs --example full_crud_api
```

Then try:

- `GET http://127.0.0.1:3000/todos`
- `POST http://127.0.0.1:3000/todos`
- `GET http://127.0.0.1:3000/todos/1`
- `PATCH http://127.0.0.1:3000/todos/1`
- `DELETE http://127.0.0.1:3000/todos/1`

### `streaming_api`

Shows Server-Sent Events (SSE) with a small progress feed.

Run it with:

```sh
cargo run -p rustapi-rs --example streaming_api
```

Then open:

- `http://127.0.0.1:3000/events`

### `jobs_api`

Shows an in-memory job queue, enqueue endpoint, and manual worker tick endpoint.

Run it with:

```sh
cargo run -p rustapi-rs --example jobs_api --features extras-jobs
```

Then try:

- `POST http://127.0.0.1:3000/jobs/email`
- `POST http://127.0.0.1:3000/jobs/process-next`
- `GET http://127.0.0.1:3000/jobs/stats`

### `typed_path_poc`

Shows typed path definitions, type-safe route registration, and URI generation with `TypedPath`.

Run it with:

```sh
cargo run -p rustapi-rs --example typed_path_poc
```

### `status_demo`

Shows the automatic status page and a few endpoints that generate traffic, latency, and failures for demonstration purposes.

Run it with:

```sh
cargo run -p rustapi-rs --example status_demo
```

Then open:

- `http://127.0.0.1:3000/status`
- `http://127.0.0.1:3000/fast`
- `http://127.0.0.1:3000/slow`
- `http://127.0.0.1:3000/flaky`

## Notes

- Keep this file aligned with the actual `.rs` files in this directory.
- User-facing examples should import from `rustapi_rs::prelude::*` unless the example is explicitly about internals.
- Additional example ideas tracked in `tasks.md` are roadmap items until their files exist here.
