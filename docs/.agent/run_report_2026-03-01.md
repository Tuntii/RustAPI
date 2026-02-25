# Documentation Run Report: 2026-03-01

**Status**: Success
**Detected Version**: v0.1.335 (No change)
**Focus**: Cookbook Expansion & Learning Path Improvement

## Changes

### 1. Fixes
- **`docs/cookbook/src/recipes/file_uploads.md`**: Clarified usage of `.body_limit()` vs `DefaultBodyLimit` middleware. Removed confusing double-configuration in the example.
- **`docs/cookbook/src/recipes/testing.md`**: Fixed missing `RequestMatcher` import in `MockServer` example.

### 2. New Recipes
- **`docs/cookbook/src/recipes/validation.md`**: Added "Advanced Validation Patterns" covering custom validators, cross-field validation, and error customization.

### 3. Learning Path Improvements
- **`docs/cookbook/src/learning/curriculum.md`**:
  - **Module 9 (WebSockets)**: Added "The Live Chat Room" mini-project.
  - **Module 14 (High Performance)**: Added explicit instruction to enable `http3` feature.
  - **Module 5 (Validation)**: Linked to the new Validation recipe.

### 4. Index Updates
- Added "Advanced Validation" to `docs/cookbook/src/SUMMARY.md`.

## Next Steps
- Consider a recipe for "Structured Logging with Tracing".
- Review "Module 12: Observability" for potential mini-project additions.
