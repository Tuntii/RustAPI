# Community & Open Source

RustAPI is an independent open-source project. Contributions, questions, and feedback are welcome.

## Get help

| Channel | Best for |
|---------|----------|
| [GitHub Discussions](https://github.com/Tuntii/RustAPI/discussions) | Questions, ideas, show-and-tell |
| [GitHub Issues](https://github.com/Tuntii/RustAPI/issues) | Bugs and feature requests |
| [Cookbook](cookbook/src/SUMMARY.md) | Guides, recipes, architecture |
| [docs.rs](https://docs.rs/rustapi-rs) | API reference |

Before opening an issue, search existing issues and discussions. For bugs, include Rust version, feature flags, and a minimal reproduction when possible.

## Contribute

We accept contributions of all sizes:

- **Code** — bug fixes, features, tests, benchmarks
- **Documentation** — README, cookbook, examples, typo fixes
- **Examples** — in-repo samples or the [examples repository](https://github.com/Tuntii/rustapi-rs-examples)
- **Community** — issue triage, discussion answers, release testing

Start here:

1. Read [CONTRIBUTING.md](../CONTRIBUTING.md) for setup, testing, and PR workflow
2. Read [CODE_OF_CONDUCT.md](../CODE_OF_CONDUCT.md)
3. Pick an issue labeled [`good first issue`](https://github.com/Tuntii/RustAPI/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22) or [`help wanted`](https://github.com/Tuntii/RustAPI/issues?q=is%3Aissue+is%3Aopen+label%3A%22help+wanted%22) when available
4. Fork → branch → PR (squash merge to `main`)

### Documentation contributions

Documentation lives in several places:

| Location | Contents |
|----------|----------|
| `README.md` | Project overview and quick start |
| `docs/` | Standalone guides (getting started, architecture, production) |
| `docs/cookbook/src/` | mdBook cookbook source |
| `crates/*/README.md` | Per-crate overviews |
| `CHANGELOG.md` / `RELEASES.md` | Release history |

When you change public behavior, update the cookbook recipe or reference page that matches the feature. When you only fix internals, a CHANGELOG entry under **Changed** or **Fixed** is enough.

Key docs to keep in sync on release:

- Version pins in `docs/GETTING_STARTED.md`, cookbook recipes, and `README.md`
- [Production Baseline](PRODUCTION_BASELINE.md) and [Production Checklist](PRODUCTION_CHECKLIST.md) when defaults change
- [RustAPI Cloud recipe](cookbook/src/recipes/rustapi_cloud.md) when CLI cloud commands change

### Public API changes

User-facing API surface is defined by the `rustapi-rs` facade. Changes that affect public types or feature flags may require:

- Updates to `api/public/` snapshots (CI enforces labels on PRs)
- A [CHANGELOG.md](../CHANGELOG.md) entry
- Migration notes in the cookbook when behavior changes

See [CONTRACT.md](../CONTRACT.md) for stability rules.

## Project values

- **Stable facade** — application code imports `rustapi-rs`, not internal crates
- **Evidence over claims** — benchmarks and behavior changes should be test-backed
- **Small, reviewable PRs** — easier to merge and safer for contributors
- **Respectful collaboration** — see the Code of Conduct

## Releases

Releases are tagged `v0.1.<commit-count>` and published to [crates.io](https://crates.io/crates/rustapi-rs). See [CHANGELOG.md](../CHANGELOG.md) and [RELEASES.md](../RELEASES.md) for notes.

**Repository split:** RustAPI Cloud backend development happens in [RustAPI-Cloud](https://github.com/Tuntii/RustAPI-Cloud). This repo ships the framework and CLI only.

## Security

Report vulnerabilities privately per [SECURITY.md](../SECURITY.md). Do not open public issues for undisclosed security problems.

## License

MIT OR Apache-2.0, at your option. See [LICENSE-MIT](../LICENSE-MIT) and [LICENSE-APACHE](../LICENSE-APACHE).