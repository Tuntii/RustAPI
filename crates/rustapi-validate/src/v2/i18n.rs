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
    let result = if let Some(locale) = locale {
        t!(key, locale = locale).to_string()
    } else {
        t!(key).to_string()
    };

    // Fallback to English if translation is missing (returns key)
    if result == key {
        t!(key, locale = "en").to_string()
    } else {
        result
    }
}

/// Helper to translate with arguments.
pub fn translate_with_args(key: &str, locale: Option<&str>, _args: &[(&str, &str)]) -> String {
    if let Some(locale) = locale {
        // rust-i18n t! macro doesn't support dynamic args easily in this wrapped form
        // We might need to use the lower level API or just interpolate ourselves
        // For now let's use the basic t! with variable interpolation if possible
        // But t! requires string literals for keys mostly or known args.
        // Let's stick to basic translation for now and use our existing interpolation
        // in RuleError for variable replacement.
        t!(key, locale = locale).to_string()
    } else {
        t!(key).to_string()
    }
}
