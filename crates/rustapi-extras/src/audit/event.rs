//! Audit event types and structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};

/// Unique identifier for an audit event.
pub type AuditEventId = String;

/// Actions that can be audited.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    /// Resource creation
    Create,
    /// Resource read/view
    Read,
    /// Resource update
    Update,
    /// Resource deletion
    Delete,
    /// User login
    Login,
    /// User logout
    Logout,
    /// Failed login attempt
    LoginFailed,
    /// Permission granted
    PermissionGranted,
    /// Permission revoked
    PermissionRevoked,
    /// Data export (GDPR relevance)
    DataExport,
    /// Data deletion request (GDPR relevance)
    DataDeletionRequest,
    /// Configuration change
    ConfigChange,
    /// API key creation
    ApiKeyCreated,
    /// API key revocation
    ApiKeyRevoked,
    /// Password change
    PasswordChange,
    /// MFA enabled/disabled
    MfaChange,
    /// Custom action
    Custom(String),
}

impl AuditAction {
    /// Check if this action is GDPR-relevant.
    pub fn is_gdpr_relevant(&self) -> bool {
        matches!(
            self,
            AuditAction::Create
                | AuditAction::Update
                | AuditAction::Delete
                | AuditAction::DataExport
                | AuditAction::DataDeletionRequest
                | AuditAction::Login
                | AuditAction::PermissionGranted
                | AuditAction::PermissionRevoked
        )
    }

    /// Check if this action is security-relevant (SOC2).
    pub fn is_security_relevant(&self) -> bool {
        matches!(
            self,
            AuditAction::Login
                | AuditAction::LoginFailed
                | AuditAction::Logout
                | AuditAction::PermissionGranted
                | AuditAction::PermissionRevoked
                | AuditAction::ApiKeyCreated
                | AuditAction::ApiKeyRevoked
                | AuditAction::PasswordChange
                | AuditAction::MfaChange
                | AuditAction::ConfigChange
        )
    }
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditAction::Create => write!(f, "create"),
            AuditAction::Read => write!(f, "read"),
            AuditAction::Update => write!(f, "update"),
            AuditAction::Delete => write!(f, "delete"),
            AuditAction::Login => write!(f, "login"),
            AuditAction::Logout => write!(f, "logout"),
            AuditAction::LoginFailed => write!(f, "login_failed"),
            AuditAction::PermissionGranted => write!(f, "permission_granted"),
            AuditAction::PermissionRevoked => write!(f, "permission_revoked"),
            AuditAction::DataExport => write!(f, "data_export"),
            AuditAction::DataDeletionRequest => write!(f, "data_deletion_request"),
            AuditAction::ConfigChange => write!(f, "config_change"),
            AuditAction::ApiKeyCreated => write!(f, "api_key_created"),
            AuditAction::ApiKeyRevoked => write!(f, "api_key_revoked"),
            AuditAction::PasswordChange => write!(f, "password_change"),
            AuditAction::MfaChange => write!(f, "mfa_change"),
            AuditAction::Custom(s) => write!(f, "custom:{}", s),
        }
    }
}

/// Severity level for audit events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditSeverity {
    /// Informational - normal operations
    Info,
    /// Warning - unusual but not critical
    Warning,
    /// Critical - security or compliance concern
    Critical,
}

impl Default for AuditSeverity {
    fn default() -> Self {
        Self::Info
    }
}

/// Compliance-related information for GDPR/SOC2.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComplianceInfo {
    /// Whether this event involves personal data (GDPR).
    #[serde(default)]
    pub involves_personal_data: bool,
    /// Data subject identifier (for GDPR data subject access requests).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_subject_id: Option<String>,
    /// Legal basis for processing (GDPR Article 6).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legal_basis: Option<String>,
    /// Data retention category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_category: Option<String>,
    /// Whether this requires special category handling (GDPR Article 9).
    #[serde(default)]
    pub special_category_data: bool,
    /// Cross-border transfer indicator.
    #[serde(default)]
    pub cross_border_transfer: bool,
    /// SOC2 control reference (e.g., "CC6.1").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub soc2_control: Option<String>,
}

impl ComplianceInfo {
    /// Create new compliance info.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark as involving personal data.
    pub fn personal_data(mut self, involves: bool) -> Self {
        self.involves_personal_data = involves;
        self
    }

    /// Set the data subject ID (for GDPR).
    pub fn data_subject(mut self, id: impl Into<String>) -> Self {
        self.data_subject_id = Some(id.into());
        self
    }

    /// Set the legal basis for processing.
    pub fn legal_basis(mut self, basis: impl Into<String>) -> Self {
        self.legal_basis = Some(basis.into());
        self
    }

    /// Set the retention category.
    pub fn retention(mut self, category: impl Into<String>) -> Self {
        self.retention_category = Some(category.into());
        self
    }

    /// Mark as special category data (GDPR Article 9).
    pub fn special_category(mut self, is_special: bool) -> Self {
        self.special_category_data = is_special;
        self
    }

    /// Mark as involving cross-border transfer.
    pub fn cross_border(mut self, transfer: bool) -> Self {
        self.cross_border_transfer = transfer;
        self
    }

    /// Set SOC2 control reference.
    pub fn soc2_control(mut self, control: impl Into<String>) -> Self {
        self.soc2_control = Some(control.into());
        self
    }
}

/// An audit event record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event identifier (UUID).
    pub id: AuditEventId,
    /// Unix timestamp in milliseconds.
    pub timestamp: u64,
    /// The action that was performed.
    pub action: AuditAction,
    /// Whether the action succeeded.
    pub success: bool,
    /// Severity level.
    pub severity: AuditSeverity,
    /// Actor (user, service, or system) that performed the action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,
    /// Actor type (user, service, system).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_type: Option<String>,
    /// IP address of the actor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    /// User agent string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    /// Resource type (e.g., "users", "orders").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_type: Option<String>,
    /// Resource identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<String>,
    /// Request ID for correlation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Session ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Additional context/metadata.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
    /// Compliance information.
    #[serde(default)]
    pub compliance: ComplianceInfo,
    /// Error message if action failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Changes made (before/after for updates).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changes: Option<AuditChanges>,
}

/// Record of changes made during an update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditChanges {
    /// Fields that were changed with their old values.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub before: HashMap<String, serde_json::Value>,
    /// Fields that were changed with their new values.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub after: HashMap<String, serde_json::Value>,
}

impl AuditChanges {
    /// Create a new changes record.
    pub fn new() -> Self {
        Self {
            before: HashMap::new(),
            after: HashMap::new(),
        }
    }

    /// Record a field change.
    pub fn field(
        mut self,
        name: impl Into<String>,
        before: impl Into<serde_json::Value>,
        after: impl Into<serde_json::Value>,
    ) -> Self {
        let name = name.into();
        self.before.insert(name.clone(), before.into());
        self.after.insert(name, after.into());
        self
    }
}

impl Default for AuditChanges {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditEvent {
    /// Create a new audit event with the given action.
    pub fn new(action: AuditAction) -> Self {
        let id = generate_event_id();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Self {
            id,
            timestamp,
            action,
            success: true,
            severity: AuditSeverity::Info,
            actor_id: None,
            actor_type: None,
            ip_address: None,
            user_agent: None,
            resource_type: None,
            resource_id: None,
            request_id: None,
            session_id: None,
            metadata: HashMap::new(),
            compliance: ComplianceInfo::default(),
            error_message: None,
            changes: None,
        }
    }

    /// Set whether the action succeeded.
    pub fn success(mut self, success: bool) -> Self {
        self.success = success;
        if !success {
            self.severity = AuditSeverity::Warning;
        }
        self
    }

    /// Set the severity level.
    pub fn severity(mut self, severity: AuditSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Set the actor (user/service performing the action).
    pub fn actor(mut self, actor_id: impl Into<String>) -> Self {
        self.actor_id = Some(actor_id.into());
        self
    }

    /// Set the actor type.
    pub fn actor_type(mut self, actor_type: impl Into<String>) -> Self {
        self.actor_type = Some(actor_type.into());
        self
    }

    /// Set the IP address.
    pub fn ip_address(mut self, ip: IpAddr) -> Self {
        self.ip_address = Some(ip.to_string());
        self
    }

    /// Set the IP address from a string.
    pub fn ip_address_str(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = Some(ip.into());
        self
    }

    /// Set the user agent.
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    /// Set the resource being acted upon.
    pub fn resource(
        mut self,
        resource_type: impl Into<String>,
        resource_id: impl Into<String>,
    ) -> Self {
        self.resource_type = Some(resource_type.into());
        self.resource_id = Some(resource_id.into());
        self
    }

    /// Set just the resource type.
    pub fn resource_type(mut self, resource_type: impl Into<String>) -> Self {
        self.resource_type = Some(resource_type.into());
        self
    }

    /// Set just the resource ID.
    pub fn resource_id(mut self, resource_id: impl Into<String>) -> Self {
        self.resource_id = Some(resource_id.into());
        self
    }

    /// Set the request ID for correlation.
    pub fn request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Set the session ID.
    pub fn session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Add metadata.
    pub fn meta(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set compliance information.
    pub fn compliance(mut self, compliance: ComplianceInfo) -> Self {
        self.compliance = compliance;
        self
    }

    /// Set error message (for failed actions).
    pub fn error(mut self, message: impl Into<String>) -> Self {
        self.error_message = Some(message.into());
        self.success = false;
        if self.severity == AuditSeverity::Info {
            self.severity = AuditSeverity::Warning;
        }
        self
    }

    /// Set changes (for update actions).
    pub fn changes(mut self, changes: AuditChanges) -> Self {
        self.changes = Some(changes);
        self
    }

    /// Convert to JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Convert to pretty JSON string.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Generate a unique event ID.
fn generate_event_id() -> String {
    use rand::{rngs::OsRng, RngCore};

    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);

    // Format as UUID-like string
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent::new(AuditAction::Create)
            .resource("users", "user-123")
            .actor("admin@example.com")
            .success(true);

        assert_eq!(event.action, AuditAction::Create);
        assert_eq!(event.resource_type, Some("users".to_string()));
        assert_eq!(event.resource_id, Some("user-123".to_string()));
        assert_eq!(event.actor_id, Some("admin@example.com".to_string()));
        assert!(event.success);
        assert!(!event.id.is_empty());
        assert!(event.timestamp > 0);
    }

    #[test]
    fn test_audit_event_with_compliance() {
        let compliance = ComplianceInfo::new()
            .personal_data(true)
            .data_subject("user-456")
            .legal_basis("consent")
            .soc2_control("CC6.1");

        let event = AuditEvent::new(AuditAction::Update).compliance(compliance);

        assert!(event.compliance.involves_personal_data);
        assert_eq!(
            event.compliance.data_subject_id,
            Some("user-456".to_string())
        );
        assert_eq!(event.compliance.legal_basis, Some("consent".to_string()));
        assert_eq!(event.compliance.soc2_control, Some("CC6.1".to_string()));
    }

    #[test]
    fn test_audit_event_with_changes() {
        let changes = AuditChanges::new()
            .field("email", "old@example.com", "new@example.com")
            .field("name", "Old Name", "New Name");

        let event = AuditEvent::new(AuditAction::Update).changes(changes);

        let c = event.changes.unwrap();
        assert_eq!(c.before.get("email").unwrap(), "old@example.com");
        assert_eq!(c.after.get("email").unwrap(), "new@example.com");
    }

    #[test]
    fn test_audit_action_relevance() {
        assert!(AuditAction::DataExport.is_gdpr_relevant());
        assert!(AuditAction::Login.is_security_relevant());
        assert!(!AuditAction::Read.is_security_relevant());
    }

    #[test]
    fn test_audit_event_serialization() {
        let event = AuditEvent::new(AuditAction::Login)
            .actor("user@example.com")
            .ip_address("192.168.1.1".parse().unwrap())
            .meta("browser", "Chrome");

        let json = event.to_json().unwrap();
        assert!(json.contains("login"));
        assert!(json.contains("user@example.com"));
        assert!(json.contains("192.168.1.1"));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// **Feature: v1-features-roadmap, Property 17: Audit event completeness**
    /// **Validates: Requirements 11.1, 11.2, 11.3**
    ///
    /// For any audit event:
    /// - All required fields (id, timestamp, action) SHALL be populated
    /// - Serialization SHALL preserve all data including GDPR/SOC2 compliance fields
    /// - Event IDs SHALL be unique and valid UUID format
    /// - Timestamps SHALL be monotonically increasing (or equal) within reasonable tolerance

    /// Strategy for generating audit actions
    fn audit_action_strategy() -> impl Strategy<Value = AuditAction> {
        prop_oneof![
            Just(AuditAction::Create),
            Just(AuditAction::Read),
            Just(AuditAction::Update),
            Just(AuditAction::Delete),
            Just(AuditAction::Login),
            Just(AuditAction::Logout),
            Just(AuditAction::LoginFailed),
            Just(AuditAction::PermissionGranted),
            Just(AuditAction::PermissionRevoked),
            Just(AuditAction::DataExport),
            Just(AuditAction::DataDeletionRequest),
            Just(AuditAction::ConfigChange),
            Just(AuditAction::ApiKeyCreated),
            Just(AuditAction::ApiKeyRevoked),
            Just(AuditAction::PasswordChange),
            Just(AuditAction::MfaChange),
            "[a-z_]{3,20}".prop_map(AuditAction::Custom),
        ]
    }

    /// Strategy for generating actor IDs
    fn actor_id_strategy() -> impl Strategy<Value = String> {
        "[a-z0-9_.-]{3,50}@[a-z]{3,10}\\.[a-z]{2,4}"
    }

    /// Strategy for generating resource types
    fn resource_type_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("users".to_string()),
            Just("orders".to_string()),
            Just("products".to_string()),
            Just("invoices".to_string()),
            Just("sessions".to_string()),
        ]
    }

    /// Strategy for generating resource IDs
    fn resource_id_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9-]{10,36}"
    }

    /// Strategy for generating IP addresses
    fn ip_address_strategy() -> impl Strategy<Value = IpAddr> {
        prop_oneof![
            (0u8..255, 0u8..255, 0u8..255, 0u8..255)
                .prop_map(|(a, b, c, d)| format!("{}.{}.{}.{}", a, b, c, d).parse::<IpAddr>().unwrap()),
        ]
    }

    /// Strategy for generating compliance info
    fn compliance_strategy() -> impl Strategy<Value = ComplianceInfo> {
        (
            proptest::bool::ANY, // involves_personal_data
            proptest::option::of("[a-z0-9-]{10,20}"), // data_subject_id
            proptest::option::of(prop_oneof![
                Just("consent".to_string()),
                Just("contract".to_string()),
                Just("legitimate_interest".to_string()),
            ]),
            proptest::option::of(prop_oneof![
                Just("short_term".to_string()),
                Just("long_term".to_string()),
                Just("permanent".to_string()),
            ]),
            proptest::bool::ANY, // special_category_data
            proptest::bool::ANY, // cross_border_transfer
            proptest::option::of("[A-Z]{2,4}[0-9.]{1,5}"), // soc2_control
        )
            .prop_map(
                |(
                    personal_data,
                    subject_id,
                    legal_basis,
                    retention,
                    special,
                    cross_border,
                    soc2,
                )| {
                    let mut info = ComplianceInfo::new().personal_data(personal_data);
                    if let Some(id) = subject_id {
                        info = info.data_subject(id);
                    }
                    if let Some(basis) = legal_basis {
                        info = info.legal_basis(basis);
                    }
                    if let Some(ret) = retention {
                        info = info.retention(ret);
                    }
                    info = info.special_category(special).cross_border(cross_border);
                    if let Some(ctrl) = soc2 {
                        info = info.soc2_control(ctrl);
                    }
                    info
                },
            )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 17: Event always has required fields populated
        #[test]
        fn prop_event_has_required_fields(action in audit_action_strategy()) {
            let event = AuditEvent::new(action.clone());
            
            // ID must be non-empty and valid UUID format
            prop_assert!(!event.id.is_empty());
            prop_assert!(event.id.contains('-')); // UUID has hyphens
            prop_assert_eq!(event.id.split('-').count(), 5); // UUID format: 8-4-4-4-12
            
            // Timestamp must be reasonable (not zero, not in far future)
            prop_assert!(event.timestamp > 0);
            prop_assert!(event.timestamp < u64::MAX / 2); // Reasonable upper bound
            
            // Action must match
            prop_assert_eq!(event.action, action);
        }

        /// Property 17: Event IDs are unique
        #[test]
        fn prop_event_ids_unique(action in audit_action_strategy()) {
            let event1 = AuditEvent::new(action.clone());
            let event2 = AuditEvent::new(action);
            
            // Each event should have a unique ID
            prop_assert_ne!(event1.id, event2.id);
        }

        /// Property 17: Serialization round-trip preserves all fields
        #[test]
        fn prop_serialization_roundtrip(
            action in audit_action_strategy(),
            actor in actor_id_strategy(),
            resource_type in resource_type_strategy(),
            resource_id in resource_id_strategy(),
            success in proptest::bool::ANY,
        ) {
            let event = AuditEvent::new(action)
                .actor(actor.clone())
                .resource(resource_type.clone(), resource_id.clone())
                .success(success);
            
            // Serialize to JSON
            let json = event.to_json().unwrap();
            
            // Deserialize back
            let deserialized: AuditEvent = serde_json::from_str(&json).unwrap();
            
            // All fields should match
            prop_assert_eq!(deserialized.id, event.id);
            prop_assert_eq!(deserialized.timestamp, event.timestamp);
            prop_assert_eq!(deserialized.action, event.action);
            prop_assert_eq!(deserialized.success, event.success);
            prop_assert_eq!(deserialized.actor_id, event.actor_id);
            prop_assert_eq!(deserialized.resource_type, event.resource_type);
            prop_assert_eq!(deserialized.resource_id, event.resource_id);
        }

        /// Property 17: Compliance info serialization preserves GDPR/SOC2 fields
        #[test]
        fn prop_compliance_serialization(
            action in audit_action_strategy(),
            compliance in compliance_strategy(),
        ) {
            let event = AuditEvent::new(action).compliance(compliance.clone());
            
            // Serialize to JSON
            let json = event.to_json().unwrap();
            
            // Deserialize back
            let deserialized: AuditEvent = serde_json::from_str(&json).unwrap();
            
            // Compliance fields should match
            prop_assert_eq!(
                deserialized.compliance.involves_personal_data,
                compliance.involves_personal_data
            );
            prop_assert_eq!(
                deserialized.compliance.data_subject_id,
                compliance.data_subject_id
            );
            prop_assert_eq!(
                deserialized.compliance.legal_basis,
                compliance.legal_basis
            );
            prop_assert_eq!(
                deserialized.compliance.retention_category,
                compliance.retention_category
            );
            prop_assert_eq!(
                deserialized.compliance.special_category_data,
                compliance.special_category_data
            );
            prop_assert_eq!(
                deserialized.compliance.cross_border_transfer,
                compliance.cross_border_transfer
            );
            prop_assert_eq!(
                deserialized.compliance.soc2_control,
                compliance.soc2_control
            );
        }

        /// Property 17: IP address field formats correctly
        #[test]
        fn prop_ip_address_field(
            action in audit_action_strategy(),
            ip in ip_address_strategy(),
        ) {
            let event = AuditEvent::new(action).ip_address(ip);
            
            prop_assert!(event.ip_address.is_some());
            let ip_str = event.ip_address.as_ref().unwrap();
            
            // Should be parseable back to IpAddr
            prop_assert!(ip_str.parse::<IpAddr>().is_ok());
        }

        /// Property 17: Metadata preserves key-value pairs
        #[test]
        fn prop_metadata_preservation(
            action in audit_action_strategy(),
            key in "[a-z_]{3,20}",
            value in "[a-zA-Z0-9 ]{1,50}",
        ) {
            let event = AuditEvent::new(action).meta(key.clone(), value.clone());
            
            prop_assert!(event.metadata.contains_key(&key));
            prop_assert_eq!(event.metadata.get(&key), Some(&value));
            
            // Serialize and deserialize
            let json = event.to_json().unwrap();
            let deserialized: AuditEvent = serde_json::from_str(&json).unwrap();
            
            prop_assert_eq!(deserialized.metadata.get(&key), Some(&value));
        }

        /// Property 17: Failed actions set appropriate flags
        #[test]
        fn prop_failed_action_flags(
            action in audit_action_strategy(),
            error_msg in "[a-zA-Z0-9 ]{10,100}",
        ) {
            let event = AuditEvent::new(action).error(error_msg.clone());
            
            // Error should set success to false
            prop_assert!(!event.success);
            
            // Error message should be preserved
            prop_assert_eq!(event.error_message, Some(error_msg));
            
            // Severity should be at least Warning
            prop_assert!(event.severity >= AuditSeverity::Warning);
        }

        /// Property 17: Changes record preserves before/after values
        #[test]
        fn prop_changes_preservation(
            action in audit_action_strategy(),
            field_name in "[a-z_]{3,15}",
            old_value in "[a-zA-Z0-9]{5,20}",
            new_value in "[a-zA-Z0-9]{5,20}",
        ) {
            let changes = AuditChanges::new()
                .field(field_name.clone(), old_value.clone(), new_value.clone());
            
            let event = AuditEvent::new(action).changes(changes);
            
            prop_assert!(event.changes.is_some());
            let c = event.changes.as_ref().unwrap();
            
            prop_assert_eq!(c.before.get(&field_name).unwrap(), &serde_json::json!(old_value));
            prop_assert_eq!(c.after.get(&field_name).unwrap(), &serde_json::json!(new_value));
            
            // Serialize and verify
            let json = event.to_json().unwrap();
            let deserialized: AuditEvent = serde_json::from_str(&json).unwrap();
            let dc = deserialized.changes.unwrap();
            
            prop_assert_eq!(dc.before.get(&field_name).unwrap(), &serde_json::json!(old_value));
            prop_assert_eq!(dc.after.get(&field_name).unwrap(), &serde_json::json!(new_value));
        }

        /// Property 17: GDPR-relevant actions identified correctly
        #[test]
        fn prop_gdpr_relevance(action in audit_action_strategy()) {
            let is_gdpr = action.is_gdpr_relevant();
            
            match action {
                AuditAction::Create
                | AuditAction::Update
                | AuditAction::Delete
                | AuditAction::DataExport
                | AuditAction::DataDeletionRequest
                | AuditAction::Login
                | AuditAction::PermissionGranted
                | AuditAction::PermissionRevoked => {
                    prop_assert!(is_gdpr);
                }
                _ => {
                    // Other actions may or may not be GDPR-relevant
                }
            }
        }

        /// Property 17: SOC2-relevant actions identified correctly
        #[test]
        fn prop_soc2_relevance(action in audit_action_strategy()) {
            let is_soc2 = action.is_security_relevant();
            
            match action {
                AuditAction::Login
                | AuditAction::LoginFailed
                | AuditAction::Logout
                | AuditAction::PermissionGranted
                | AuditAction::PermissionRevoked
                | AuditAction::ApiKeyCreated
                | AuditAction::ApiKeyRevoked
                | AuditAction::PasswordChange
                | AuditAction::MfaChange
                | AuditAction::ConfigChange => {
                    prop_assert!(is_soc2);
                }
                _ => {
                    prop_assert!(!is_soc2);
                }
            }
        }

        /// Property 17: Event timestamps are reasonable
        #[test]
        fn prop_timestamps_reasonable(_seed in 0u32..100) {
            use std::time::{Duration, SystemTime, UNIX_EPOCH};
            
            let event1 = AuditEvent::new(AuditAction::Create);
            std::thread::sleep(Duration::from_millis(1));
            let event2 = AuditEvent::new(AuditAction::Update);
            
            // Timestamps should be monotonically increasing (or equal if very fast)
            prop_assert!(event2.timestamp >= event1.timestamp);
            
            // Both timestamps should be close to current time
            let now_millis = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            
            prop_assert!(event1.timestamp <= now_millis);
            prop_assert!(event2.timestamp <= now_millis);
            
            // Timestamps should not be too old (within last hour)
            let one_hour_ago = now_millis - (60 * 60 * 1000);
            prop_assert!(event1.timestamp >= one_hour_ago);
            prop_assert!(event2.timestamp >= one_hour_ago);
        }
    }
}
