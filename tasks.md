# tasks.md — RustAPI Engineering Hygiene Roadmap

Scope: Non-marketing. Focus: correctness, reproducibility, security posture, release discipline, and CI guarantees.

Conventions
- [ ] = pending, [x] = done
- Each task must end with a verifiable artifact (file, CI check, rule, or command output)
- Merge policy: no direct pushes to main once Phase 1 is complete

---

## Phase 0 — Baseline Inventory (1 session)

Goal: Understand current repo state and establish a stable baseline.

- [ ] 0.1 Create a "baseline" issue/PR for this tasks.md
  - DoD: PR exists that adds tasks.md and links to the next phases.

- [x] 0.2 Record current CI status and required checks list (as-is)
  - **Workflows**:
    - `CI`: Test (default + all-features), Lint (fmt + clippy), Build (debug + release), Docs.
    - `Security Audit`: `cargo audit` daily/on-change.
    - `Publish`, `Benchmark`, `Coverage`, `Deploy Cookbook`.
  - **Required Checks**:
    - `cargo fmt --all -- --check`
    - `cargo clippy --workspace --all-features -- -D warnings`
    - `cargo test --workspace --all-features`
    - `cargo doc --workspace --all-features --no-deps`
  - DoD: A short section added to tasks.md or an issue comment containing:
    - Current workflows
    - Which ones are green
    - Any flaky jobs observed

- [x] 0.3 Confirm crate/workspace structure and public API surface assumptions
  - DoD: A doc note (in docs/architecture.md or README) stating:
    - Workspace crates list
    - Which crates are "public" vs "internal"
    - Semver policy assumption for 0.x

---

## Phase 1 — GitHub Guardrails (High ROI)

Goal: Prevent regressions by enforcing quality gates.

- [ ] 1.1 Enable branch protection rule for `main`
  - Require PR before merge
  - Require status checks to pass
  - Require branches to be up-to-date before merging
  - Disable force-push and branch deletion
  - DoD: `main` protected; screenshots or settings summary captured in PR description.

- [ ] 1.2 Enforce linear history merges
  - Enable squash merge (and/or rebase merge), disable merge commits
  - DoD: Repo settings updated; validated by attempting a merge commit and seeing it blocked.

- [ ] 1.3 Add CODEOWNERS (even if single maintainer)
  - Suggested: core crates, macros, workflows, docs
  - DoD: `.github/CODEOWNERS` exists and matches repo structure.

- [ ] 1.4 Harden Actions permissions
  - Restrict to GitHub verified actions where possible
  - Reduce workflow token permissions (principle of least privilege)
  - DoD: workflows declare `permissions:` explicitly; repo settings reviewed.

---

## Phase 2 — CI Baseline: Reproducible & Comprehensive

Goal: CI becomes the contract: fmt + clippy + tests + docs.

- [ ] 2.1 Standardize CI commands for workspace
  - fmt: `cargo fmt --all -- --check`
  - clippy: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - tests: `cargo test --workspace --all-targets --all-features`
  - DoD: CI workflow uses these exact commands (or justified deviations documented).

- [ ] 2.2 Add feature-matrix tests
  - `--no-default-features`
  - `--all-features`
  - Any named “meta” feature sets (e.g., `full`)
  - DoD: CI has separate jobs or a matrix; all green.

- [ ] 2.3 Add docs build check
  - `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`
  - DoD: CI job exists and passes.

- [ ] 2.4 Add MSRV policy + CI enforcement
  - Set `rust-version = "X.Y"` in relevant Cargo.toml(s)
  - Add CI job using MSRV toolchain for `cargo check/test` (at least check)
  - DoD: MSRV stated in README and enforced in CI.

- [ ] 2.5 Optional: OS matrix (pragmatic)
  - Minimum: ubuntu; optional: windows
  - DoD: matrix added OR decision documented why not needed.

---

## Phase 3 — Security & Supply Chain (Fail-Safe)

Goal: Security checks are actionable and meaningful.

- [ ] 3.1 Add/confirm `deny.toml` policy
  - License allowlist
  - Banned crates (if any)
  - Advisory handling
  - DoD: `deny.toml` exists; `cargo deny check` succeeds locally and in CI.

- [ ] 3.2 Change security workflow behavior from "informational" to "enforceable"
  - PRs: can be informational (optional)
  - main/release tags: must fail on findings (no `continue-on-error`)
  - DoD: `continue-on-error` removed for enforcement path; behavior documented.

- [ ] 3.3 Add Rust CodeQL scanning (optional but recommended)
  - DoD: Code scanning configured and running on PRs.

- [ ] 3.4 Secret hygiene
  - Remove unnecessary tokens for public-only workflows
  - Scope secrets to required jobs
  - DoD: Secrets list audited; no unused secrets remain.

---

## Phase 4 — Coverage & Benchmarks: Reproducible Evidence

Goal: Numbers become reproducible artifacts, not marketing claims.

- [ ] 4.1 Pin tarpaulin container/tag and make coverage deterministic
  - Avoid floating `develop-nightly`
  - DoD: coverage workflow uses a pinned version and produces a coverage artifact.

- [ ] 4.2 Store coverage output as workflow artifact
  - DoD: CI uploads `cobertura.xml` (or chosen output) and it’s downloadable.

- [ ] 4.3 Benchmarks as artifacts
  - Benchmark workflow uploads benchmark results (`cargo bench` output or JSON)
  - DoD: workflow produces an artifact and README links to "how to reproduce".

- [ ] 4.4 Add a `./scripts/bench.sh` and `./scripts/coverage.sh` (optional)
  - DoD: scripts exist, documented in README, and match CI commands.

---

## Phase 5 — Release Discipline & crates.io Publishing

Goal: Releases are consistent, automated, and auditable.

- [ ] 5.1 Define release trigger policy
  - Tag format: `vX.Y.Z`
  - DoD: documented in CONTRIBUTING.md or RELEASE.md.

- [ ] 5.2 Automate crate publishing safely
  - Publish on tags only
  - Use `cargo publish --locked`
  - Handle multi-crate publish ordering
  - DoD: publish workflow triggers on tag and performs a dry-run step (or real publish when ready).

- [ ] 5.3 Changelog enforcement
  - Require CHANGELOG entry for user-facing changes
  - DoD: PR checklist includes changelog requirement; release script checks it (optional).

- [ ] 5.4 Add `RELEASE.md` (lightweight)
  - DoD: A single doc describing exact steps to cut a release and rollback.

---

## Phase 6 — API Surface & Semver Hygiene (Framework-Level)

Goal: Public API stability and breakage control.

- [ ] 6.1 Identify and label public crates/modules
  - Define which crates are intended for direct use
  - DoD: documented list exists and maintained.

- [ ] 6.2 Add API review rules
  - Prefer `pub(crate)` by default
  - Document unsafe policy + rationale
  - DoD: CONTRIBUTING.md updated with explicit rules.

- [ ] 6.3 Optional: public API diff checks
  - Use `cargo public-api` or rustdoc JSON diff
  - DoD: CI job flags unintended public API changes.

---

## Phase 7 — Documentation Quality Gates (Non-Marketing)

Goal: Docs are correct, compile, and reflect reality.

- [ ] 7.1 Ensure all README code samples compile
  - Add doctest / compile tests where possible
  - DoD: CI validates samples or a dedicated "examples" job exists.

- [ ] 7.2 Architecture doc baseline
  - Minimal: crate graph, request lifecycle, extension points
  - DoD: `docs/architecture.md` exists and matches current code.

- [ ] 7.3 Cookbook/docs build pipeline (if using GitHub Pages)
  - DoD: docs build is reproducible and its workflow is green.

---

## Phase 8 — Maintenance Automation (Keep It Clean)

Goal: Reduce manual work; catch drift early.

- [ ] 8.1 Add Dependabot config for Cargo + GitHub Actions
  - DoD: `.github/dependabot.yml` exists; PRs auto-created.

- [ ] 8.2 Add `cargo fmt`/`clippy` pre-commit guidance (optional)
  - DoD: CONTRIBUTING.md suggests exact commands.

- [ ] 8.3 Add stale policy for issues/PRs (optional; only if needed)
  - DoD: stale bot configured OR explicitly not used (documented).

---

## Appendix — Local Dev Quick Commands

- fmt: `cargo fmt --all`
- clippy: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- tests: `cargo test --workspace --all-targets --all-features`
- docs: `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`
- deny: `cargo deny check`
- audit: `cargo audit`
