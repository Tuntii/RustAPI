//! Property tests for API version routing
//!
//! These tests verify that the API versioning system correctly:
//! - Parses versions from various sources
//! - Routes requests to appropriate version handlers
//! - Handles deprecation properly

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::versioning::*;

    /// Property 19: API Version Routing
    ///
    /// Validates: Requirements 13.2, 13.3
    /// - Version extraction from path/header/query works correctly
    /// - Version-based routing dispatches to correct handlers
    /// - Fallback behavior works as expected

    #[test]
    fn test_version_parsing_various_formats() {
        // Test various version string formats
        let formats = [
            ("1", ApiVersion::new(1, 0, 0)),
            ("v1", ApiVersion::new(1, 0, 0)),
            ("V1", ApiVersion::new(1, 0, 0)),
            ("1.2", ApiVersion::new(1, 2, 0)),
            ("v1.2", ApiVersion::new(1, 2, 0)),
            ("1.2.3", ApiVersion::new(1, 2, 3)),
            ("v1.2.3", ApiVersion::new(1, 2, 3)),
        ];

        for (input, expected) in formats {
            let parsed: ApiVersion = input.parse().unwrap();
            assert_eq!(parsed, expected, "Failed to parse: {}", input);
        }
    }

    #[test]
    fn test_version_comparison() {
        let v1_0_0 = ApiVersion::new(1, 0, 0);
        let v1_0_1 = ApiVersion::new(1, 0, 1);
        let v1_1_0 = ApiVersion::new(1, 1, 0);
        let v2_0_0 = ApiVersion::new(2, 0, 0);

        assert!(v1_0_0 < v1_0_1);
        assert!(v1_0_1 < v1_1_0);
        assert!(v1_1_0 < v2_0_0);
        assert!(v1_0_0 == v1_0_0);
    }

    #[test]
    fn test_version_compatibility() {
        let v1_0 = ApiVersion::new(1, 0, 0);
        let v1_5 = ApiVersion::new(1, 5, 0);
        let v2_0 = ApiVersion::new(2, 0, 0);

        // Same major version is compatible
        assert!(v1_0.is_compatible_with(&v1_5));
        assert!(v1_5.is_compatible_with(&v1_0));

        // Different major version is not compatible
        assert!(!v1_0.is_compatible_with(&v2_0));
    }

    #[test]
    fn test_path_version_extraction() {
        let extractor = VersionExtractor::with_strategy(VersionStrategy::path());

        let tests = [
            ("/v1/users", Some(ApiVersion::new(1, 0, 0))),
            ("/v2/products/123", Some(ApiVersion::new(2, 0, 0))),
            ("/v1.2/items", Some(ApiVersion::new(1, 2, 0))),
            ("/users", None), // No version
        ];

        for (path, expected) in tests {
            let extracted = extractor.extract_from_path(path);
            assert_eq!(extracted, expected, "Path: {}", path);
        }
    }

    #[test]
    fn test_header_version_extraction() {
        let extractor = VersionExtractor::with_strategy(VersionStrategy::header());

        let mut headers = std::collections::HashMap::new();
        headers.insert("x-api-version".to_string(), "2.0".to_string());

        let extracted = extractor.extract_from_headers(&headers);
        assert_eq!(extracted, Some(ApiVersion::new(2, 0, 0)));
    }

    #[test]
    fn test_query_version_extraction() {
        let extractor = VersionExtractor::with_strategy(VersionStrategy::query());

        let tests = [
            ("version=1", Some(ApiVersion::new(1, 0, 0))),
            ("foo=bar&version=2.1", Some(ApiVersion::new(2, 1, 0))),
            ("api-version=3", None), // Wrong param name
        ];

        for (query, expected) in tests {
            let extracted = extractor.extract_from_query(query);
            assert_eq!(extracted, expected, "Query: {}", query);
        }
    }

    #[test]
    fn test_accept_header_version_extraction() {
        let extractor = VersionExtractor::with_strategy(VersionStrategy::accept());

        let mut headers = std::collections::HashMap::new();
        headers.insert(
            "accept".to_string(),
            "application/vnd.api.v2+json".to_string(),
        );

        let extracted = extractor.extract_from_headers(&headers);
        assert_eq!(extracted, Some(ApiVersion::new(2, 0, 0)));
    }

    #[test]
    fn test_custom_path_pattern() {
        let extractor =
            VersionExtractor::with_strategy(VersionStrategy::path_with_pattern("/api/{version}/"));

        let tests = [
            ("/api/1/users", Some(ApiVersion::new(1, 0, 0))),
            ("/api/2.0/products", Some(ApiVersion::new(2, 0, 0))),
            ("/v1/users", None), // Doesn't match pattern
        ];

        for (path, expected) in tests {
            let extracted = extractor.extract_from_path(path);
            assert_eq!(extracted, expected, "Path: {}", path);
        }
    }

    #[test]
    fn test_version_router_resolution() {
        let router = VersionRouter::new()
            .version(
                ApiVersion::v1(),
                VersionedRouteConfig::version(ApiVersion::v1()),
            )
            .version(
                ApiVersion::v2(),
                VersionedRouteConfig::version(ApiVersion::v2()),
            );

        let resolved = router.resolve_from_path("/v1/api/users");
        assert!(resolved.found);
        assert_eq!(resolved.version, ApiVersion::v1());

        let resolved = router.resolve_from_path("/v2/api/products");
        assert!(resolved.found);
        assert_eq!(resolved.version, ApiVersion::v2());
    }

    #[test]
    fn test_version_router_fallback_default() {
        let router = VersionRouter::new()
            .default_version(ApiVersion::v1())
            .fallback(VersionFallback::Default)
            .version(
                ApiVersion::v1(),
                VersionedRouteConfig::version(ApiVersion::v1()),
            );

        // Non-existent version should fall back to default
        let resolved = router.resolve_from_path("/v99/test");
        assert_eq!(resolved.version, ApiVersion::v1());
    }

    #[test]
    fn test_version_router_fallback_latest() {
        let router = VersionRouter::new()
            .fallback(VersionFallback::Latest)
            .version(
                ApiVersion::v1(),
                VersionedRouteConfig::version(ApiVersion::v1()),
            )
            .version(
                ApiVersion::v2(),
                VersionedRouteConfig::version(ApiVersion::v2()),
            )
            .version(
                ApiVersion::v3(),
                VersionedRouteConfig::version(ApiVersion::v3()),
            );

        let resolved = router.resolve_from_path("/v99/test");
        assert_eq!(resolved.version, ApiVersion::v3());
    }

    #[test]
    fn test_version_deprecation() {
        let router = VersionRouter::new()
            .version(
                ApiVersion::v1(),
                VersionedRouteConfig::version(ApiVersion::v1())
                    .with_deprecation_message("Use v2 instead")
                    .with_sunset("2024-12-31T23:59:59Z"),
            )
            .version(
                ApiVersion::v2(),
                VersionedRouteConfig::version(ApiVersion::v2()),
            );

        assert!(router.is_deprecated(&ApiVersion::v1()));
        assert!(!router.is_deprecated(&ApiVersion::v2()));

        let info = router.get_deprecation_info(&ApiVersion::v1()).unwrap();
        assert_eq!(info.message, Some("Use v2 instead".to_string()));
        assert_eq!(info.sunset, Some("2024-12-31T23:59:59Z".to_string()));
    }

    #[test]
    fn test_deprecation_response_headers() {
        let resolved = ResolvedVersion {
            version: ApiVersion::v1(),
            found: true,
            deprecated: true,
            deprecation_message: Some("Deprecated".to_string()),
            sunset: Some("2024-12-31".to_string()),
        };

        let headers = resolved.response_headers();
        assert!(headers.contains_key("API-Version"));
        assert!(headers.contains_key("Deprecation"));
        assert!(headers.contains_key("Sunset"));
    }

    #[test]
    fn test_version_range_contains() {
        let range = VersionRange::between(ApiVersion::new(1, 0, 0), ApiVersion::new(2, 0, 0));

        assert!(range.contains(&ApiVersion::new(1, 0, 0)));
        assert!(range.contains(&ApiVersion::new(1, 5, 0)));
        assert!(range.contains(&ApiVersion::new(2, 0, 0)));
        assert!(!range.contains(&ApiVersion::new(0, 9, 0)));
        assert!(!range.contains(&ApiVersion::new(2, 0, 1)));
    }

    #[test]
    fn test_version_range_major() {
        let range = VersionRange::major(1);

        assert!(range.contains(&ApiVersion::new(1, 0, 0)));
        assert!(range.contains(&ApiVersion::new(1, 99, 99)));
        assert!(!range.contains(&ApiVersion::new(0, 9, 0)));
        assert!(!range.contains(&ApiVersion::new(2, 0, 0)));
    }

    #[test]
    fn test_version_range_exclude() {
        let range = VersionRange::major(1).exclude(ApiVersion::new(1, 5, 0));

        assert!(range.contains(&ApiVersion::new(1, 0, 0)));
        assert!(range.contains(&ApiVersion::new(1, 4, 0)));
        assert!(!range.contains(&ApiVersion::new(1, 5, 0)));
        assert!(range.contains(&ApiVersion::new(1, 6, 0)));
    }

    #[test]
    fn test_strip_version_from_path() {
        let extractor = VersionExtractor::with_strategy(VersionStrategy::path());

        let tests = [
            ("/v1/users", "/users"),
            ("/v2/products/123", "/products/123"),
            ("/v1.2/items", "/items"),
        ];

        for (input, expected) in tests {
            let stripped = extractor.strip_version_from_path(input);
            assert_eq!(stripped, expected, "Input: {}", input);
        }
    }

    #[test]
    fn test_version_as_path_segment() {
        assert_eq!(ApiVersion::new(1, 0, 0).as_path_segment(), "v1");
        assert_eq!(ApiVersion::new(1, 2, 0).as_path_segment(), "v1.2");
        assert_eq!(ApiVersion::new(1, 2, 3).as_path_segment(), "v1.2.3");
    }

    #[test]
    fn test_multiple_strategies() {
        let extractor = VersionExtractor::with_strategies(vec![
            VersionStrategy::path(),
            VersionStrategy::header(),
            VersionStrategy::query(),
        ])
        .default_version(ApiVersion::v1());

        // Path extraction works
        assert_eq!(
            extractor.extract_from_path("/v2/test"),
            Some(ApiVersion::new(2, 0, 0))
        );

        // Query extraction works
        assert_eq!(
            extractor.extract_from_query("version=3"),
            Some(ApiVersion::new(3, 0, 0))
        );

        // Header extraction works
        let mut headers = std::collections::HashMap::new();
        headers.insert("x-api-version".to_string(), "4".to_string());
        assert_eq!(
            extractor.extract_from_headers(&headers),
            Some(ApiVersion::new(4, 0, 0))
        );
    }

    #[test]
    fn test_versioned_spec_builder() {
        let specs = VersionedSpecBuilder::new("My API")
            .description("API description")
            .version(
                ApiVersion::v1(),
                VersionedRouteConfig::version(ApiVersion::v1()),
            )
            .version(
                ApiVersion::v2(),
                VersionedRouteConfig::version(ApiVersion::v2()).with_deprecation_message("Legacy"),
            )
            .build_31();

        assert_eq!(specs.len(), 2);
        assert!(specs.contains_key(&ApiVersion::v1()));
        assert!(specs.contains_key(&ApiVersion::v2()));

        // Check v2 has deprecation in summary
        let v2_spec = specs.get(&ApiVersion::v2()).unwrap();
        assert!(v2_spec
            .info
            .summary
            .as_ref()
            .unwrap()
            .contains("DEPRECATED"));
    }

    #[test]
    fn test_registered_versions_sorted() {
        let router = VersionRouter::new()
            .version(
                ApiVersion::v3(),
                VersionedRouteConfig::version(ApiVersion::v3()),
            )
            .version(
                ApiVersion::v1(),
                VersionedRouteConfig::version(ApiVersion::v1()),
            )
            .version(
                ApiVersion::v2(),
                VersionedRouteConfig::version(ApiVersion::v2()),
            );

        let versions = router.registered_versions();
        assert_eq!(
            versions,
            vec![ApiVersion::v1(), ApiVersion::v2(), ApiVersion::v3()]
        );
    }

    #[test]
    fn test_latest_version() {
        let router = VersionRouter::new()
            .version(
                ApiVersion::v1(),
                VersionedRouteConfig::version(ApiVersion::v1()),
            )
            .version(
                ApiVersion::new(2, 1, 0),
                VersionedRouteConfig::version(ApiVersion::new(2, 1, 0)),
            );

        assert_eq!(router.latest_version(), Some(ApiVersion::new(2, 1, 0)));
    }
}
