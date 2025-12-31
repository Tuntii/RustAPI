# Contributing to RustAPI

Thank you for your interest in contributing to RustAPI! This document provides guidelines and information for contributors.

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment for everyone.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/Tuntii/RustAPI.git`
3. Create a new branch: `git checkout -b feature/your-feature-name`
4. Make your changes
5. Run tests: `cargo test --workspace`
6. Submit a pull request

## Development Setup

### Prerequisites

- Rust 1.75 or later
- Cargo (comes with Rust)

### Building

```bash
# Build all crates
cargo build --workspace

# Build with all features
cargo build --workspace --all-features
```

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run tests with all features
cargo test --workspace --all-features

# Run a specific crate's tests
cargo test -p rustapi-core
```

## Code Style

### Formatting

All code must be formatted with `rustfmt`:

```bash
cargo fmt --all
```

### Linting

All code must pass `clippy` checks:

```bash
cargo clippy --workspace --all-features -- -D warnings
```

### Documentation

- All public APIs must have rustdoc documentation
- Include code examples in doc comments where appropriate
- Doc examples must compile and run

## Pull Request Process

1. **Create a descriptive PR title** following conventional commits:
   - `feat:` for new features
   - `fix:` for bug fixes
   - `docs:` for documentation changes
   - `refactor:` for code refactoring
   - `test:` for test additions/changes
   - `chore:` for maintenance tasks

2. **Fill out the PR template** with:
   - Description of changes
   - Related issue numbers
   - Testing performed

3. **Ensure all checks pass**:
   - All tests pass
   - Code is formatted (`cargo fmt`)
   - No clippy warnings (`cargo clippy`)
   - Documentation builds

4. **Request review** from maintainers

5. **Address feedback** promptly and push updates

## Commit Guidelines

- Write clear, concise commit messages
- Use present tense ("Add feature" not "Added feature")
- Reference issues when applicable (`Fixes #123`)

## Project Structure

```
RustAPI/
├── crates/
│   ├── rustapi-rs/       # Public-facing crate (re-exports)
│   ├── rustapi-core/     # Core HTTP engine and routing
│   ├── rustapi-macros/   # Procedural macros
│   ├── rustapi-validate/ # Validation integration
│   ├── rustapi-openapi/  # OpenAPI/Swagger support
│   └── rustapi-extras/   # Optional features (JWT, CORS, etc.)
├── examples/             # Example applications
├── benches/              # Benchmarks
└── scripts/              # Build and publish scripts
```

## Adding New Features

1. Discuss the feature in an issue first
2. Follow the existing architecture patterns
3. Add tests for new functionality
4. Update documentation
5. Add examples if applicable

## Reporting Issues

When reporting issues, please include:

- Rust version (`rustc --version`)
- RustAPI version
- Minimal reproduction code
- Expected vs actual behavior
- Error messages (if any)

## Questions?

Feel free to open an issue for questions or join discussions in existing issues.

Thank you for contributing to RustAPI!
