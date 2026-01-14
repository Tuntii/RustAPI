# The Testing Pyramid

A healthy project requires a balanced testing strategy. RustAPI relies on three layers of verification.

## 1. Unit Tests (The Base)
**Where**: Helper modules, core logic snippets, utility functions.
**Goal**: Verify logic in isolation.
**Command**: `cargo test`

```rust
#[test]
fn test_email_validation() {
    assert!(validate_email("test@example.com").is_ok());
}
```

## 2. Integration / Flow Tests (The Middle)
**Where**: `tests/integration/` and specific "Action" tests.
**Goal**: Verify that components work together. Does the Action talk to the database correctly?
**Mocking**: We use mocking for external services (Stripe, AWS) but favor real database containers (Testcontainers) for data integrity.

## 3. End-to-End & Benchmarks (The Top)
**Where**: `benches/` and CI simulation scripts.
**Goal**: Performance verification and full system sanity.
**Tool**: `check_quality.ps1` runs the full suite.

## Continuous Integration Strategy
We do not push code that fails locally.
- Run `scripts/simulate_ci.ps1` before every push.
- This script runs `fmt`, `clippy`, and `test` in a production-like environment.

> [!WARNING]
> Dead code is technical debt. Use the quality check script to find and eliminate unused logic regularly.
