# Performance Philosophy

RustAPI is built on the principle of **Zero-Cost Abstractions**. We do not compromise runtime performance for developer ergonomics; we strive to offer both.

## Core Tenets

### 1. Static Dispatch Over Dynamic
Where possible, we use generics and `impl Trait` rather than `Box<dyn Trait>`. This allows the compiler to monomorphize code, inlining function calls and enabling optimizations that vtables prevent.

### 2. Async/Await and Tokio
We are fully committed to the `tokio` ecosystem.
- **IO-Bound**: We use async IO for everything (database, cache, network).
- **CPU-Bound**: Heavy computation is offloaded to `spawn_blocking` to avoid blocking the reactor.

### 3. Allocation Minimization
We use `Bytes` and references where possible to avoid cloning `String`s unnecessarily. The deserialization layer is tuned to borrow from the input buffer when feasible.

## Measuring Success
We don't guess; we measure.
- **Micro-benchmarks**: Use `criterion` for hot paths (serialization, routing).
- **Macro-benchmarks**: Use `hey` or `wrk` for end-to-end HTTP throughput.

> [!IMPORTANT]
> A regression in a micro-benchmark is a blocking issue. We treat performance as a feature.

## The "Action" Overhead
You might ask: *"Does creating a struct for every action add overhead?"*
**Answer**: No. The compiler optimizes away the struct wrapper, leaving just the logic execution. It is zero-cost organization.
