# Performance Benchmarks

This document is the **authoritative source** for public RustAPI performance claims.

If `README.md`, cookbook pages, release notes, or other user-facing docs mention throughput or latency numbers, they should either:

- link to this file, or
- explicitly identify themselves as a historical point-in-time snapshot.

## Current status

RustAPI has benchmark automation entry points in the repository, but the public docs should avoid treating older point-in-time numbers as the current universal baseline.

Today that means:

- historical benchmark numbers in older release notes are kept for historical context,
- current public docs should prefer qualitative performance guidance plus this benchmark source,
- new absolute claims should only be published here after a fresh, reproducible run with raw output attached.

## Canonical benchmark entry points

### Local

Use the repository benchmark script:

```powershell
./scripts/bench.ps1
```

This currently runs:

```powershell
cargo bench --workspace
cargo run -p rustapi-core --example perf_snapshot --release
```

### CI

The repository also includes a manual benchmark workflow:

- `.github/workflows/benchmark.yml`

That workflow runs `cargo bench --workspace` and uploads the raw output as `benchmark_results.txt`.

It also runs `cargo run -p rustapi-core --example perf_snapshot --release` and uploads the raw output as `perf_snapshot.txt`.

## Latest validated snapshot

The following snapshot was generated in this session from a real local run.

### Environment

- **Date**: 2026-07-05
- **CPU**: AMD Ryzen 7 4800H with Radeon Graphics
- **OS**: Microsoft Windows 10 Home
- **Rust**: `rustc 1.91.0 (f8297e351 2025-10-28)` (workspace MSRV 1.85)
- **Cargo**: `cargo 1.91.0 (ea2d97820 2025-10-10)`
- **Command**: `cargo run -p rustapi-core --example perf_snapshot --release`
- **Warmup iterations**: `2000`
- **Measured iterations**: `20000`
- **Feature context**: `rustapi-core` default features (`swagger-ui`, `tracing`)
- **Workload**: synthetic in-process request pipeline benchmark for a static `GET /hello` route

### Latency outputs and feature-cost matrix

| Scenario | Execution path | Features | Req/s | Mean (µs) | p50 (µs) | p95 (µs) | p99 (µs) |
|---|---|---|---:|---:|---:|---:|---:|
| `baseline` | ultra fast | no middleware, no interceptors | 1,631,774 | 0.50 | 0.50 | 0.60 | 2.20 |
| `request_interceptor` | fast | 1 request interceptor | 1,602,654 | 0.53 | 0.50 | 0.50 | 2.20 |
| `request_response_interceptors` | fast | 1 request + 1 response interceptor | 1,238,206 | 0.69 | 0.70 | 0.90 | 2.50 |
| `middleware_only` | full | 1 middleware layer | 631,309 | 1.45 | 1.30 | 2.60 | 3.20 |
| `full_stack_minimal` | full | 1 middleware + 1 request + 1 response interceptor | 937,941 | 0.97 | 0.90 | 2.00 | 2.70 |
| `request_id_layer` | full | `RequestIdLayer` | 426,398 | 2.21 | 2.00 | 3.80 | 3.90 |

### Relative overhead vs baseline

| Scenario | Req/s delta | p99 delta |
|---|---:|---:|
| `baseline` | +0.00% | +0.00% |
| `request_interceptor` | -1.78% | +0.00% |
| `request_response_interceptors` | -24.12% | +13.64% |
| `middleware_only` | -61.31% | +45.45% |
| `full_stack_minimal` | -42.52% | +22.73% |
| `request_id_layer` | -73.87% | +77.27% |

## Execution path comparison

This snapshot confirms the intended three-tier execution model:

- **Ultra fast** path remains the cheapest route for static handler execution with no middleware or interceptors.
- **Fast** path adds modest overhead for interceptors without dropping into the full middleware stack.
- **Full** path is measurably more expensive, especially once real middleware such as `RequestIdLayer` is added.

Because this benchmark is synthetic and in-process, treat it as a **framework pipeline cost snapshot**, not as an end-to-end HTTP server benchmark or a cross-framework comparison.

## Publishing rules for new benchmark claims

Before adding or updating public performance numbers, capture all of the following in the benchmark record:

- hardware (CPU, RAM)
- OS
- Rust toolchain version
- benchmark command
- scenario/workload description
- enabled feature flags
- request rate / throughput metric
- latency distribution, including $p50$, $p95$, and $p99$
- memory footprint, if reported

If a claim does not include enough metadata to be reproduced, it should not be treated as canonical.

## Historical notes

Some existing changelog entries include benchmark numbers from older runs. Those are still useful as release-history context, but they are **not** the canonical current baseline unless they are linked back from this document.

In particular, the `0.1.202` changelog entry records a Windows 11 / Ryzen 9 5900X snapshot. Treat it as a historical benchmark note, not the current authoritative cross-framework comparison.

## Guidance for other docs

- `README.md` should link here instead of embedding standalone req/s claims.
- Performance-focused cookbook pages should explain **how** RustAPI stays fast and point here for benchmark publication policy.
- Release notes may summarize benchmark-related improvements, but they should cite this document for the benchmark source of truth.

## Still intentionally open

The following performance work remains open:

- broader end-to-end benchmark scenarios beyond the synthetic in-process pipeline snapshot

When additional benchmark families are ready, add them here first and then link outward.