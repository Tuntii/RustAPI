//! Request guards for route-level authorization
//!
//! This module provides guard extractors for role-based and permission-based access control.
//!
//! # Example
//!
//! ```rust,no_run
//! use rustapi_extras::{RoleGuard, PermissionGuard};
//! use rustapi_core::Json;
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct AdminData {
//!     message: String,
//! }
//!
//! // Extractor-based guards
//! async fn admin_only(guard: RoleGuard) -> Json<AdminData> {
//!     Json(AdminData {
//!         message: format!("Welcome, {}!", guard.role),
//!     })
//! }
//! ```

use rustapi_core::{ApiError, FromRequestParts, Request};

/// Role-based guard extractor
///
/// Extracts the authenticated user and provides the user's role.
/// Requires JWT middleware to be enabled.
#[derive(Debug, Clone)]
pub struct RoleGuard {
    /// The user's role
    pub role: String,
}

impl FromRequestParts for RoleGuard {
    fn from_request_parts(req: &Request) -> rustapi_core::Result<Self> {
        let extensions = req.extensions();

        #[cfg(feature = "jwt")]
        {
            use crate::jwt::{AuthUser, ValidatedClaims};

            // Try to get ValidatedClaims<serde_json::Value> from extensions
            if let Some(validated) = extensions.get::<ValidatedClaims<serde_json::Value>>() {
                // Extract role from claims
                if let Some(role) = validated.0.get("role").and_then(|r| r.as_str()) {
                    return Ok(Self {
                        role: role.to_string(),
                    });
                }
            }

            // Also try AuthUser for backward compatibility
            if let Some(user) = extensions.get::<AuthUser<serde_json::Value>>() {
                if let Some(role) = user.0.get("role").and_then(|r| r.as_str()) {
                    return Ok(Self {
                        role: role.to_string(),
                    });
                }
            }
        }

        #[cfg(not(feature = "jwt"))]
        {
            let _ = extensions;
        }

        Err(ApiError::forbidden(
            "Authentication required: missing or invalid role",
        ))
    }
}

impl RoleGuard {
    /// Check if the user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.role == role
    }

    /// Require a specific role, returning an error if not matched
    pub fn require_role(&self, role: &str) -> Result<(), ApiError> {
        if self.has_role(role) {
            Ok(())
        } else {
            Err(ApiError::forbidden(format!("Required role: {}", role)))
        }
    }
}

/// Permission-based guard extractor
///
/// Extracts the authenticated user and provides the user's permissions.
/// Requires JWT middleware and permissions in the JWT claims.
#[derive(Debug, Clone)]
pub struct PermissionGuard {
    /// The user's permissions
    pub permissions: Vec<String>,
}

impl FromRequestParts for PermissionGuard {
    fn from_request_parts(req: &Request) -> rustapi_core::Result<Self> {
        let extensions = req.extensions();

        #[cfg(feature = "jwt")]
        {
            use crate::jwt::{AuthUser, ValidatedClaims};

            // Try ValidatedClaims first
            if let Some(validated) = extensions.get::<ValidatedClaims<serde_json::Value>>() {
                if let Some(permissions_value) = validated.0.get("permissions") {
                    if let Some(permissions_array) = permissions_value.as_array() {
                        let permissions: Vec<String> = permissions_array
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();

                        if !permissions.is_empty() {
                            return Ok(Self { permissions });
                        }
                    }
                }
            }

            // Also try AuthUser
            if let Some(user) = extensions.get::<AuthUser<serde_json::Value>>() {
                if let Some(permissions_value) = user.0.get("permissions") {
                    if let Some(permissions_array) = permissions_value.as_array() {
                        let permissions: Vec<String> = permissions_array
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();

                        if !permissions.is_empty() {
                            return Ok(Self { permissions });
                        }
                    }
                }
            }
        }

        #[cfg(not(feature = "jwt"))]
        {
            let _ = extensions;
        }

        Err(ApiError::forbidden(
            "Authentication required: missing or invalid permissions",
        ))
    }
}

impl PermissionGuard {
    /// Check if the user has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| p == permission)
    }

    /// Require a specific permission, returning an error if not matched
    pub fn require_permission(&self, permission: &str) -> Result<(), ApiError> {
        if self.has_permission(permission) {
            Ok(())
        } else {
            Err(ApiError::forbidden(format!(
                "Required permission: {}",
                permission
            )))
        }
    }

    /// Check if the user has any of the given permissions
    pub fn has_any_permission(&self, permissions: &[&str]) -> bool {
        self.permissions
            .iter()
            .any(|p| permissions.contains(&p.as_str()))
    }

    /// Check if the user has all of the given permissions
    pub fn has_all_permissions(&self, permissions: &[&str]) -> bool {
        permissions
            .iter()
            .all(|required| self.has_permission(required))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[tokio::test]
    async fn role_guard_without_auth_fails() {
        let req = Request::from_http_request(
            http::Request::builder()
                .method("GET")
                .uri("/")
                .body(())
                .unwrap(),
            Bytes::new(),
        );

        let result = RoleGuard::from_request_parts(&req);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn permission_guard_without_auth_fails() {
        let req = Request::from_http_request(
            http::Request::builder()
                .method("GET")
                .uri("/")
                .body(())
                .unwrap(),
            Bytes::new(),
        );

        let result = PermissionGuard::from_request_parts(&req);
        assert!(result.is_err());
    }

    #[test]
    fn role_guard_has_role_works() {
        let guard = RoleGuard {
            role: "admin".to_string(),
        };

        assert!(guard.has_role("admin"));
        assert!(!guard.has_role("user"));
    }

    #[test]
    fn permission_guard_has_permission_works() {
        let guard = PermissionGuard {
            permissions: vec!["users.read".to_string(), "users.write".to_string()],
        };

        assert!(guard.has_permission("users.read"));
        assert!(guard.has_permission("users.write"));
        assert!(!guard.has_permission("users.delete"));
    }

    #[test]
    fn permission_guard_has_all_permissions_works() {
        let guard = PermissionGuard {
            permissions: vec!["users.read".to_string(), "users.write".to_string()],
        };

        assert!(guard.has_all_permissions(&["users.read", "users.write"]));
        assert!(!guard.has_all_permissions(&["users.read", "users.delete"]));
    }
}
