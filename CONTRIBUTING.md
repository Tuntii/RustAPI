# Contributing to RustAPI

Thank you for your interest in contributing to RustAPI! We welcome contributions of all kinds - bug reports, feature requests, documentation improvements, and code contributions.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Testing](#testing)
- [Code Style](#code-style)
- [Pull Request Process](#pull-request-process)
- [Project Structure](#project-structure)
- [Release Process](#release-process)
- [Getting Help](#getting-help)

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment for everyone.

## Getting Started

### First Time Contributors

New to open source? Check out these resources:
- [How to Contribute to Open Source](https://opensource.guide/how-to-contribute/)
- [First Contributions](https://github.com/firstcontributions/first-contributions)

### Quick Start

1. **Fork the repository** - Click the "Fork" button on GitHub
2. **Clone your fork**:
   ```bash
   git clone https://github.com/YOUR-USERNAME/RustAPI.git
   cd RustAPI
   ```
3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/Tuntii/RustAPI.git
   ```
4. **Create a new branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```
5. **Make your changes** (see guidelines below)
6. **Test your changes**:
   ```bash
   cargo test --workspace
   cargo clippy --workspace -- -D warnings
   cargo fmt --all -- --check
   ```
7. **Commit and push**:
   ```bash
   git add .
   git commit -m "feat: add awesome feature"
   git push origin feature/your-feature-name
   ```
8. **Create a Pull Request** on GitHub

## Development Setup

### Prerequisites

- **Rust 1.75 or later** - Install from [rustup.rs](https://rustup.rs/)
- **Git** - For version control
- **Code editor** - VS Code with rust-analyzer recommended

### Building

```bash
# Build all crates
cargo build --workspace

# Build with all features
cargo build --workspace --all-features

# Build specific crate
cargo build -p rustapi-core

# Build in release mode
cargo build --workspace --release
```

### Running Examples

```bash
# Run a specific example
cargo run -p hello-world

# List all examples
ls examples/
```

## Making Changes

### Finding Issues to Work On

- Look for issues labeled `good first issue` or `help wanted`
- Check the [project board](https://github.com/Tuntii/RustAPI/projects) for planned features
- Feel free to propose new features in an issue first

### Before You Start

1. **Check existing issues** - Someone might already be working on it
2. **Discuss large changes** - Open an issue to discuss your approach
3. **Keep PRs focused** - One feature/fix per PR

### Types of Contributions

- 🐛 **Bug Fixes** - Fix issues and add regression tests
- ✨ **New Features** - Add new functionality
- 📝 **Documentation** - Improve docs, add examples
- 🎨 **Code Quality** - Refactoring, performance improvements
- ✅ **Tests** - Add test coverage
- 🔧 **Tooling** - Improve build scripts, CI/CD

## Testing

## Testing

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run tests with all features
cargo test --workspace --all-features

# Run tests for a specific crate
cargo test -p rustapi-core

# Run a specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run property tests (may take longer)
cargo test --workspace --release
```

### Writing Tests

- Add unit tests in the same file as the code
- Add integration tests in `tests/` directory
- Use property-based testing with `proptest` for complex logic
- Test error cases and edge cases
- Add doc tests for public APIs

Example:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Arrange
        let input = setup_test_data();
        
        // Act
        let result = your_function(input);
        
        // Assert
        assert_eq!(result, expected);
    }
}
```

## Code Style

### Formatting

All code must be formatted with `rustfmt`:

```bash
# Format all code
cargo fmt --all

# Check formatting without making changes
cargo fmt --all -- --check
```

Configuration is in [rustfmt.toml](rustfmt.toml).

### Linting

All code must pass `clippy` checks:

```bash
# Run clippy on all crates
cargo clippy --workspace --all-features -- -D warnings

# Run clippy with specific lint levels
cargo clippy --workspace -- -W clippy::all -D warnings
```

### Documentation

- **Public APIs must have rustdoc comments**
- Use `///` for item documentation
- Use `//!` for module documentation
- Include code examples in doc comments
- Doc examples must compile and run

Example:
```rust
/// Handles HTTP requests using the registered routes.
///
/// # Example
///
/// ```rust
/// use rustapi_rs::prelude::*;
///
/// #[rustapi_rs::get("/hello")]
/// async fn hello() -> &'static str {
///     "Hello, World!"
/// }
/// ```
pub async fn handle_request() { }
```

### Naming Conventions

- Use `snake_case` for functions, variables, modules
- Use `PascalCase` for types, traits, enums
- Use `SCREAMING_SNAKE_CASE` for constants
- Prefix private items with underscore if unused
- Use descriptive names, avoid abbreviations

### Error Handling

- Use `Result<T, E>` for fallible operations
- Use `thiserror` for custom error types
- Provide helpful error messages
- Document error conditions in rustdoc

## Pull Request Process

### PR Title Format

Follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat: add new feature` - New functionality
- `fix: resolve bug in router` - Bug fixes
- `docs: update API documentation` - Documentation changes
- `refactor: restructure handler logic` - Code refactoring
- `test: add router tests` - Test additions/changes
- `perf: optimize route matching` - Performance improvements
- `chore: update dependencies` - Maintenance tasks
- `ci: update GitHub Actions` - CI/CD changes

### PR Checklist

Before submitting, ensure:

- [ ] Code follows style guidelines (`cargo fmt`, `cargo clippy`)
- [ ] All tests pass (`cargo test --workspace`)
- [ ] New tests added for new functionality
- [ ] Documentation updated (if applicable)
- [ ] Examples added/updated (if applicable)
- [ ] CHANGELOG.md updated (for significant changes)
- [ ] No breaking changes (or clearly documented)
- [ ] PR description explains what and why

### PR Template

When creating a PR, include:

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Related Issues
Fixes #123, Closes #456

## Testing
- Describe how you tested the changes
- Include relevant test commands

## Screenshots (if applicable)
Add screenshots for UI changes

## Checklist
- [ ] Tests pass locally
- [ ] Code is formatted
- [ ] Documentation updated
```

### Review Process

1. **Automated checks** run on your PR (tests, formatting, clippy)
2. **Maintainer review** - May request changes
3. **Address feedback** - Push updates to your branch
4. **Approval** - Once approved, PR will be merged
5. **Merge** - Squash merge to main branch

### After Your PR is Merged

- Your changes will be in the next release
- You'll be credited in CHANGELOG.md
- Thank you for contributing! 🎉

## Commit Guidelines

- Write clear, concise commit messages
- Use present tense ("Add feature" not "Added feature")
- Use imperative mood ("Move cursor to..." not "Moves cursor to...")
- Reference issues when applicable (`Fixes #123`, `Closes #456`)
- Limit first line to 72 characters
- Add detailed description in commit body if needed

**Good commit messages:**
```
feat: add WebSocket support to core router

Implement WebSocket handler registration and upgrade logic.
Includes connection lifecycle management and message handling.

Fixes #123
```

```
fix: resolve path parameter parsing issue

Path parameters with special characters were not properly decoded.
Now using percent-decoding for all path params.

Closes #456
```

## Project Structure

```
RustAPI/
├── crates/
│   ├── rustapi-rs/       # 🎯 Public-facing crate (re-exports)
│   ├── rustapi-core/     # ⚙️  Core HTTP engine and routing
│   ├── rustapi-macros/   # 🔧 Procedural macros (#[get], #[post], etc.)
│   ├── rustapi-validate/ # ✅ Validation integration (validator crate)
│   ├── rustapi-openapi/  # 📚 OpenAPI/Swagger documentation
│   ├── rustapi-extras/   # 🎁 Optional features (JWT, CORS, SQLx helpers)
│   ├── rustapi-toon/     # 🎨 TOON format support
│   ├── rustapi-ws/       # 🔌 WebSocket support
│   ├── rustapi-view/     # 🖼️  Template rendering (Tera)
│   └── cargo-rustapi/    # 📦 CLI tool
├── examples/             # 📖 Example applications
│   ├── hello-world/      # Basic example
│   ├── crud-api/         # CRUD operations
│   ├── auth-api/         # Authentication
│   ├── sqlx-crud/        # Database integration
│   ├── websocket/        # WebSocket example
│   └── ...
├── benches/              # 🏃 Performance benchmarks
├── docs/                 # 📝 Documentation
├── scripts/              # 🛠️  Build and publish scripts
└── memories/             # 🧠 Project memory/context
```

### Crate Dependencies

```
rustapi-rs (public API)
├── rustapi-core (HTTP engine)
│   ├── rustapi-macros (proc macros)
│   └── rustapi-openapi (OpenAPI specs)
├── rustapi-validate (validation)
├── rustapi-extras (optional features)
├── rustapi-toon (TOON format)
├── rustapi-ws (WebSocket)
└── rustapi-view (templates)
```

### Where to Make Changes

- **Adding HTTP features** → `rustapi-core`
- **Adding proc macros** → `rustapi-macros`
- **Adding validation** → `rustapi-validate`
- **Adding OpenAPI features** → `rustapi-openapi`
- **Adding optional features** → `rustapi-extras`
- **Adding examples** → `examples/`
- **Adding tests** → relevant crate's `tests/` directory
- **Adding docs** → `docs/` or inline rustdoc

## Release Process

### Versioning

RustAPI follows [Semantic Versioning](https://semver.org/):

- **MAJOR** (0.x.0) - Breaking changes
- **MINOR** (0.1.x) - New features, backwards compatible
- **PATCH** (0.1.x) - Bug fixes, backwards compatible

### Release Checklist (Maintainers)

1. Update version in `Cargo.toml` (workspace.package.version)
2. Update all crate references to new version
3. Update CHANGELOG.md with release notes
4. Run full test suite: `cargo test --workspace --all-features`
5. Build documentation: `cargo doc --workspace --all-features`
6. Tag release: `git tag v0.1.x`
7. Push tag: `git push origin v0.1.x`
8. Publish crates: `./scripts/publish.ps1` or `./scripts/smart_publish.ps1`
9. Create GitHub release with changelog

## Getting Help

### Resources

- 📖 **Documentation**: [docs/](docs/)
- 💬 **Discussions**: [GitHub Discussions](https://github.com/Tuntii/RustAPI/discussions)
- 🐛 **Issues**: [GitHub Issues](https://github.com/Tuntii/RustAPI/issues)
- 📧 **Contact**: Open an issue for questions

### Reporting Issues

When reporting bugs, please include:

1. **Environment**:
   - Rust version: `rustc --version`
   - RustAPI version
   - Operating system

2. **Description**:
   - What you expected to happen
   - What actually happened
   - Steps to reproduce

3. **Code**:
   - Minimal reproduction code
   - Relevant error messages
   - Stack traces (if applicable)

**Issue Template:**
```markdown
## Description
Brief description of the issue

## Environment
- Rust version: 1.75.0
- RustAPI version: 0.1.7
- OS: Windows 11

## Steps to Reproduce
1. Create a route with...
2. Call the endpoint...
3. See error...

## Expected Behavior
What should happen

## Actual Behavior
What actually happens

## Code
\```rust
// Minimal reproduction code
\```

## Error Messages
\```
// Error output
\```
```

### Feature Requests

We welcome feature requests! Please:

1. Check if the feature already exists or is planned
2. Explain the use case and why it's valuable
3. Consider if it fits the project's scope
4. Be open to discussion about implementation

### Security Issues

**Do not open public issues for security vulnerabilities!**

Please report security issues via:
- GitHub Security Advisories (preferred)
- Email to maintainers

## Recognition

All contributors will be:
- Listed in CHANGELOG.md for their contributions
- Credited in release notes
- Added to GitHub's contributors list

### Top Contributors

Special thanks to all our contributors! You can see them on the [contributors page](https://github.com/Tuntii/RustAPI/graphs/contributors).

---

## Thank You! 🙏

Your contributions help make RustAPI better for everyone. Whether you're fixing a typo, adding a feature, or reporting a bug - every contribution matters!

Happy coding! 🦀✨
