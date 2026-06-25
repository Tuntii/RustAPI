# Recipe: Simulating CI

Nothing is worse than pushing code and waiting 10 minutes just to fail on a formatting check.

## The Local CI Runner
We have replicated the GitHub Actions workflow in a local script.

```powershell
.\scripts\simulate_ci.ps1
```

## What it does
1.  **Format Check**: `cargo fmt --check`. Fails if you code is messy.
2.  **Build**: Ensures everything compiles.
3.  **Tests**: Runs the full test suite.
4.  **Linter**: Runs `clippy` with strict settings.

## When to run it
- Before **every** push.
- After merging a large PR.

> [!TIP]
> If it passes locally, it is 99% guaranteed to pass on GitHub Actions.
