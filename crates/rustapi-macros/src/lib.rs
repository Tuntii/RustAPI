//!
//! This crate provides the attribute macros used in RustAPI:
//!
//! - `#[rustapi::main]` - Main entry point macro
//! - `#[rustapi::get("/path")]` - GET route handler
//! - `#[rustapi::post("/path")]` - POST route handler
//! - `#[rustapi::put("/path")]` - PUT route handler
//! - `#[rustapi::patch("/path")]` - PATCH route handler
//! - `#[rustapi::delete("/path")]` - DELETE route handler
//! - `#[derive(Validate)]` - Validation derive macro
//!
//! ## Debugging
//!
//! Set `RUSTAPI_DEBUG=1` environment variable during compilation to see
//! expanded macro output for debugging purposes.

use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Expr, Fields, FnArg, GenericArgument, ItemFn,
    Lit, LitStr, Meta, PathArguments, ReturnType, Type,
};

mod api_error;

/// Derive macro for `ApiError`
///
/// # Example
///
/// ```rust,ignore
/// #[derive(ApiError)]
/// enum MyError {
///     #[error(status = 404, message = "User not found")]
///     UserNotFound,
/// }
/// ```

/// Auto-register a schema type for zero-config OpenAPI.
///
/// Attach this to a `struct` or `enum` that also derives `Schema` (utoipa::ToSchema).
/// This ensures the type is registered into RustAPI's OpenAPI components even if it is
/// only referenced indirectly (e.g. as a nested field type).
///
/// ```rust,ignore
/// use rustapi_rs::prelude::*;
///
/// #[rustapi_rs::schema]
/// #[derive(Serialize, Schema)]
/// struct UserInfo { /* ... */ }
/// ```
#[proc_macro_attribute]
pub fn schema(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::Item);

    let (ident, generics) = match &input {
        syn::Item::Struct(s) => (&s.ident, &s.generics),
        syn::Item::Enum(e) => (&e.ident, &e.generics),
        _ => {
            return syn::Error::new_spanned(
                &input,
                "#[rustapi_rs::schema] can only be used on structs or enums",
            )
            .to_compile_error()
            .into();
        }
    };

    if !generics.params.is_empty() {
        return syn::Error::new_spanned(
            generics,
            "#[rustapi_rs::schema] does not support generic types",
        )
        .to_compile_error()
        .into();
    }

    let registrar_ident = syn::Ident::new(
        &format!("__RUSTAPI_AUTO_SCHEMA_{}", ident),
        proc_macro2::Span::call_site(),
    );

    let expanded = quote! {
        #input

        #[allow(non_upper_case_globals)]
        #[::rustapi_rs::__private::linkme::distributed_slice(::rustapi_rs::__private::AUTO_SCHEMAS)]
        #[linkme(crate = ::rustapi_rs::__private::linkme)]
        static #registrar_ident: fn(&mut ::rustapi_rs::__private::rustapi_openapi::OpenApiSpec) =
            |spec: &mut ::rustapi_rs::__private::rustapi_openapi::OpenApiSpec| {
                spec.register_in_place::<#ident>();
            };
    };

    debug_output("schema", &expanded);
    expanded.into()
}

fn extract_schema_types(ty: &Type, out: &mut Vec<Type>, allow_leaf: bool) {
    match ty {
        Type::Reference(r) => extract_schema_types(&r.elem, out, allow_leaf),
        Type::Path(tp) => {
            let Some(seg) = tp.path.segments.last() else {
                return;
            };

            let ident = seg.ident.to_string();

            let unwrap_first_generic = |out: &mut Vec<Type>| {
                if let PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        extract_schema_types(inner, out, true);
                    }
                }
            };

            match ident.as_str() {
                // Request/response wrappers
                "Json" | "ValidatedJson" | "Created" => {
                    unwrap_first_generic(out);
                }
                // WithStatus<T, CODE>
                "WithStatus" => {
                    if let PathArguments::AngleBracketed(args) = &seg.arguments {
                        if let Some(GenericArgument::Type(inner)) = args.args.first() {
                            extract_schema_types(inner, out, true);
                        }
                    }
                }
                // Common combinators
                "Option" | "Result" => {
                    if let PathArguments::AngleBracketed(args) = &seg.arguments {
                        if let Some(GenericArgument::Type(inner)) = args.args.first() {
                            extract_schema_types(inner, out, allow_leaf);
                        }
                    }
                }
                _ => {
                    if allow_leaf {
                        out.push(ty.clone());
                    }
                }
            }
        }
        _ => {}
    }
}

fn collect_handler_schema_types(input: &ItemFn) -> Vec<Type> {
    let mut found: Vec<Type> = Vec::new();

    for arg in &input.sig.inputs {
        if let FnArg::Typed(pat_ty) = arg {
            extract_schema_types(&pat_ty.ty, &mut found, false);
        }
    }

    if let ReturnType::Type(_, ty) = &input.sig.output {
        extract_schema_types(ty, &mut found, false);
    }

    // Dedup by token string.
    let mut seen = HashSet::<String>::new();
    found
        .into_iter()
        .filter(|t| seen.insert(quote!(#t).to_string()))
        .collect()
}

/// Check if RUSTAPI_DEBUG is enabled at compile time
fn is_debug_enabled() -> bool {
    std::env::var("RUSTAPI_DEBUG")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Print debug output if RUSTAPI_DEBUG=1 is set
fn debug_output(name: &str, tokens: &proc_macro2::TokenStream) {
    if is_debug_enabled() {
        eprintln!("\n=== RUSTAPI_DEBUG: {} ===", name);
        eprintln!("{}", tokens);
        eprintln!("=== END {} ===\n", name);
    }
}

/// Validate route path syntax at compile time
///
/// Returns Ok(()) if the path is valid, or Err with a descriptive error message.
fn validate_path_syntax(path: &str, span: proc_macro2::Span) -> Result<(), syn::Error> {
    // Path must start with /
    if !path.starts_with('/') {
        return Err(syn::Error::new(
            span,
            format!("route path must start with '/', got: \"{}\"", path),
        ));
    }

    // Check for empty path segments (double slashes)
    if path.contains("//") {
        return Err(syn::Error::new(
            span,
            format!(
                "route path contains empty segment (double slash): \"{}\"",
                path
            ),
        ));
    }

    // Validate path parameter syntax
    let mut brace_depth = 0;
    let mut param_start = None;

    for (i, ch) in path.char_indices() {
        match ch {
            '{' => {
                if brace_depth > 0 {
                    return Err(syn::Error::new(
                        span,
                        format!(
                            "nested braces are not allowed in route path at position {}: \"{}\"",
                            i, path
                        ),
                    ));
                }
                brace_depth += 1;
                param_start = Some(i);
            }
            '}' => {
                if brace_depth == 0 {
                    return Err(syn::Error::new(
                        span,
                        format!(
                            "unmatched closing brace '}}' at position {} in route path: \"{}\"",
                            i, path
                        ),
                    ));
                }
                brace_depth -= 1;

                // Check that parameter name is not empty
                if let Some(start) = param_start {
                    let param_name = &path[start + 1..i];
                    if param_name.is_empty() {
                        return Err(syn::Error::new(
                            span,
                            format!(
                                "empty parameter name '{{}}' at position {} in route path: \"{}\"",
                                start, path
                            ),
                        ));
                    }
                    // Validate parameter name contains only valid identifier characters
                    if !param_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                        return Err(syn::Error::new(
                            span,
                            format!(
                                "invalid parameter name '{{{}}}' at position {} - parameter names must contain only alphanumeric characters and underscores: \"{}\"",
                                param_name, start, path
                            ),
                        ));
                    }
                    // Parameter name must not start with a digit
                    if param_name
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_digit())
                        .unwrap_or(false)
                    {
                        return Err(syn::Error::new(
                            span,
                            format!(
                                "parameter name '{{{}}}' cannot start with a digit at position {}: \"{}\"",
                                param_name, start, path
                            ),
                        ));
                    }
                }
                param_start = None;
            }
            // Check for invalid characters in path (outside of parameters)
            _ if brace_depth == 0 => {
                // Allow alphanumeric, -, _, ., /, and common URL characters
                if !ch.is_alphanumeric() && !"-_./*".contains(ch) {
                    return Err(syn::Error::new(
                        span,
                        format!(
                            "invalid character '{}' at position {} in route path: \"{}\"",
                            ch, i, path
                        ),
                    ));
                }
            }
            _ => {}
        }
    }

    // Check for unclosed braces
    if brace_depth > 0 {
        return Err(syn::Error::new(
            span,
            format!(
                "unclosed brace '{{' in route path (missing closing '}}'): \"{}\"",
                path
            ),
        ));
    }

    Ok(())
}

/// Main entry point macro for RustAPI applications
///
/// This macro wraps your async main function with the tokio runtime.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_rs::prelude::*;
///
/// #[rustapi::main]
/// async fn main() -> Result<()> {
///     RustApi::new()
///         .mount(hello)
///         .run("127.0.0.1:8080")
///         .await
/// }
/// ```
#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    let attrs = &input.attrs;
    let vis = &input.vis;
    let sig = &input.sig;
    let block = &input.block;

    let expanded = quote! {
        #(#attrs)*
        #[::tokio::main]
        #vis #sig {
            #block
        }
    };

    debug_output("main", &expanded);

    TokenStream::from(expanded)
}

/// Internal helper to generate route handler macros
fn generate_route_handler(method: &str, attr: TokenStream, item: TokenStream) -> TokenStream {
    let path = parse_macro_input!(attr as LitStr);
    let input = parse_macro_input!(item as ItemFn);

    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis;
    let fn_attrs = &input.attrs;
    let fn_async = &input.sig.asyncness;
    let fn_inputs = &input.sig.inputs;
    let fn_output = &input.sig.output;
    let fn_block = &input.block;
    let fn_generics = &input.sig.generics;

    let schema_types = collect_handler_schema_types(&input);

    let path_value = path.value();

    // Validate path syntax at compile time
    if let Err(err) = validate_path_syntax(&path_value, path.span()) {
        return err.to_compile_error().into();
    }

    // Generate a companion module with route info
    let route_fn_name = syn::Ident::new(&format!("{}_route", fn_name), fn_name.span());
    // Generate unique name for auto-registration static
    let auto_route_name = syn::Ident::new(&format!("__AUTO_ROUTE_{}", fn_name), fn_name.span());

    // Generate unique names for schema registration
    let schema_reg_fn_name =
        syn::Ident::new(&format!("__{}_register_schemas", fn_name), fn_name.span());
    let auto_schema_name = syn::Ident::new(&format!("__AUTO_SCHEMA_{}", fn_name), fn_name.span());

    // Pick the right route helper function based on method
    let route_helper = match method {
        "GET" => quote!(::rustapi_rs::get_route),
        "POST" => quote!(::rustapi_rs::post_route),
        "PUT" => quote!(::rustapi_rs::put_route),
        "PATCH" => quote!(::rustapi_rs::patch_route),
        "DELETE" => quote!(::rustapi_rs::delete_route),
        _ => quote!(::rustapi_rs::get_route),
    };

    // Extract metadata from attributes to chain builder methods
    let mut chained_calls = quote!();

    for attr in fn_attrs {
        // Check for tag, summary, description
        // Use loose matching on the last segment to handle crate renaming or fully qualified paths
        if let Some(ident) = attr.path().segments.last().map(|s| &s.ident) {
            let ident_str = ident.to_string();
            if ident_str == "tag" {
                if let Ok(lit) = attr.parse_args::<LitStr>() {
                    let val = lit.value();
                    chained_calls = quote! { #chained_calls .tag(#val) };
                }
            } else if ident_str == "summary" {
                if let Ok(lit) = attr.parse_args::<LitStr>() {
                    let val = lit.value();
                    chained_calls = quote! { #chained_calls .summary(#val) };
                }
            } else if ident_str == "description" {
                if let Ok(lit) = attr.parse_args::<LitStr>() {
                    let val = lit.value();
                    chained_calls = quote! { #chained_calls .description(#val) };
                }
            }
        }
    }

    let expanded = quote! {
        // The original handler function
        #(#fn_attrs)*
        #fn_vis #fn_async fn #fn_name #fn_generics (#fn_inputs) #fn_output #fn_block

        // Route info function - creates a Route for this handler
        #[doc(hidden)]
        #fn_vis fn #route_fn_name() -> ::rustapi_rs::Route {
            #route_helper(#path_value, #fn_name)
                #chained_calls
        }

        // Auto-register route with linkme
        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        #[::rustapi_rs::__private::linkme::distributed_slice(::rustapi_rs::__private::AUTO_ROUTES)]
        #[linkme(crate = ::rustapi_rs::__private::linkme)]
        static #auto_route_name: fn() -> ::rustapi_rs::Route = #route_fn_name;

        // Auto-register referenced schemas with linkme (best-effort)
        #[doc(hidden)]
        #[allow(non_snake_case)]
        fn #schema_reg_fn_name(spec: &mut ::rustapi_rs::__private::rustapi_openapi::OpenApiSpec) {
            #( spec.register_in_place::<#schema_types>(); )*
        }

        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        #[::rustapi_rs::__private::linkme::distributed_slice(::rustapi_rs::__private::AUTO_SCHEMAS)]
        #[linkme(crate = ::rustapi_rs::__private::linkme)]
        static #auto_schema_name: fn(&mut ::rustapi_rs::__private::rustapi_openapi::OpenApiSpec) = #schema_reg_fn_name;
    };

    debug_output(&format!("{} {}", method, path_value), &expanded);

    TokenStream::from(expanded)
}

/// GET route handler macro
///
/// # Example
///
/// ```rust,ignore
/// #[rustapi::get("/users")]
/// async fn list_users() -> Json<Vec<User>> {
///     Json(vec![])
/// }
///
/// #[rustapi::get("/users/{id}")]
/// async fn get_user(Path(id): Path<i64>) -> Result<User> {
///     Ok(User { id, name: "John".into() })
/// }
/// ```
#[proc_macro_attribute]
pub fn get(attr: TokenStream, item: TokenStream) -> TokenStream {
    generate_route_handler("GET", attr, item)
}

/// POST route handler macro
#[proc_macro_attribute]
pub fn post(attr: TokenStream, item: TokenStream) -> TokenStream {
    generate_route_handler("POST", attr, item)
}

/// PUT route handler macro
#[proc_macro_attribute]
pub fn put(attr: TokenStream, item: TokenStream) -> TokenStream {
    generate_route_handler("PUT", attr, item)
}

/// PATCH route handler macro
#[proc_macro_attribute]
pub fn patch(attr: TokenStream, item: TokenStream) -> TokenStream {
    generate_route_handler("PATCH", attr, item)
}

/// DELETE route handler macro
#[proc_macro_attribute]
pub fn delete(attr: TokenStream, item: TokenStream) -> TokenStream {
    generate_route_handler("DELETE", attr, item)
}

// ============================================
// Route Metadata Macros
// ============================================

/// Tag macro for grouping endpoints in OpenAPI documentation
///
/// # Example
///
/// ```rust,ignore
/// #[rustapi::get("/users")]
/// #[rustapi::tag("Users")]
/// async fn list_users() -> Json<Vec<User>> {
///     Json(vec![])
/// }
/// ```
#[proc_macro_attribute]
pub fn tag(attr: TokenStream, item: TokenStream) -> TokenStream {
    let tag = parse_macro_input!(attr as LitStr);
    let input = parse_macro_input!(item as ItemFn);

    let attrs = &input.attrs;
    let vis = &input.vis;
    let sig = &input.sig;
    let block = &input.block;
    let tag_value = tag.value();

    // Add a doc comment with the tag info for documentation
    let expanded = quote! {
        #[doc = concat!("**Tag:** ", #tag_value)]
        #(#attrs)*
        #vis #sig #block
    };

    TokenStream::from(expanded)
}

/// Summary macro for endpoint summary in OpenAPI documentation
///
/// # Example
///
/// ```rust,ignore
/// #[rustapi::get("/users")]
/// #[rustapi::summary("List all users")]
/// async fn list_users() -> Json<Vec<User>> {
///     Json(vec![])
/// }
/// ```
#[proc_macro_attribute]
pub fn summary(attr: TokenStream, item: TokenStream) -> TokenStream {
    let summary = parse_macro_input!(attr as LitStr);
    let input = parse_macro_input!(item as ItemFn);

    let attrs = &input.attrs;
    let vis = &input.vis;
    let sig = &input.sig;
    let block = &input.block;
    let summary_value = summary.value();

    // Add a doc comment with the summary
    let expanded = quote! {
        #[doc = #summary_value]
        #(#attrs)*
        #vis #sig #block
    };

    TokenStream::from(expanded)
}

/// Description macro for detailed endpoint description in OpenAPI documentation
///
/// # Example
///
/// ```rust,ignore
/// #[rustapi::get("/users")]
/// #[rustapi::description("Returns a list of all users in the system. Supports pagination.")]
/// async fn list_users() -> Json<Vec<User>> {
///     Json(vec![])
/// }
/// ```
#[proc_macro_attribute]
pub fn description(attr: TokenStream, item: TokenStream) -> TokenStream {
    let desc = parse_macro_input!(attr as LitStr);
    let input = parse_macro_input!(item as ItemFn);

    let attrs = &input.attrs;
    let vis = &input.vis;
    let sig = &input.sig;
    let block = &input.block;
    let desc_value = desc.value();

    // Add a doc comment with the description
    let expanded = quote! {
        #[doc = ""]
        #[doc = #desc_value]
        #(#attrs)*
        #vis #sig #block
    };

    TokenStream::from(expanded)
}

// ============================================
// Validation Derive Macro
// ============================================

/// Parsed validation rule from field attributes
#[derive(Debug)]
struct ValidationRuleInfo {
    rule_type: String,
    params: Vec<(String, String)>,
    message: Option<String>,
    #[allow(dead_code)]
    group: Option<String>,
}

/// Parse validation attributes from a field
fn parse_validate_attrs(attrs: &[Attribute]) -> Vec<ValidationRuleInfo> {
    let mut rules = Vec::new();

    for attr in attrs {
        if !attr.path().is_ident("validate") {
            continue;
        }

        // Parse the validate attribute
        if let Ok(meta) = attr.parse_args::<Meta>() {
            if let Some(rule) = parse_validate_meta(&meta) {
                rules.push(rule);
            }
        } else if let Ok(nested) = attr
            .parse_args_with(syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated)
        {
            for meta in nested {
                if let Some(rule) = parse_validate_meta(&meta) {
                    rules.push(rule);
                }
            }
        }
    }

    rules
}

/// Parse a single validation meta item
fn parse_validate_meta(meta: &Meta) -> Option<ValidationRuleInfo> {
    match meta {
        Meta::Path(path) => {
            // Simple rule like #[validate(email)]
            let ident = path.get_ident()?.to_string();
            Some(ValidationRuleInfo {
                rule_type: ident,
                params: Vec::new(),
                message: None,
                group: None,
            })
        }
        Meta::List(list) => {
            // Rule with params like #[validate(length(min = 3, max = 50))]
            let rule_type = list.path.get_ident()?.to_string();
            let mut params = Vec::new();
            let mut message = None;
            let mut group = None;

            // Parse nested params
            if let Ok(nested) = list.parse_args_with(
                syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
            ) {
                for nested_meta in nested {
                    if let Meta::NameValue(nv) = &nested_meta {
                        let key = nv.path.get_ident()?.to_string();
                        let value = expr_to_string(&nv.value)?;

                        if key == "message" {
                            message = Some(value);
                        } else if key == "group" {
                            group = Some(value);
                        } else {
                            params.push((key, value));
                        }
                    }
                }
            }

            Some(ValidationRuleInfo {
                rule_type,
                params,
                message,
                group,
            })
        }
        Meta::NameValue(nv) => {
            // Rule like #[validate(regex = "pattern")]
            let rule_type = nv.path.get_ident()?.to_string();
            let value = expr_to_string(&nv.value)?;

            Some(ValidationRuleInfo {
                rule_type: rule_type.clone(),
                params: vec![(rule_type, value)],
                message: None,
                group: None,
            })
        }
    }
}

/// Convert an expression to a string value
fn expr_to_string(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Lit(lit) => match &lit.lit {
            Lit::Str(s) => Some(s.value()),
            Lit::Int(i) => Some(i.base10_digits().to_string()),
            Lit::Float(f) => Some(f.base10_digits().to_string()),
            Lit::Bool(b) => Some(b.value.to_string()),
            _ => None,
        },
        _ => None,
    }
}

/// Generate validation code for a single rule
fn generate_rule_validation(
    field_name: &str,
    _field_type: &Type,
    rule: &ValidationRuleInfo,
) -> proc_macro2::TokenStream {
    let field_ident = syn::Ident::new(field_name, proc_macro2::Span::call_site());
    let field_name_str = field_name;

    match rule.rule_type.as_str() {
        "email" => {
            let message = rule
                .message
                .as_ref()
                .map(|m| quote! { .with_message(#m) })
                .unwrap_or_default();
            quote! {
                {
                    let rule = ::rustapi_validate::v2::EmailRule::new() #message;
                    if let Err(e) = ::rustapi_validate::v2::ValidationRule::validate(&rule, &self.#field_ident) {
                        errors.add(#field_name_str, e);
                    }
                }
            }
        }
        "length" => {
            let min = rule
                .params
                .iter()
                .find(|(k, _)| k == "min")
                .and_then(|(_, v)| v.parse::<usize>().ok());
            let max = rule
                .params
                .iter()
                .find(|(k, _)| k == "max")
                .and_then(|(_, v)| v.parse::<usize>().ok());
            let message = rule
                .message
                .as_ref()
                .map(|m| quote! { .with_message(#m) })
                .unwrap_or_default();

            let rule_creation = match (min, max) {
                (Some(min), Some(max)) => {
                    quote! { ::rustapi_validate::v2::LengthRule::new(#min, #max) }
                }
                (Some(min), None) => quote! { ::rustapi_validate::v2::LengthRule::min(#min) },
                (None, Some(max)) => quote! { ::rustapi_validate::v2::LengthRule::max(#max) },
                (None, None) => quote! { ::rustapi_validate::v2::LengthRule::new(0, usize::MAX) },
            };

            quote! {
                {
                    let rule = #rule_creation #message;
                    if let Err(e) = ::rustapi_validate::v2::ValidationRule::validate(&rule, &self.#field_ident) {
                        errors.add(#field_name_str, e);
                    }
                }
            }
        }
        "range" => {
            let min = rule
                .params
                .iter()
                .find(|(k, _)| k == "min")
                .map(|(_, v)| v.clone());
            let max = rule
                .params
                .iter()
                .find(|(k, _)| k == "max")
                .map(|(_, v)| v.clone());
            let message = rule
                .message
                .as_ref()
                .map(|m| quote! { .with_message(#m) })
                .unwrap_or_default();

            // Determine the numeric type from the field type
            let rule_creation = match (min, max) {
                (Some(min), Some(max)) => {
                    let min_lit: proc_macro2::TokenStream = min.parse().unwrap();
                    let max_lit: proc_macro2::TokenStream = max.parse().unwrap();
                    quote! { ::rustapi_validate::v2::RangeRule::new(#min_lit, #max_lit) }
                }
                (Some(min), None) => {
                    let min_lit: proc_macro2::TokenStream = min.parse().unwrap();
                    quote! { ::rustapi_validate::v2::RangeRule::min(#min_lit) }
                }
                (None, Some(max)) => {
                    let max_lit: proc_macro2::TokenStream = max.parse().unwrap();
                    quote! { ::rustapi_validate::v2::RangeRule::max(#max_lit) }
                }
                (None, None) => {
                    return quote! {};
                }
            };

            quote! {
                {
                    let rule = #rule_creation #message;
                    if let Err(e) = ::rustapi_validate::v2::ValidationRule::validate(&rule, &self.#field_ident) {
                        errors.add(#field_name_str, e);
                    }
                }
            }
        }
        "regex" => {
            let pattern = rule
                .params
                .iter()
                .find(|(k, _)| k == "regex" || k == "pattern")
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            let message = rule
                .message
                .as_ref()
                .map(|m| quote! { .with_message(#m) })
                .unwrap_or_default();

            quote! {
                {
                    let rule = ::rustapi_validate::v2::RegexRule::new(#pattern) #message;
                    if let Err(e) = ::rustapi_validate::v2::ValidationRule::validate(&rule, &self.#field_ident) {
                        errors.add(#field_name_str, e);
                    }
                }
            }
        }
        "url" => {
            let message = rule
                .message
                .as_ref()
                .map(|m| quote! { .with_message(#m) })
                .unwrap_or_default();
            quote! {
                {
                    let rule = ::rustapi_validate::v2::UrlRule::new() #message;
                    if let Err(e) = ::rustapi_validate::v2::ValidationRule::validate(&rule, &self.#field_ident) {
                        errors.add(#field_name_str, e);
                    }
                }
            }
        }
        "required" => {
            let message = rule
                .message
                .as_ref()
                .map(|m| quote! { .with_message(#m) })
                .unwrap_or_default();
            quote! {
                {
                    let rule = ::rustapi_validate::v2::RequiredRule::new() #message;
                    if let Err(e) = ::rustapi_validate::v2::ValidationRule::validate(&rule, &self.#field_ident) {
                        errors.add(#field_name_str, e);
                    }
                }
            }
        }
        _ => {
            // Unknown rule - skip
            quote! {}
        }
    }
}

/// Generate async validation code for a single rule
fn generate_async_rule_validation(
    field_name: &str,
    rule: &ValidationRuleInfo,
) -> proc_macro2::TokenStream {
    let field_ident = syn::Ident::new(field_name, proc_macro2::Span::call_site());
    let field_name_str = field_name;

    match rule.rule_type.as_str() {
        "async_unique" => {
            let table = rule
                .params
                .iter()
                .find(|(k, _)| k == "table")
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            let column = rule
                .params
                .iter()
                .find(|(k, _)| k == "column")
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            let message = rule
                .message
                .as_ref()
                .map(|m| quote! { .with_message(#m) })
                .unwrap_or_default();

            quote! {
                {
                    let rule = ::rustapi_validate::v2::AsyncUniqueRule::new(#table, #column) #message;
                    if let Err(e) = ::rustapi_validate::v2::AsyncValidationRule::validate_async(&rule, &self.#field_ident, ctx).await {
                        errors.add(#field_name_str, e);
                    }
                }
            }
        }
        "async_exists" => {
            let table = rule
                .params
                .iter()
                .find(|(k, _)| k == "table")
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            let column = rule
                .params
                .iter()
                .find(|(k, _)| k == "column")
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            let message = rule
                .message
                .as_ref()
                .map(|m| quote! { .with_message(#m) })
                .unwrap_or_default();

            quote! {
                {
                    let rule = ::rustapi_validate::v2::AsyncExistsRule::new(#table, #column) #message;
                    if let Err(e) = ::rustapi_validate::v2::AsyncValidationRule::validate_async(&rule, &self.#field_ident, ctx).await {
                        errors.add(#field_name_str, e);
                    }
                }
            }
        }
        "async_api" => {
            let endpoint = rule
                .params
                .iter()
                .find(|(k, _)| k == "endpoint")
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            let message = rule
                .message
                .as_ref()
                .map(|m| quote! { .with_message(#m) })
                .unwrap_or_default();

            quote! {
                {
                    let rule = ::rustapi_validate::v2::AsyncApiRule::new(#endpoint) #message;
                    if let Err(e) = ::rustapi_validate::v2::AsyncValidationRule::validate_async(&rule, &self.#field_ident, ctx).await {
                        errors.add(#field_name_str, e);
                    }
                }
            }
        }
        _ => {
            // Not an async rule
            quote! {}
        }
    }
}

/// Check if a rule is async
fn is_async_rule(rule: &ValidationRuleInfo) -> bool {
    matches!(
        rule.rule_type.as_str(),
        "async_unique" | "async_exists" | "async_api"
    )
}

/// Derive macro for implementing Validate and AsyncValidate traits
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_macros::Validate;
///
/// #[derive(Validate)]
/// struct CreateUser {
///     #[validate(email, message = "Invalid email format")]
///     email: String,
///     
///     #[validate(length(min = 3, max = 50))]
///     username: String,
///     
///     #[validate(range(min = 18, max = 120))]
///     age: u8,
///     
///     #[validate(async_unique(table = "users", column = "email"))]
///     email: String,
/// }
/// ```
#[proc_macro_derive(Validate, attributes(validate))]
pub fn derive_validate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Only support structs with named fields
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new_spanned(
                    &input,
                    "Validate can only be derived for structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(&input, "Validate can only be derived for structs")
                .to_compile_error()
                .into();
        }
    };

    // Collect sync and async validation code for each field
    let mut sync_validations = Vec::new();
    let mut async_validations = Vec::new();
    let mut has_async_rules = false;

    for field in fields {
        let field_name = field.ident.as_ref().unwrap().to_string();
        let field_type = &field.ty;
        let rules = parse_validate_attrs(&field.attrs);

        for rule in &rules {
            if is_async_rule(rule) {
                has_async_rules = true;
                let validation = generate_async_rule_validation(&field_name, rule);
                async_validations.push(validation);
            } else {
                let validation = generate_rule_validation(&field_name, field_type, rule);
                sync_validations.push(validation);
            }
        }
    }

    // Generate the Validate impl
    let validate_impl = quote! {
        impl #impl_generics ::rustapi_validate::v2::Validate for #name #ty_generics #where_clause {
            fn validate(&self) -> Result<(), ::rustapi_validate::v2::ValidationErrors> {
                let mut errors = ::rustapi_validate::v2::ValidationErrors::new();

                #(#sync_validations)*

                errors.into_result()
            }
        }
    };

    // Generate the AsyncValidate impl if there are async rules
    let async_validate_impl = if has_async_rules {
        quote! {
            #[::async_trait::async_trait]
            impl #impl_generics ::rustapi_validate::v2::AsyncValidate for #name #ty_generics #where_clause {
                async fn validate_async(&self, ctx: &::rustapi_validate::v2::ValidationContext) -> Result<(), ::rustapi_validate::v2::ValidationErrors> {
                    let mut errors = ::rustapi_validate::v2::ValidationErrors::new();

                    #(#async_validations)*

                    errors.into_result()
                }
            }
        }
    } else {
        // Provide a default AsyncValidate impl that just returns Ok
        quote! {
            #[::async_trait::async_trait]
            impl #impl_generics ::rustapi_validate::v2::AsyncValidate for #name #ty_generics #where_clause {
                async fn validate_async(&self, _ctx: &::rustapi_validate::v2::ValidationContext) -> Result<(), ::rustapi_validate::v2::ValidationErrors> {
                    Ok(())
                }
            }
        }
    };

    let expanded = quote! {
        #validate_impl
        #async_validate_impl
    };

    debug_output("Validate derive", &expanded);

    TokenStream::from(expanded)
}

// ============================================
// ApiError Derive Macro
// ============================================

/// Parsed error attribute info

/// Derive macro for implementing IntoResponse for error enums
///
/// # Example
///
/// ```rust,ignore
/// #[derive(ApiError)]
/// enum UserError {
///     #[error(status = 404, message = "User not found")]
///     NotFound(i64),
///     
///     #[error(status = 400, code = "validation_error")]
///     InvalidInput(String),
/// }
/// ```
#[proc_macro_derive(ApiError, attributes(error))]
pub fn derive_api_error(input: TokenStream) -> TokenStream {
    api_error::expand_derive_api_error(input)
}

// ============================================
// TypedPath Derive Macro
// ============================================

/// Derive macro for TypedPath
///
/// # Example
///
/// ```rust,ignore
/// #[derive(TypedPath, Deserialize, Serialize)]
/// #[typed_path("/users/{id}/posts/{post_id}")]
/// struct PostPath {
///     id: u64,
///     post_id: String,
/// }
/// ```
#[proc_macro_derive(TypedPath, attributes(typed_path))]
pub fn derive_typed_path(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Find the #[typed_path("...")] attribute
    let mut path_str = None;
    for attr in &input.attrs {
        if attr.path().is_ident("typed_path") {
            if let Ok(lit) = attr.parse_args::<LitStr>() {
                path_str = Some(lit.value());
            }
        }
    }

    let path = match path_str {
        Some(p) => p,
        None => {
            return syn::Error::new_spanned(
                &input,
                "#[derive(TypedPath)] requires a #[typed_path(\"...\")] attribute",
            )
            .to_compile_error()
            .into();
        }
    };

    // Validate path syntax
    if let Err(err) = validate_path_syntax(&path, proc_macro2::Span::call_site()) {
        return err.to_compile_error().into();
    }

    // Generate to_uri implementation
    // We need to parse the path and replace {param} with self.param
    let mut format_string = String::new();
    let mut format_args = Vec::new();

    let mut chars = path.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '{' {
            let mut param_name = String::new();
            while let Some(&c) = chars.peek() {
                if c == '}' {
                    chars.next(); // Consume '}'
                    break;
                }
                param_name.push(chars.next().unwrap());
            }

            if param_name.is_empty() {
                return syn::Error::new_spanned(
                    &input,
                    "Empty path parameter not allowed in typed_path",
                )
                .to_compile_error()
                .into();
            }

            format_string.push_str("{}");
            let ident = syn::Ident::new(&param_name, proc_macro2::Span::call_site());
            format_args.push(quote! { self.#ident });
        } else {
            format_string.push(ch);
        }
    }

    let expanded = quote! {
        impl #impl_generics ::rustapi_rs::prelude::TypedPath for #name #ty_generics #where_clause {
            const PATH: &'static str = #path;

            fn to_uri(&self) -> String {
                format!(#format_string, #(#format_args),*)
            }
        }
    };

    debug_output("TypedPath derive", &expanded);
    TokenStream::from(expanded)
}
