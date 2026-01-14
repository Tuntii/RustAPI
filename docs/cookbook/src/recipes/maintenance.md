# Recipe: Maintenance & Quality

Keeping the codebase clean is as important as adding features. RustAPI accumulates "dead code" easily when features are iterated on rapidly.

## The Quality Script
We provide a unified script to handle linting, dead code detection, and testing.

```powershell
.\scripts\check_quality.ps1
```

## Handling Dead Code
The script runs `cargo check` with `RUSTFLAGS="-W unused"`.

### Scenario 1: It's actually dead
**Action**: Delete it.
Don't comment it out. Git history remembers it if you need it back.

### Scenario 2: It's for the future
**Action**: Use `#[allow(dead_code)]`.
Be explicit.
```rust
#[allow(dead_code)] // Planned for phase 2
fn future_helper() { ... }
```

### Scenario 3: False Positive
Struct fields that are only used in Debug impls or serialization might flag as unused.
**Action**: Check if you really need that field. If yes, ignore the warning or derive `Allow`.

## Linting (Clippy)
We enforce `cargo clippy -- -D warnings` in CI.
- **Complexity**: If clippy says a function is too complex, refactor it.
- **Optimization**: If clippy suggests a faster iterator method, take it.
