#[cfg(feature = "i18n")]
use rust_i18n::t;

/// Helper to translate a message.
///
/// Falls back to the message key if no translation is found.
///
/// # Arguments
///
/// * `key` - The message key (e.g. "validation.email.invalid")
/// * `locale` - The locale to use (e.g. "en", "tr"). If None, uses default.
pub fn translate(key: &str, locale: Option<&str>) -> String {
    #[cfg(feature = "i18n")]
    {
        let result = if let Some(locale) = locale {
            t!(key, locale = locale).to_string()
        } else {
            t!(key).to_string()
        };

        if result == key {
            t!(key, locale = "en").to_string()
        } else {
            result
        }
    }

    #[cfg(not(feature = "i18n"))]
    {
        let _ = locale;
        fallback_message(key)
    }
}

/// Helper to translate with arguments.
pub fn translate_with_args(key: &str, locale: Option<&str>, _args: &[(&str, &str)]) -> String {
    translate(key, locale)
}

#[cfg(not(feature = "i18n"))]
fn fallback_message(key: &str) -> String {
    match key {
        "validation.email.invalid" => "Invalid email format",
        "validation.length.min" => "Length must be at least %{min} characters",
        "validation.length.max" => "Length must be at most %{max} characters",
        "validation.length.exact" => "Length must be exactly %{len} characters",
        "validation.range.min" => "Value must be at least %{min}",
        "validation.range.max" => "Value must be at most %{max}",
        "validation.range.between" => "Value must be between %{min} and %{max}",
        "validation.required.missing" => "This field is required",
        "validation.url.invalid" => "Invalid URL format",
        "validation.regex.mismatch" => "Value does not match pattern: %{pattern}",
        "validation.unique.taken" => "This value is already taken",
        "validation.exists.not_found" => "Value does not exist",
        "validation.api.invalid" => "External validation failed",
        "validation.credit_card.invalid_format" => "Invalid credit card format",
        "validation.credit_card.invalid" => "Invalid credit card number",
        "validation.ip.v4_required" => "IPv4 address required",
        "validation.ip.v6_required" => "IPv6 address required",
        "validation.ip.invalid" => "Invalid IP address",
        "validation.phone.invalid" => "Invalid phone number",
        "validation.contains.missing" => "Required value is missing",
        other => other,
    }
    .to_string()
}
