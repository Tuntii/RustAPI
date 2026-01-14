//! Version-based routing
//!
//! Provides routing infrastructure for versioned APIs.

use super::strategy::{VersionExtractor, VersionStrategy};
use super::version::{ApiVersion, VersionRange};
use crate::v31::OpenApi31Spec;
use crate::OpenApiSpec;
use std::collections::HashMap;

/// Configuration for a versioned route
#[derive(Debug, Clone)]
pub struct VersionedRouteConfig {
    /// Version matcher for this route
    pub matcher: VersionRange,
    /// Whether this version is deprecated
    pub deprecated: bool,
    /// Deprecation message
    pub deprecation_message: Option<String>,
    /// Sunset date (RFC 3339)
    pub sunset: Option<String>,
}

impl VersionedRouteConfig {
    /// Create a new route config for a specific version
    pub fn version(version: ApiVersion) -> Self {
        Self {
            matcher: VersionRange::exact(version),
            deprecated: false,
            deprecation_message: None,
            sunset: None,
        }
    }

    /// Create a route config for a version range
    pub fn range(range: VersionRange) -> Self {
        Self {
            matcher: range,
            deprecated: false,
            deprecation_message: None,
            sunset: None,
        }
    }

    /// Mark this version as deprecated
    pub fn deprecated(mut self) -> Self {
        self.deprecated = true;
        self
    }

    /// Add a deprecation message
    pub fn with_deprecation_message(mut self, message: impl Into<String>) -> Self {
        self.deprecated = true;
        self.deprecation_message = Some(message.into());
        self
    }

    /// Set a sunset date
    pub fn with_sunset(mut self, date: impl Into<String>) -> Self {
        self.sunset = Some(date.into());
        self
    }

    /// Check if this config matches a version
    pub fn matches(&self, version: &ApiVersion) -> bool {
        self.matcher.contains(version)
    }
}

/// Router for version-based API routing
///
/// This router manages different versions of your API and can:
/// - Route requests to the appropriate version handler
/// - Generate separate OpenAPI specs for each version
/// - Handle version deprecation and sunset
#[derive(Debug, Clone)]
pub struct VersionRouter {
    /// Version extraction strategy
    extractor: VersionExtractor,
    /// Registered versions with their specs
    versions: HashMap<ApiVersion, VersionInfo>,
    /// Default version to use when none specified
    default_version: ApiVersion,
    /// Fallback behavior
    fallback: VersionFallback,
}

/// Information about a version
#[derive(Debug, Clone)]
struct VersionInfo {
    /// Route configuration
    config: VersionedRouteConfig,
    /// OpenAPI spec for this version (3.1)
    spec_31: Option<OpenApi31Spec>,
    /// OpenAPI spec for this version (3.0)
    spec_30: Option<OpenApiSpec>,
}

/// Fallback behavior when version is not found
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VersionFallback {
    /// Use the default version
    #[default]
    Default,
    /// Use the latest version
    Latest,
    /// Return an error
    Error,
}

impl VersionRouter {
    /// Create a new version router
    pub fn new() -> Self {
        Self {
            extractor: VersionExtractor::new(),
            versions: HashMap::new(),
            default_version: ApiVersion::v1(),
            fallback: VersionFallback::Default,
        }
    }

    /// Set the versioning strategy
    pub fn strategy(mut self, strategy: VersionStrategy) -> Self {
        self.extractor = VersionExtractor::with_strategy(strategy);
        self
    }

    /// Add multiple strategies (tried in order)
    pub fn strategies(mut self, strategies: Vec<VersionStrategy>) -> Self {
        self.extractor = VersionExtractor::with_strategies(strategies);
        self
    }

    /// Set the default version
    pub fn default_version(mut self, version: ApiVersion) -> Self {
        self.default_version = version;
        self.extractor = self.extractor.default_version(version);
        self
    }

    /// Set the fallback behavior
    pub fn fallback(mut self, behavior: VersionFallback) -> Self {
        self.fallback = behavior;
        self
    }

    /// Register a version
    pub fn version(mut self, version: ApiVersion, config: VersionedRouteConfig) -> Self {
        self.versions.insert(
            version,
            VersionInfo {
                config,
                spec_31: None,
                spec_30: None,
            },
        );
        self
    }

    /// Register a version with OpenAPI 3.1 spec
    pub fn version_with_spec_31(
        mut self,
        version: ApiVersion,
        config: VersionedRouteConfig,
        spec: OpenApi31Spec,
    ) -> Self {
        self.versions.insert(
            version,
            VersionInfo {
                config,
                spec_31: Some(spec),
                spec_30: None,
            },
        );
        self
    }

    /// Register a version with OpenAPI 3.0 spec
    pub fn version_with_spec_30(
        mut self,
        version: ApiVersion,
        config: VersionedRouteConfig,
        spec: OpenApiSpec,
    ) -> Self {
        self.versions.insert(
            version,
            VersionInfo {
                config,
                spec_31: None,
                spec_30: Some(spec),
            },
        );
        self
    }

    /// Get all registered versions
    pub fn registered_versions(&self) -> Vec<ApiVersion> {
        let mut versions: Vec<_> = self.versions.keys().copied().collect();
        versions.sort();
        versions
    }

    /// Get the latest registered version
    pub fn latest_version(&self) -> Option<ApiVersion> {
        self.registered_versions().into_iter().max()
    }

    /// Resolve a version from a path
    pub fn resolve_from_path(&self, path: &str) -> ResolvedVersion {
        if let Some(version) = self.extractor.extract_from_path(path) {
            self.resolve_version(version)
        } else {
            self.resolve_fallback()
        }
    }

    /// Resolve a version from headers
    pub fn resolve_from_headers(&self, headers: &HashMap<String, String>) -> ResolvedVersion {
        if let Some(version) = self.extractor.extract_from_headers(headers) {
            self.resolve_version(version)
        } else {
            self.resolve_fallback()
        }
    }

    /// Resolve a version from query string
    pub fn resolve_from_query(&self, query: &str) -> ResolvedVersion {
        if let Some(version) = self.extractor.extract_from_query(query) {
            self.resolve_version(version)
        } else {
            self.resolve_fallback()
        }
    }

    /// Resolve a specific version
    fn resolve_version(&self, version: ApiVersion) -> ResolvedVersion {
        // Check for exact match
        if let Some(info) = self.versions.get(&version) {
            return ResolvedVersion {
                version,
                found: true,
                deprecated: info.config.deprecated,
                deprecation_message: info.config.deprecation_message.clone(),
                sunset: info.config.sunset.clone(),
            };
        }

        // Check for range match
        for (v, info) in &self.versions {
            if info.config.matches(&version) {
                return ResolvedVersion {
                    version: *v,
                    found: true,
                    deprecated: info.config.deprecated,
                    deprecation_message: info.config.deprecation_message.clone(),
                    sunset: info.config.sunset.clone(),
                };
            }
        }

        // Not found, use fallback
        self.resolve_fallback()
    }

    /// Resolve using fallback behavior
    fn resolve_fallback(&self) -> ResolvedVersion {
        match self.fallback {
            VersionFallback::Default => {
                let info = self.versions.get(&self.default_version);
                ResolvedVersion {
                    version: self.default_version,
                    found: info.is_some(),
                    deprecated: info.map(|i| i.config.deprecated).unwrap_or(false),
                    deprecation_message: info.and_then(|i| i.config.deprecation_message.clone()),
                    sunset: info.and_then(|i| i.config.sunset.clone()),
                }
            }
            VersionFallback::Latest => {
                if let Some(version) = self.latest_version() {
                    let info = self.versions.get(&version);
                    ResolvedVersion {
                        version,
                        found: true,
                        deprecated: info.map(|i| i.config.deprecated).unwrap_or(false),
                        deprecation_message: info
                            .and_then(|i| i.config.deprecation_message.clone()),
                        sunset: info.and_then(|i| i.config.sunset.clone()),
                    }
                } else {
                    ResolvedVersion {
                        version: self.default_version,
                        found: false,
                        deprecated: false,
                        deprecation_message: None,
                        sunset: None,
                    }
                }
            }
            VersionFallback::Error => ResolvedVersion {
                version: self.default_version,
                found: false,
                deprecated: false,
                deprecation_message: None,
                sunset: None,
            },
        }
    }

    /// Get OpenAPI 3.1 spec for a version
    pub fn get_spec_31(&self, version: &ApiVersion) -> Option<&OpenApi31Spec> {
        self.versions.get(version).and_then(|v| v.spec_31.as_ref())
    }

    /// Get OpenAPI 3.0 spec for a version
    pub fn get_spec_30(&self, version: &ApiVersion) -> Option<&OpenApiSpec> {
        self.versions.get(version).and_then(|v| v.spec_30.as_ref())
    }

    /// Strip version from path
    pub fn strip_version(&self, path: &str) -> String {
        self.extractor.strip_version_from_path(path)
    }

    /// Check if a version is deprecated
    pub fn is_deprecated(&self, version: &ApiVersion) -> bool {
        self.versions
            .get(version)
            .map(|v| v.config.deprecated)
            .unwrap_or(false)
    }

    /// Get deprecation info for a version
    pub fn get_deprecation_info(&self, version: &ApiVersion) -> Option<DeprecationInfo> {
        self.versions.get(version).and_then(|v| {
            if v.config.deprecated {
                Some(DeprecationInfo {
                    message: v.config.deprecation_message.clone(),
                    sunset: v.config.sunset.clone(),
                })
            } else {
                None
            }
        })
    }
}

impl Default for VersionRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of version resolution
#[derive(Debug, Clone)]
pub struct ResolvedVersion {
    /// The resolved version
    pub version: ApiVersion,
    /// Whether the version was found
    pub found: bool,
    /// Whether the version is deprecated
    pub deprecated: bool,
    /// Deprecation message
    pub deprecation_message: Option<String>,
    /// Sunset date
    pub sunset: Option<String>,
}

impl ResolvedVersion {
    /// Get HTTP headers for this resolved version
    pub fn response_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();

        // Add API-Version header
        headers.insert("API-Version".to_string(), self.version.to_string());

        // Add deprecation headers if deprecated
        if self.deprecated {
            headers.insert("Deprecation".to_string(), "true".to_string());

            if let Some(sunset) = &self.sunset {
                headers.insert("Sunset".to_string(), sunset.clone());
            }

            if let Some(message) = &self.deprecation_message {
                headers.insert("X-Deprecation-Notice".to_string(), message.clone());
            }
        }

        headers
    }
}

/// Deprecation information
#[derive(Debug, Clone)]
pub struct DeprecationInfo {
    /// Deprecation message
    pub message: Option<String>,
    /// Sunset date (RFC 3339)
    pub sunset: Option<String>,
}

/// Builder for creating versioned OpenAPI specs
pub struct VersionedSpecBuilder {
    /// Base title
    title: String,
    /// Base description
    description: Option<String>,
    /// Versions to build
    versions: Vec<(ApiVersion, VersionedRouteConfig)>,
}

impl VersionedSpecBuilder {
    /// Create a new builder
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            versions: Vec::new(),
        }
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add a version
    pub fn version(mut self, version: ApiVersion, config: VersionedRouteConfig) -> Self {
        self.versions.push((version, config));
        self
    }

    /// Build OpenAPI 3.1 specs for all versions
    pub fn build_31(&self) -> HashMap<ApiVersion, OpenApi31Spec> {
        let mut specs = HashMap::new();

        for (version, config) in &self.versions {
            let mut spec = OpenApi31Spec::new(
                format!("{} {}", self.title, version.as_path_segment()),
                version.to_string(),
            );

            if let Some(desc) = &self.description {
                spec = spec.description(desc.clone());
            }

            // Add deprecation info
            if config.deprecated {
                let mut info = "DEPRECATED".to_string();
                if let Some(msg) = &config.deprecation_message {
                    info.push_str(&format!(": {}", msg));
                }
                if let Some(sunset) = &config.sunset {
                    info.push_str(&format!(" (Sunset: {})", sunset));
                }
                spec.info.summary = Some(info);
            }

            specs.insert(*version, spec);
        }

        specs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let router = VersionRouter::new()
            .strategy(VersionStrategy::path())
            .default_version(ApiVersion::v1())
            .version(
                ApiVersion::v1(),
                VersionedRouteConfig::version(ApiVersion::v1()),
            )
            .version(
                ApiVersion::v2(),
                VersionedRouteConfig::version(ApiVersion::v2()).deprecated(),
            );

        assert_eq!(
            router.registered_versions(),
            vec![ApiVersion::v1(), ApiVersion::v2()]
        );
        assert!(!router.is_deprecated(&ApiVersion::v1()));
        assert!(router.is_deprecated(&ApiVersion::v2()));
    }

    #[test]
    fn test_resolve_from_path() {
        let router = VersionRouter::new()
            .version(
                ApiVersion::v1(),
                VersionedRouteConfig::version(ApiVersion::v1()),
            )
            .version(
                ApiVersion::v2(),
                VersionedRouteConfig::version(ApiVersion::v2()),
            );

        let resolved = router.resolve_from_path("/v1/users");
        assert!(resolved.found);
        assert_eq!(resolved.version, ApiVersion::v1());

        let resolved = router.resolve_from_path("/v2/products");
        assert!(resolved.found);
        assert_eq!(resolved.version, ApiVersion::v2());
    }

    #[test]
    fn test_resolve_fallback() {
        let router = VersionRouter::new()
            .default_version(ApiVersion::v1())
            .fallback(VersionFallback::Default)
            .version(
                ApiVersion::v1(),
                VersionedRouteConfig::version(ApiVersion::v1()),
            );

        // v3 not registered, should fall back to default
        let resolved = router.resolve_from_path("/v3/test");
        assert_eq!(resolved.version, ApiVersion::v1());
    }

    #[test]
    fn test_deprecation_info() {
        let router = VersionRouter::new().version(
            ApiVersion::v1(),
            VersionedRouteConfig::version(ApiVersion::v1())
                .with_deprecation_message("Use v2 instead")
                .with_sunset("2024-12-31T23:59:59Z"),
        );

        let info = router.get_deprecation_info(&ApiVersion::v1()).unwrap();
        assert_eq!(info.message, Some("Use v2 instead".to_string()));
        assert_eq!(info.sunset, Some("2024-12-31T23:59:59Z".to_string()));
    }

    #[test]
    fn test_response_headers() {
        let resolved = ResolvedVersion {
            version: ApiVersion::v1(),
            found: true,
            deprecated: true,
            deprecation_message: Some("Legacy version".to_string()),
            sunset: Some("2024-12-31".to_string()),
        };

        let headers = resolved.response_headers();
        assert_eq!(headers.get("API-Version"), Some(&"1.0.0".to_string()));
        assert_eq!(headers.get("Deprecation"), Some(&"true".to_string()));
        assert_eq!(headers.get("Sunset"), Some(&"2024-12-31".to_string()));
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
                VersionedRouteConfig::version(ApiVersion::v2()),
            )
            .build_31();

        assert_eq!(specs.len(), 2);
        assert!(specs.contains_key(&ApiVersion::v1()));
        assert!(specs.contains_key(&ApiVersion::v2()));
    }
}
