//! Validation groups for conditional validation.

use serde::{Deserialize, Serialize};

/// Validation groups for applying different rules based on operation type.
///
/// ## Example
///
/// ```rust,ignore
/// use rustapi_validate::v2::prelude::*;
///
/// struct User {
///     id: Option<i64>,
///     email: String,
/// }
///
/// impl User {
///     fn validate_for_group(&self, group: ValidationGroup) -> Result<(), ValidationErrors> {
///         let mut errors = ValidationErrors::new();
///         
///         // Email is always required
///         if let Err(e) = EmailRule::default().validate(&self.email) {
///             errors.add("email", e);
///         }
///         
///         // ID is required only for updates
///         if group == ValidationGroup::Update && self.id.is_none() {
///             errors.add("id", RuleError::new("required", "ID is required for updates"));
///         }
///         
///         errors.into_result()
///     }
/// }
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationGroup {
    /// Validation rules for creating new records
    Create,
    /// Validation rules for updating existing records
    Update,
    /// Custom validation group with a name
    Custom(String),
    /// Default group - applies to all operations
    #[default]
    Default,
}

impl ValidationGroup {
    /// Create a custom validation group.
    pub fn custom(name: impl Into<String>) -> Self {
        Self::Custom(name.into())
    }

    /// Check if this group matches another group.
    ///
    /// Default group matches everything.
    pub fn matches(&self, other: &ValidationGroup) -> bool {
        match (self, other) {
            (ValidationGroup::Default, _) => true,
            // Default context check removed as it should only validate Default rules
            (a, b) => a == b,
        }
    }

    /// Get the group name as a string.
    pub fn name(&self) -> &str {
        match self {
            ValidationGroup::Create => "create",
            ValidationGroup::Update => "update",
            ValidationGroup::Custom(name) => name,
            ValidationGroup::Default => "default",
        }
    }
}

impl From<&str> for ValidationGroup {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "create" => ValidationGroup::Create,
            "update" => ValidationGroup::Update,
            "default" => ValidationGroup::Default,
            other => ValidationGroup::Custom(other.to_string()),
        }
    }
}

impl std::fmt::Display for ValidationGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// A validation rule with an associated group.
#[derive(Debug, Clone)]
pub struct GroupedRule<R> {
    /// The validation rule
    pub rule: R,
    /// The group this rule belongs to
    pub group: ValidationGroup,
}

impl<R> GroupedRule<R> {
    /// Create a new grouped rule.
    pub fn new(rule: R, group: ValidationGroup) -> Self {
        Self { rule, group }
    }

    /// Create a rule for the Create group.
    pub fn for_create(rule: R) -> Self {
        Self::new(rule, ValidationGroup::Create)
    }

    /// Create a rule for the Update group.
    pub fn for_update(rule: R) -> Self {
        Self::new(rule, ValidationGroup::Update)
    }

    /// Create a rule for the Default group (applies to all).
    pub fn for_default(rule: R) -> Self {
        Self::new(rule, ValidationGroup::Default)
    }

    /// Check if this rule should be applied for the given group.
    pub fn applies_to(&self, group: &ValidationGroup) -> bool {
        self.group.matches(group)
    }
}

/// A collection of grouped validation rules for a field.
#[derive(Debug, Clone, Default)]
pub struct GroupedRules<R> {
    rules: Vec<GroupedRule<R>>,
}

impl<R> GroupedRules<R> {
    /// Create a new empty collection.
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Add a rule with a group.
    pub fn add(mut self, rule: R, group: ValidationGroup) -> Self {
        self.rules.push(GroupedRule::new(rule, group));
        self
    }

    /// Add a rule for the Create group.
    pub fn on_create(self, rule: R) -> Self {
        self.add(rule, ValidationGroup::Create)
    }

    /// Add a rule for the Update group.
    pub fn on_update(self, rule: R) -> Self {
        self.add(rule, ValidationGroup::Update)
    }

    /// Add a rule that applies to all groups.
    pub fn always(self, rule: R) -> Self {
        self.add(rule, ValidationGroup::Default)
    }

    /// Get rules that apply to a specific group.
    pub fn for_group<'a>(&'a self, group: &'a ValidationGroup) -> impl Iterator<Item = &'a R> + 'a {
        self.rules
            .iter()
            .filter(move |gr| gr.applies_to(group))
            .map(|gr| &gr.rule)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_from_str() {
        assert_eq!(ValidationGroup::from("create"), ValidationGroup::Create);
        assert_eq!(ValidationGroup::from("update"), ValidationGroup::Update);
        assert_eq!(ValidationGroup::from("default"), ValidationGroup::Default);
        assert_eq!(
            ValidationGroup::from("custom_group"),
            ValidationGroup::Custom("custom_group".to_string())
        );
    }

    #[test]
    fn group_matches() {
        assert!(ValidationGroup::Default.matches(&ValidationGroup::Create));
        assert!(!ValidationGroup::Create.matches(&ValidationGroup::Default));
        assert!(ValidationGroup::Create.matches(&ValidationGroup::Create));
        assert!(!ValidationGroup::Create.matches(&ValidationGroup::Update));
    }

    #[test]
    fn group_name() {
        assert_eq!(ValidationGroup::Create.name(), "create");
        assert_eq!(ValidationGroup::Update.name(), "update");
        assert_eq!(ValidationGroup::Default.name(), "default");
        assert_eq!(ValidationGroup::Custom("test".to_string()).name(), "test");
    }

    #[test]
    fn group_serialization() {
        let group = ValidationGroup::Create;
        let json = serde_json::to_string(&group).unwrap();
        assert_eq!(json, "\"create\"");

        let parsed: ValidationGroup = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ValidationGroup::Create);
    }

    #[test]
    fn grouped_rule_applies_to() {
        let create_rule = GroupedRule::for_create("rule1");
        let update_rule = GroupedRule::for_update("rule2");
        let default_rule = GroupedRule::for_default("rule3");

        assert!(create_rule.applies_to(&ValidationGroup::Create));
        assert!(!create_rule.applies_to(&ValidationGroup::Update));
        assert!(!create_rule.applies_to(&ValidationGroup::Default));

        assert!(!update_rule.applies_to(&ValidationGroup::Create));
        assert!(update_rule.applies_to(&ValidationGroup::Update));
        assert!(!update_rule.applies_to(&ValidationGroup::Default));

        assert!(default_rule.applies_to(&ValidationGroup::Create));
        assert!(default_rule.applies_to(&ValidationGroup::Update));
        assert!(default_rule.applies_to(&ValidationGroup::Default));
    }

    #[test]
    fn grouped_rules_for_group() {
        let rules = GroupedRules::new()
            .on_create("create_only")
            .on_update("update_only")
            .always("always");

        let create_rules: Vec<_> = rules.for_group(&ValidationGroup::Create).collect();
        assert_eq!(create_rules.len(), 2);
        assert!(create_rules.contains(&&"create_only"));
        assert!(create_rules.contains(&&"always"));

        let update_rules: Vec<_> = rules.for_group(&ValidationGroup::Update).collect();
        assert_eq!(update_rules.len(), 2);
        assert!(update_rules.contains(&&"update_only"));
        assert!(update_rules.contains(&&"always"));
    }
}
