# Run Report: 2026-02-16

## Detected Version
- **Ref:** v0.1.335
- **Status:** No version change detected. Running "Continuous Improvement" phase.

## Changes
### Cookbook Improvements
- **New Recipe:** `recipes/advanced_middleware.md`
  - Covers Rate Limiting (`RateLimitLayer`), Request Deduplication (`DedupLayer`), and Response Caching (`CacheLayer`).
- **New Recipe:** `recipes/audit_logging.md`
  - Covers Audit Events (`AuditEvent`), Actions (`AuditAction`), Compliance (`ComplianceInfo`), and GDPR/SOC2 features.
- **New Recipe:** `recipes/oauth2_client.md`
  - Covers OAuth2 Client configuration (`OAuth2Config`) and authorization flow (`OAuth2Client`).

### Learning Path Improvements
- **Curriculum Update:** `learning/curriculum.md`
  - Added **Module 8: Advanced Middleware** (Phase 3).
  - Added **Module 7: Authentication** expanded to include OAuth2.
  - Added **Module 12: Observability & Auditing** (Phase 4).
  - Renumbered subsequent modules to maintain sequence.

### Documentation Tracking
- Updated `docs_inventory.md` with new recipe files.
- Updated `docs_coverage.md` to reflect coverage of `rate-limit`, `dedup`, `cache`, `audit`, and `oauth2-client` features in `rustapi-extras`.

## TODOs / Next Steps
- Validate links in new recipes.
- Consider adding a dedicated recipe for "Guard" (RBAC) middleware once the feature stabilizes.
- Verify `rustapi-extras` feature flags in examples against `Cargo.toml`.
