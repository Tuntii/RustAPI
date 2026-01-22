//! Property-based tests for the v2 validation engine.

#[cfg(test)]
mod property_tests {
    use crate::v2::rules::*;
    use crate::v2::traits::{SerializableRule, ValidationRule};
    use proptest::prelude::*;

    // **Feature: v1-features-roadmap, Property 1: Validation rule round-trip**
    // **Validates: Requirements 1.9, 1.10**
    //
    // For any valid validation rule definition, serializing then deserializing
    // the rule SHALL produce an equivalent rule definition.

    // Strategy for generating optional strings (for custom messages)
    fn optional_message_strategy() -> impl Strategy<Value = Option<String>> {
        prop_oneof![Just(None), "[a-zA-Z0-9 ]{1,50}".prop_map(Some),]
    }

    // Strategy for generating EmailRule
    fn email_rule_strategy() -> impl Strategy<Value = EmailRule> {
        optional_message_strategy().prop_map(|message| EmailRule { message })
    }

    // Strategy for generating LengthRule
    fn length_rule_strategy() -> impl Strategy<Value = LengthRule> {
        (
            prop_oneof![Just(None), (0usize..1000).prop_map(Some)],
            prop_oneof![Just(None), (0usize..1000).prop_map(Some)],
            optional_message_strategy(),
        )
            .prop_map(|(min, max, message)| LengthRule { min, max, message })
    }

    // Strategy for generating RangeRule<i64>
    fn range_rule_i64_strategy() -> impl Strategy<Value = RangeRule<i64>> {
        (
            prop_oneof![Just(None), (-1000i64..1000).prop_map(Some)],
            prop_oneof![Just(None), (-1000i64..1000).prop_map(Some)],
            optional_message_strategy(),
        )
            .prop_map(|(min, max, message)| RangeRule { min, max, message })
    }

    // Strategy for generating RegexRule
    fn regex_rule_strategy() -> impl Strategy<Value = RegexRule> {
        (
            // Use simple, valid regex patterns
            prop_oneof![
                Just(r"^\d+$".to_string()),
                Just(r"^[a-z]+$".to_string()),
                Just(r"^\w+@\w+\.\w+$".to_string()),
                Just(r"^[A-Z]{2,4}$".to_string()),
            ],
            optional_message_strategy(),
        )
            .prop_map(|(pattern, message)| {
                let mut rule = RegexRule::new(pattern);
                rule.message = message;
                rule
            })
    }

    // Strategy for generating UrlRule
    fn url_rule_strategy() -> impl Strategy<Value = UrlRule> {
        optional_message_strategy().prop_map(|message| UrlRule { message })
    }

    // Strategy for generating RequiredRule
    fn required_rule_strategy() -> impl Strategy<Value = RequiredRule> {
        optional_message_strategy().prop_map(|message| RequiredRule { message })
    }

    // Strategy for generating AsyncUniqueRule
    fn async_unique_rule_strategy() -> impl Strategy<Value = AsyncUniqueRule> {
        ("[a-z_]{1,20}", "[a-z_]{1,20}", optional_message_strategy()).prop_map(
            |(table, column, message)| AsyncUniqueRule {
                table,
                column,
                message,
            },
        )
    }

    // Strategy for generating AsyncExistsRule
    fn async_exists_rule_strategy() -> impl Strategy<Value = AsyncExistsRule> {
        ("[a-z_]{1,20}", "[a-z_]{1,20}", optional_message_strategy()).prop_map(
            |(table, column, message)| AsyncExistsRule {
                table,
                column,
                message,
            },
        )
    }

    // Strategy for generating AsyncApiRule
    fn async_api_rule_strategy() -> impl Strategy<Value = AsyncApiRule> {
        (
            "https://[a-z]{1,10}\\.[a-z]{2,4}/[a-z]{1,10}",
            optional_message_strategy(),
        )
            .prop_map(|(endpoint, message)| AsyncApiRule { endpoint, message })
    }

    // Strategy for generating SerializableRule
    fn serializable_rule_strategy() -> impl Strategy<Value = SerializableRule> {
        prop_oneof![
            optional_message_strategy().prop_map(|message| SerializableRule::Email { message }),
            (
                prop_oneof![Just(None), (0usize..1000).prop_map(Some)],
                prop_oneof![Just(None), (0usize..1000).prop_map(Some)],
                optional_message_strategy(),
            )
                .prop_map(|(min, max, message)| SerializableRule::Length {
                    min,
                    max,
                    message
                }),
            // Use integer values cast to f64 to avoid floating point precision issues
            (
                prop_oneof![Just(None), (-1000i64..1000).prop_map(|v| Some(v as f64))],
                prop_oneof![Just(None), (-1000i64..1000).prop_map(|v| Some(v as f64))],
                optional_message_strategy(),
            )
                .prop_map(|(min, max, message)| SerializableRule::Range {
                    min,
                    max,
                    message
                }),
            (
                prop_oneof![Just(r"^\d+$".to_string()), Just(r"^[a-z]+$".to_string()),],
                optional_message_strategy(),
            )
                .prop_map(|(pattern, message)| SerializableRule::Regex { pattern, message }),
            optional_message_strategy().prop_map(|message| SerializableRule::Url { message }),
            optional_message_strategy().prop_map(|message| SerializableRule::Required { message }),
            ("[a-z_]{1,20}", "[a-z_]{1,20}", optional_message_strategy(),).prop_map(
                |(table, column, message)| SerializableRule::AsyncUnique {
                    table,
                    column,
                    message,
                }
            ),
            ("[a-z_]{1,20}", "[a-z_]{1,20}", optional_message_strategy(),).prop_map(
                |(table, column, message)| SerializableRule::AsyncExists {
                    table,
                    column,
                    message,
                }
            ),
            (
                "https://[a-z]{1,10}\\.[a-z]{2,4}/[a-z]{1,10}",
                optional_message_strategy(),
            )
                .prop_map(|(endpoint, message)| SerializableRule::AsyncApi { endpoint, message }),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Property 1: EmailRule round-trip
        #[test]
        fn email_rule_roundtrip(rule in email_rule_strategy()) {
            let json = serde_json::to_string(&rule).unwrap();
            let parsed: EmailRule = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(rule, parsed);
        }

        // Property 1: LengthRule round-trip
        #[test]
        fn length_rule_roundtrip(rule in length_rule_strategy()) {
            let json = serde_json::to_string(&rule).unwrap();
            let parsed: LengthRule = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(rule, parsed);
        }

        // Property 1: RangeRule round-trip
        #[test]
        fn range_rule_roundtrip(rule in range_rule_i64_strategy()) {
            let json = serde_json::to_string(&rule).unwrap();
            let parsed: RangeRule<i64> = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(rule, parsed);
        }

        // Property 1: RegexRule round-trip
        #[test]
        fn regex_rule_roundtrip(rule in regex_rule_strategy()) {
            let json = serde_json::to_string(&rule).unwrap();
            let parsed: RegexRule = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(rule, parsed);
        }

        // Property 1: UrlRule round-trip
        #[test]
        fn url_rule_roundtrip(rule in url_rule_strategy()) {
            let json = serde_json::to_string(&rule).unwrap();
            let parsed: UrlRule = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(rule, parsed);
        }

        // Property 1: RequiredRule round-trip
        #[test]
        fn required_rule_roundtrip(rule in required_rule_strategy()) {
            let json = serde_json::to_string(&rule).unwrap();
            let parsed: RequiredRule = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(rule, parsed);
        }

        // Property 1: AsyncUniqueRule round-trip
        #[test]
        fn async_unique_rule_roundtrip(rule in async_unique_rule_strategy()) {
            let json = serde_json::to_string(&rule).unwrap();
            let parsed: AsyncUniqueRule = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(rule, parsed);
        }

        // Property 1: AsyncExistsRule round-trip
        #[test]
        fn async_exists_rule_roundtrip(rule in async_exists_rule_strategy()) {
            let json = serde_json::to_string(&rule).unwrap();
            let parsed: AsyncExistsRule = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(rule, parsed);
        }

        // Property 1: AsyncApiRule round-trip
        #[test]
        fn async_api_rule_roundtrip(rule in async_api_rule_strategy()) {
            let json = serde_json::to_string(&rule).unwrap();
            let parsed: AsyncApiRule = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(rule, parsed);
        }

        // Property 1: SerializableRule round-trip (comprehensive)
        #[test]
        fn serializable_rule_roundtrip(rule in serializable_rule_strategy()) {
            let json = serde_json::to_string(&rule).unwrap();
            let parsed: SerializableRule = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(rule, parsed);
        }

        // Property 1: Pretty print produces valid attribute syntax
        #[test]
        fn serializable_rule_pretty_print_valid(rule in serializable_rule_strategy()) {
            let pretty = rule.pretty_print();
            // Should start with #[validate(
            prop_assert!(pretty.starts_with("#[validate("));
            // Should end with )]
            prop_assert!(pretty.ends_with(")]"));
        }
    }

    // **Feature: v1-features-roadmap, Property 2: Sync validation correctness**
    // **Validates: Requirements 1.3**
    //
    // For any input value and sync validation rule (email, length, range, regex, url),
    // the validation result SHALL correctly identify valid and invalid inputs
    // according to the rule specification.

    // Strategy for valid emails
    fn valid_email_strategy() -> impl Strategy<Value = String> {
        ("[a-z]{1,10}@[a-z]{1,10}\\.[a-z]{2,4}").prop_map(|s| s)
    }

    // Strategy for invalid emails
    fn invalid_email_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("invalid".to_string()),
            Just("@domain.com".to_string()),
            Just("user@".to_string()),
            Just("".to_string()),
            Just("no-at-sign".to_string()),
        ]
    }

    // Strategy for strings within length bounds
    #[allow(dead_code)]
    fn string_within_bounds(min: usize, max: usize) -> impl Strategy<Value = String> {
        prop::collection::vec(prop::char::range('a', 'z'), min..=max)
            .prop_map(|chars| chars.into_iter().collect())
    }

    // Strategy for strings outside length bounds (too short)
    #[allow(dead_code)]
    fn string_too_short(min: usize) -> impl Strategy<Value = String> {
        if min == 0 {
            Just("".to_string()).boxed()
        } else {
            prop::collection::vec(prop::char::range('a', 'z'), 0..min)
                .prop_map(|chars| chars.into_iter().collect())
                .boxed()
        }
    }

    // Strategy for strings outside length bounds (too long)
    #[allow(dead_code)]
    fn string_too_long(max: usize) -> impl Strategy<Value = String> {
        prop::collection::vec(prop::char::range('a', 'z'), (max + 1)..=(max + 10))
            .prop_map(|chars| chars.into_iter().collect())
    }

    // Strategy for valid URLs
    fn valid_url_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("https://example.com".to_string()),
            Just("http://test.org/path".to_string()),
            Just("https://sub.domain.com/path?query=1".to_string()),
            Just("ftp://files.example.com/file.txt".to_string()),
        ]
    }

    // Strategy for invalid URLs
    fn invalid_url_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("not-a-url".to_string()),
            Just("example.com".to_string()),
            Just("://missing-scheme.com".to_string()),
            Just("".to_string()),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Property 2: EmailRule accepts valid emails
        #[test]
        fn email_rule_accepts_valid(email in valid_email_strategy()) {
            let rule = EmailRule::new();
            prop_assert!(rule.validate(&email).is_ok());
        }

        // Property 2: EmailRule rejects invalid emails
        #[test]
        fn email_rule_rejects_invalid(email in invalid_email_strategy()) {
            let rule = EmailRule::new();
            prop_assert!(rule.validate(&email).is_err());
        }

        // Property 2: LengthRule accepts strings within bounds
        #[test]
        fn length_rule_accepts_within_bounds(
            min in 1usize..10,
            max in 10usize..50,
        ) {
            prop_assume!(min <= max);
            let rule = LengthRule::new(min, max);
            let value: String = (0..((min + max) / 2)).map(|_| 'a').collect();
            prop_assert!(rule.validate(&value).is_ok());
        }

        // Property 2: LengthRule rejects strings below minimum
        #[test]
        fn length_rule_rejects_too_short(
            min in 2usize..10,
            max in 10usize..50,
        ) {
            prop_assume!(min <= max);
            let rule = LengthRule::new(min, max);
            let value: String = (0..(min - 1)).map(|_| 'a').collect();
            prop_assert!(rule.validate(&value).is_err());
        }

        // Property 2: LengthRule rejects strings above maximum
        #[test]
        fn length_rule_rejects_too_long(
            min in 1usize..10,
            max in 10usize..50,
        ) {
            prop_assume!(min <= max);
            let rule = LengthRule::new(min, max);
            let value: String = (0..(max + 1)).map(|_| 'a').collect();
            prop_assert!(rule.validate(&value).is_err());
        }

        // Property 2: RangeRule accepts values within bounds
        #[test]
        fn range_rule_accepts_within_bounds(
            min in -100i64..0,
            max in 0i64..100,
            value in -100i64..100,
        ) {
            prop_assume!(min <= max);
            prop_assume!(value >= min && value <= max);
            let rule = RangeRule::new(min, max);
            prop_assert!(rule.validate(&value).is_ok());
        }

        // Property 2: RangeRule rejects values below minimum
        #[test]
        fn range_rule_rejects_below_min(
            min in 0i64..50,
            max in 50i64..100,
        ) {
            prop_assume!(min <= max);
            let rule = RangeRule::new(min, max);
            let value = min - 1;
            prop_assert!(rule.validate(&value).is_err());
        }

        // Property 2: RangeRule rejects values above maximum
        #[test]
        fn range_rule_rejects_above_max(
            min in 0i64..50,
            max in 50i64..100,
        ) {
            prop_assume!(min <= max);
            let rule = RangeRule::new(min, max);
            let value = max + 1;
            prop_assert!(rule.validate(&value).is_err());
        }

        // Property 2: RegexRule accepts matching strings
        #[test]
        fn regex_rule_accepts_matching(digits in "[0-9]{1,10}") {
            let rule = RegexRule::new(r"^\d+$");
            prop_assert!(rule.validate(&digits).is_ok());
        }

        // Property 2: RegexRule rejects non-matching strings
        #[test]
        fn regex_rule_rejects_non_matching(letters in "[a-z]{1,10}") {
            let rule = RegexRule::new(r"^\d+$");
            prop_assert!(rule.validate(&letters).is_err());
        }

        // Property 2: UrlRule accepts valid URLs
        #[test]
        fn url_rule_accepts_valid(url in valid_url_strategy()) {
            let rule = UrlRule::new();
            prop_assert!(rule.validate(&url).is_ok());
        }

        // Property 2: UrlRule rejects invalid URLs
        #[test]
        fn url_rule_rejects_invalid(url in invalid_url_strategy()) {
            let rule = UrlRule::new();
            prop_assert!(rule.validate(&url).is_err());
        }

        // Property 2: RequiredRule accepts non-empty strings
        #[test]
        fn required_rule_accepts_non_empty(value in "[a-zA-Z0-9]{1,50}") {
            let rule = RequiredRule::new();
            prop_assert!(rule.validate(&value).is_ok());
        }

        // Property 2: RequiredRule rejects empty/whitespace strings
        #[test]
        fn required_rule_rejects_empty(value in prop_oneof![
            Just("".to_string()),
            Just("   ".to_string()),
            Just("\t\n".to_string()),
        ]) {
            let rule = RequiredRule::new();
            prop_assert!(rule.validate(&value).is_err());
        }
    }
}

#[cfg(test)]
mod async_property_tests {
    use crate::v2::context::{DatabaseValidator, HttpValidator, ValidationContextBuilder};
    use crate::v2::error::ValidationErrors;
    use crate::v2::rules::*;
    use crate::v2::traits::{AsyncValidate, AsyncValidationRule, Validate, ValidationRule};
    use async_trait::async_trait;
    use proptest::prelude::*;
    use std::collections::HashSet;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // **Feature: v1-features-roadmap, Property 3: Async validation execution**
    // **Validates: Requirements 1.4, 1.5, 1.6**
    //
    // For any struct with async validation rules, calling `validate_async()`
    // SHALL execute all async validators and aggregate their results.

    // Mock database validator that tracks which values exist and are unique
    struct MockDbValidator {
        existing_values: Arc<Mutex<HashSet<String>>>,
        unique_values: Arc<Mutex<HashSet<String>>>,
    }

    impl MockDbValidator {
        fn new(existing: Vec<String>, taken: Vec<String>) -> Self {
            Self {
                existing_values: Arc::new(Mutex::new(existing.into_iter().collect())),
                unique_values: Arc::new(Mutex::new(taken.into_iter().collect())),
            }
        }
    }

    #[async_trait]
    impl DatabaseValidator for MockDbValidator {
        async fn exists(&self, _table: &str, _column: &str, value: &str) -> Result<bool, String> {
            let existing = self.existing_values.lock().await;
            Ok(existing.contains(value))
        }

        async fn is_unique(
            &self,
            _table: &str,
            _column: &str,
            value: &str,
        ) -> Result<bool, String> {
            let taken = self.unique_values.lock().await;
            Ok(!taken.contains(value))
        }

        async fn is_unique_except(
            &self,
            _table: &str,
            _column: &str,
            value: &str,
            except_id: &str,
        ) -> Result<bool, String> {
            let taken = self.unique_values.lock().await;
            // Value is unique if it's not taken, or if it belongs to the excluded ID
            Ok(!taken.contains(value) || value == except_id)
        }
    }

    // Mock HTTP validator
    struct MockHttpValidator {
        valid_values: Arc<Mutex<HashSet<String>>>,
    }

    impl MockHttpValidator {
        fn new(valid: Vec<String>) -> Self {
            Self {
                valid_values: Arc::new(Mutex::new(valid.into_iter().collect())),
            }
        }
    }

    #[async_trait]
    impl HttpValidator for MockHttpValidator {
        async fn validate(&self, _endpoint: &str, value: &str) -> Result<bool, String> {
            let valid = self.valid_values.lock().await;
            Ok(valid.contains(value))
        }
    }

    // Test struct with async validation
    struct TestUser {
        email: String,
        category_id: String,
    }

    impl Validate for TestUser {
        fn validate_with_group(
            &self,
            _group: crate::v2::group::ValidationGroup,
        ) -> Result<(), ValidationErrors> {
            let mut errors = ValidationErrors::new();

            // Sync validation: email format
            let email_rule = EmailRule::new();
            if let Err(e) = email_rule.validate(&self.email) {
                errors.add("email", e);
            }

            errors.into_result()
        }
    }

    #[async_trait]
    impl AsyncValidate for TestUser {
        async fn validate_async_with_group(
            &self,
            ctx: &crate::v2::context::ValidationContext,
            _group: crate::v2::group::ValidationGroup,
        ) -> Result<(), ValidationErrors> {
            let mut errors = ValidationErrors::new();

            // Async validation: email uniqueness
            let unique_rule = AsyncUniqueRule::new("users", "email");
            if let Err(e) = unique_rule.validate_async(&self.email, ctx).await {
                errors.add("email", e);
            }

            // Async validation: category exists
            let exists_rule = AsyncExistsRule::new("categories", "id");
            if let Err(e) = exists_rule.validate_async(&self.category_id, ctx).await {
                errors.add("category_id", e);
            }

            errors.into_result()
        }
    }

    // Strategy for generating valid emails
    fn valid_email_strategy() -> impl Strategy<Value = String> {
        "[a-z]{1,10}@[a-z]{1,10}\\.[a-z]{2,4}"
    }

    // Strategy for generating category IDs
    fn category_id_strategy() -> impl Strategy<Value = String> {
        "[a-z]{1,10}"
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Property 3: Async validation executes all validators
        #[test]
        fn async_validation_executes_all_validators(
            email in valid_email_strategy(),
            category_id in category_id_strategy(),
            email_taken in proptest::bool::ANY,
            category_exists in proptest::bool::ANY,
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                // Set up mock validators based on test parameters
                let taken_emails = if email_taken {
                    vec![email.clone()]
                } else {
                    vec![]
                };

                let existing_categories = if category_exists {
                    vec![category_id.clone()]
                } else {
                    vec![]
                };

                let db = MockDbValidator::new(existing_categories, taken_emails);
                let ctx = ValidationContextBuilder::new().database(db).build();

                let user = TestUser {
                    email: email.clone(),
                    category_id: category_id.clone(),
                };

                let result = user.validate_async(&ctx).await;

                // Verify the result matches expectations
                if email_taken || !category_exists {
                    // Should have errors
                    prop_assert!(result.is_err());
                    let errors = result.unwrap_err();

                    if email_taken {
                        prop_assert!(errors.get("email").is_some());
                    }
                    if !category_exists {
                        prop_assert!(errors.get("category_id").is_some());
                    }
                } else {
                    // Should pass
                    prop_assert!(result.is_ok());
                }

                Ok(())
            })?;
        }

        // Property 3: Full validation runs both sync and async
        #[test]
        fn full_validation_runs_sync_and_async(
            email in valid_email_strategy(),
            category_id in category_id_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                // Set up validators where everything is valid
                let db = MockDbValidator::new(
                    vec![category_id.clone()],  // category exists
                    vec![],  // no emails taken
                );
                let ctx = ValidationContextBuilder::new().database(db).build();

                let user = TestUser {
                    email: email.clone(),
                    category_id: category_id.clone(),
                };

                // Full validation should pass
                let result = user.validate_full(&ctx).await;
                prop_assert!(result.is_ok());

                Ok(())
            })?;
        }

        // Property 3: Async unique rule respects exclude_id for updates
        #[test]
        fn async_unique_respects_exclude_id(
            email in valid_email_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                // Email is "taken" but we're updating the same record
                let db = MockDbValidator::new(vec![], vec![email.clone()]);
                let ctx = ValidationContextBuilder::new()
                    .database(db)
                    .exclude_id(email.clone())  // Exclude this ID from uniqueness check
                    .build();

                let rule = AsyncUniqueRule::new("users", "email");
                let result = rule.validate_async(&email, &ctx).await;

                // Should pass because we're excluding our own ID
                prop_assert!(result.is_ok());

                Ok(())
            })?;
        }

        // Property 3: Async API rule validates against HTTP endpoint
        #[test]
        fn async_api_rule_validates(
            value in "[a-z]{1,10}",
            is_valid in proptest::bool::ANY,
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let valid_values = if is_valid {
                    vec![value.clone()]
                } else {
                    vec![]
                };

                let http = MockHttpValidator::new(valid_values);
                let ctx = ValidationContextBuilder::new().http(http).build();

                let rule = AsyncApiRule::new("https://api.example.com/validate");
                let result = rule.validate_async(&value, &ctx).await;

                if is_valid {
                    prop_assert!(result.is_ok());
                } else {
                    prop_assert!(result.is_err());
                }

                Ok(())
            })?;
        }
    }
}

#[cfg(test)]
mod custom_message_property_tests {
    use crate::v2::error::RuleError;
    use crate::v2::rules::*;
    use crate::v2::traits::ValidationRule;
    use proptest::prelude::*;

    // **Feature: v1-features-roadmap, Property 4: Custom error messages**
    // **Validates: Requirements 1.7**
    //
    // For any validation failure with a custom message defined,
    // the error response SHALL contain the exact custom message.

    // Strategy for generating custom messages
    fn custom_message_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 ]{1,100}"
    }

    // Strategy for invalid emails (guaranteed to fail validation)
    fn invalid_email_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("invalid".to_string()),
            Just("no-at-sign".to_string()),
            Just("@missing-local".to_string()),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Property 4: EmailRule returns custom message on failure
        #[test]
        fn email_rule_returns_custom_message(
            custom_msg in custom_message_strategy(),
            invalid_email in invalid_email_strategy(),
        ) {
            let rule = EmailRule::new().with_message(custom_msg.clone());
            let result = rule.validate(&invalid_email);

            prop_assert!(result.is_err());
            let error = result.unwrap_err();
            prop_assert_eq!(error.message, custom_msg);
        }

        // Property 4: LengthRule returns custom message on failure
        #[test]
        fn length_rule_returns_custom_message(
            custom_msg in custom_message_strategy(),
            min in 5usize..10,
            max in 10usize..20,
        ) {
            prop_assume!(min <= max);
            let rule = LengthRule::new(min, max).with_message(custom_msg.clone());

            // Use a string that's too short
            let short_value = "ab";
            let result = rule.validate(short_value);

            prop_assert!(result.is_err());
            let error = result.unwrap_err();
            prop_assert_eq!(error.message, custom_msg);
        }

        // Property 4: RangeRule returns custom message on failure
        #[test]
        fn range_rule_returns_custom_message(
            custom_msg in custom_message_strategy(),
            min in 10i64..50,
            max in 50i64..100,
        ) {
            prop_assume!(min <= max);
            let rule = RangeRule::new(min, max).with_message(custom_msg.clone());

            // Use a value below minimum
            let low_value = min - 1;
            let result = rule.validate(&low_value);

            prop_assert!(result.is_err());
            let error = result.unwrap_err();
            prop_assert_eq!(error.message, custom_msg);
        }

        // Property 4: RequiredRule returns custom message on failure
        #[test]
        fn required_rule_returns_custom_message(
            custom_msg in custom_message_strategy(),
        ) {
            let rule = RequiredRule::new().with_message(custom_msg.clone());
            let result = rule.validate("");

            prop_assert!(result.is_err());
            let error = result.unwrap_err();
            prop_assert_eq!(error.message, custom_msg);
        }

        // Property 4: UrlRule returns custom message on failure
        #[test]
        fn url_rule_returns_custom_message(
            custom_msg in custom_message_strategy(),
        ) {
            let rule = UrlRule::new().with_message(custom_msg.clone());
            let result = rule.validate("not-a-url");

            prop_assert!(result.is_err());
            let error = result.unwrap_err();
            prop_assert_eq!(error.message, custom_msg);
        }

        // Property 4: RegexRule returns custom message on failure
        #[test]
        fn regex_rule_returns_custom_message(
            custom_msg in custom_message_strategy(),
        ) {
            let rule = RegexRule::new(r"^\d+$").with_message(custom_msg.clone());
            let result = rule.validate("not-digits");

            prop_assert!(result.is_err());
            let error = result.unwrap_err();
            prop_assert_eq!(error.message, custom_msg);
        }

        // Property 4: Error message interpolation works correctly
        #[test]
        fn error_message_interpolation(
            min in 1i64..50,
            max in 50i64..100,
            actual in 100i64..200,
        ) {
            prop_assume!(min <= max);
            prop_assume!(actual > max);

            let error = RuleError::new("range", "Value {actual} must be between {min} and {max}")
                .param("min", min)
                .param("max", max)
                .param("actual", actual);

            let interpolated = error.interpolate_message();

            // Check that all placeholders were replaced
            let min_placeholder = "{min}";
            let max_placeholder = "{max}";
            let actual_placeholder = "{actual}";
            prop_assert!(!interpolated.contains(min_placeholder));
            prop_assert!(!interpolated.contains(max_placeholder));
            prop_assert!(!interpolated.contains(actual_placeholder));

            // Check that values are present
            prop_assert!(interpolated.contains(&min.to_string()));
            prop_assert!(interpolated.contains(&max.to_string()));
            prop_assert!(interpolated.contains(&actual.to_string()));
        }
    }
}

#[cfg(test)]
mod validation_group_property_tests {
    use crate::v2::error::ValidationErrors;
    use crate::v2::group::{GroupedRules, ValidationGroup};
    use crate::v2::rules::*;
    use crate::v2::traits::ValidationRule;
    use proptest::prelude::*;

    // **Feature: v1-features-roadmap, Property 5: Validation groups**
    // **Validates: Requirements 1.8**
    //
    // For any struct with group-specific validation rules, validation SHALL
    // apply only the rules matching the specified group.

    // Strategy for generating validation groups
    fn validation_group_strategy() -> impl Strategy<Value = ValidationGroup> {
        prop_oneof![
            Just(ValidationGroup::Create),
            Just(ValidationGroup::Update),
            Just(ValidationGroup::Default),
            "[a-z]{1,10}".prop_map(ValidationGroup::Custom),
        ]
    }

    // A test struct that validates differently based on group
    struct GroupedUser {
        id: Option<i64>,
        email: String,
        password: Option<String>,
    }

    impl GroupedUser {
        fn validate_for_group(&self, group: &ValidationGroup) -> Result<(), ValidationErrors> {
            let mut errors = ValidationErrors::new();

            // Email is always required (Default group)
            let email_rules =
                GroupedRules::new().always(RequiredRule::new().with_message("Email is required"));

            for rule in email_rules.for_group(group) {
                if let Err(e) = rule.validate(&self.email) {
                    errors.add("email", e);
                }
            }

            // ID is required only for updates
            let id_rules = GroupedRules::new()
                .on_update(RequiredRule::new().with_message("ID is required for updates"));

            for rule in id_rules.for_group(group) {
                if let Err(e) = rule.validate(&self.id) {
                    errors.add("id", e);
                }
            }

            // Password is required only for creates
            let password_rules = GroupedRules::new()
                .on_create(RequiredRule::new().with_message("Password is required for new users"));

            for rule in password_rules.for_group(group) {
                if let Err(e) = rule.validate(&self.password) {
                    errors.add("password", e);
                }
            }

            errors.into_result()
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Property 5: Default group rules apply to all groups
        #[test]
        fn default_rules_apply_to_all_groups(group_val in validation_group_strategy()) {
            let user = GroupedUser {
                id: Some(1),
                email: "".to_string(),  // Invalid - should fail for all groups
                password: Some("password".to_string()),
            };

            let result = user.validate_for_group(&group_val);

            // Email validation should fail for all groups
            prop_assert!(result.is_err());
            let errors = result.unwrap_err();
            prop_assert!(errors.get("email").is_some());
        }

        // Property 5: GroupedRules filters correctly
        #[test]
        fn grouped_rules_filter_by_group(
            create_value in "[a-z]{1,5}",
            update_value in "[a-z]{1,5}",
            always_value in "[a-z]{1,5}",
        ) {
            // Skip if values are the same (can't distinguish them)
            prop_assume!(create_value != update_value);
            prop_assume!(create_value != always_value);
            prop_assume!(update_value != always_value);

            let rules = GroupedRules::new()
                .on_create(create_value.clone())
                .on_update(update_value.clone())
                .always(always_value.clone());

            // Create group should get create + always rules
            let create_rules: Vec<_> = rules.for_group(&ValidationGroup::Create).collect();
            prop_assert_eq!(create_rules.len(), 2);
            prop_assert!(create_rules.contains(&&create_value));
            prop_assert!(create_rules.contains(&&always_value));
            prop_assert!(!create_rules.contains(&&update_value));

            // Update group should get update + always rules
            let update_rules: Vec<_> = rules.for_group(&ValidationGroup::Update).collect();
            prop_assert_eq!(update_rules.len(), 2);
            prop_assert!(update_rules.contains(&&update_value));
            prop_assert!(update_rules.contains(&&always_value));
            prop_assert!(!update_rules.contains(&&create_value));

            // Default group should get only default rules
            let default_rules: Vec<_> = rules.for_group(&ValidationGroup::Default).collect();
            prop_assert_eq!(default_rules.len(), 1);
            prop_assert!(default_rules.contains(&&always_value));
            prop_assert!(!default_rules.contains(&&create_value));
            prop_assert!(!default_rules.contains(&&update_value));
        }

        // Property 5: Custom groups work correctly
        #[test]
        fn custom_groups_work(custom_name in "[a-z]{1,10}") {
            let custom_group = ValidationGroup::Custom(custom_name.clone());

            let rules = GroupedRules::new()
                .add("custom_rule", custom_group.clone())
                .always("always_rule");

            // Custom group should get its own rule + always rules
            let custom_rules: Vec<_> = rules.for_group(&custom_group).collect();
            prop_assert_eq!(custom_rules.len(), 2);
            prop_assert!(custom_rules.contains(&&"custom_rule"));
            prop_assert!(custom_rules.contains(&&"always_rule"));

            // Other groups should not get the custom rule
            let create_rules: Vec<_> = rules.for_group(&ValidationGroup::Create).collect();
            prop_assert_eq!(create_rules.len(), 1);
            prop_assert!(!create_rules.contains(&&"custom_rule"));
        }

        // Property 5: Group matching is symmetric for Default
        #[test]
        fn default_group_matching_asymmetric(group_val in validation_group_strategy()) {
            // Default matches everything (rules in Default group apply to all contexts)
            prop_assert!(ValidationGroup::Default.matches(&group_val));

            // Contexts match Default only if they ARE Default
            // (rules in specific groups do NOT apply to Default context)
            if group_val == ValidationGroup::Default {
                prop_assert!(group_val.matches(&ValidationGroup::Default));
            } else {
                prop_assert!(!group_val.matches(&ValidationGroup::Default));
            }
        }
    }

    // Non-proptest tests for specific group behavior
    #[test]
    fn create_rules_apply_only_to_create() {
        let user = GroupedUser {
            id: Some(1),
            email: "test@example.com".to_string(),
            password: None, // Missing password
        };

        // Should fail for Create group
        let create_result = user.validate_for_group(&ValidationGroup::Create);
        assert!(create_result.is_err());
        let create_errors = create_result.unwrap_err();
        assert!(create_errors.get("password").is_some());

        // Should pass for Update group (password not required)
        let update_result = user.validate_for_group(&ValidationGroup::Update);
        assert!(update_result.is_ok());
    }

    #[test]
    fn update_rules_apply_only_to_update() {
        let user = GroupedUser {
            id: None, // Missing ID
            email: "test@example.com".to_string(),
            password: Some("password".to_string()),
        };

        // Should fail for Update group
        let update_result = user.validate_for_group(&ValidationGroup::Update);
        assert!(update_result.is_err());
        let update_errors = update_result.unwrap_err();
        assert!(update_errors.get("id").is_some());

        // Should pass for Create group (ID not required)
        let create_result = user.validate_for_group(&ValidationGroup::Create);
        assert!(create_result.is_ok());
    }

    #[test]
    fn non_default_groups_match_only_self() {
        // Create only matches Create
        assert!(ValidationGroup::Create.matches(&ValidationGroup::Create));
        assert!(!ValidationGroup::Create.matches(&ValidationGroup::Default));
        assert!(!ValidationGroup::Create.matches(&ValidationGroup::Update));

        // Update only matches Update
        assert!(ValidationGroup::Update.matches(&ValidationGroup::Update));
        assert!(!ValidationGroup::Update.matches(&ValidationGroup::Default));
        assert!(!ValidationGroup::Update.matches(&ValidationGroup::Create));
    }
}
