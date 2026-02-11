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
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use std::collections::HashSet;
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Expr, Fields, FnArg, GenericArgument, ItemFn,
    Lit, LitStr, Meta, PathArguments, ReturnType, Type,
};

mod api_error;
mod derive_schema;

/// Determine the path to the RustAPI facade crate (`rustapi-rs`).
///
/// This supports dependency renaming, for example:
/// `api = { package = "rustapi-rs", version = "..." }`.
fn get_rustapi_path() -> proc_macro2::TokenStream {
    let rustapi_rs_found = crate_name("rustapi-rs").or_else(|_| crate_name("rustapi_rs"));

    if let Ok(found) = rustapi_rs_found {
        match found {
            // `FoundCrate::Itself` can occur for examples/benches inside the rustapi-rs package.
            // Use an absolute crate path so generated code also works in those targets.
            FoundCrate::Itself => quote! { ::rustapi_rs },
            FoundCrate::Name(name) => {
                let normalized = name.replace('-', "_");
                let ident = syn::Ident::new(&normalized, proc_macro2::Span::call_site());
                quote! { ::#ident }
            }
        }
    } else {
        quote! { ::rustapi_rs }
    }
}

/// Derive macro for OpenAPI Schema trait
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Schema)]
/// struct User {
///     id: i64,
///     name: String,
/// }
/// ```
#[proc_macro_derive(Schema, attributes(schema))]
pub fn derive_schema(input: TokenStream) -> TokenStream {
    derive_schema::expand_derive_schema(parse_macro_input!(input as DeriveInput)).into()
}

/// Auto-register a schema type for zero-config OpenAPI.
///
/// Attach this to a `struct` or `enum` that also derives `Schema`.
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
    let rustapi_path = get_rustapi_path();

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
        #[#rustapi_path::__private::linkme::distributed_slice(#rustapi_path::__private::AUTO_SCHEMAS)]
        #[linkme(crate = #rustapi_path::__private::linkme)]
        static #registrar_ident: fn(&mut #rustapi_path::__private::rustapi_openapi::OpenApiSpec) =
            |spec: &mut #rustapi_path::__private::rustapi_openapi::OpenApiSpec| {
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

/// Collect path parameters and their inferred types from function arguments
///
/// Returns a list of (name, schema_type) tuples.
fn collect_path_params(input: &ItemFn) -> Vec<(String, String)> {
    let mut params = Vec::new();

    for arg in &input.sig.inputs {
        if let FnArg::Typed(pat_ty) = arg {
            // Check if the argument is a Path extractor
            if let Type::Path(tp) = &*pat_ty.ty {
                if let Some(seg) = tp.path.segments.last() {
                    if seg.ident == "Path" {
                        // Extract the inner type T from Path<T>
                        if let PathArguments::AngleBracketed(args) = &seg.arguments {
                            if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                                // Map inner type to schema string
                                if let Some(schema_type) = map_type_to_schema(inner_ty) {
                                    // Extract the parameter name
                                    // We handle the pattern `Path(name)` or `name: Path<T>`
                                    // For `Path(id): Path<Uuid>`, the variable binding is inside the tuple struct pattern?
                                    // No, wait. `Path(id): Path<Uuid>` is NOT valid Rust syntax for function arguments!
                                    // Extractor destructuring uses `Path(id)` as the PATTERN.
                                    // e.g. `fn handler(Path(id): Path<Uuid>)`

                                    if let Some(name) = extract_param_name(&pat_ty.pat) {
                                        params.push((name, schema_type));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    params
}

/// Extract parameter name from pattern
///
/// Handles `Path(id)` -> "id"
/// Handles `id` -> "id" (if simple binding)
fn extract_param_name(pat: &syn::Pat) -> Option<String> {
    match pat {
        syn::Pat::Ident(ident) => Some(ident.ident.to_string()),
        syn::Pat::TupleStruct(ts) => {
            // Handle Path(id) destructuring
            // We assume the first field is the parameter we want if it's a simple identifier
            if let Some(first) = ts.elems.first() {
                extract_param_name(first)
            } else {
                None
            }
        }
        _ => None, // Complex patterns not supported for auto-detection yet
    }
}

/// Map Rust type to OpenAPI schema type string
fn map_type_to_schema(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(tp) => {
            if let Some(seg) = tp.path.segments.last() {
                let ident = seg.ident.to_string();
                match ident.as_str() {
                    "Uuid" => Some("uuid".to_string()),
                    "String" | "str" => Some("string".to_string()),
                    "bool" => Some("boolean".to_string()),
                    "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64"
                    | "usize" => Some("integer".to_string()),
                    "f32" | "f64" => Some("number".to_string()),
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
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
    let rustapi_path = get_rustapi_path();

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
        "GET" => quote!(#rustapi_path::get_route),
        "POST" => quote!(#rustapi_path::post_route),
        "PUT" => quote!(#rustapi_path::put_route),
        "PATCH" => quote!(#rustapi_path::patch_route),
        "DELETE" => quote!(#rustapi_path::delete_route),
        _ => quote!(#rustapi_path::get_route),
    };

    // Auto-detect path parameters from function arguments
    let auto_params = collect_path_params(&input);

    // Extract metadata from attributes to chain builder methods
    let mut chained_calls = quote!();

    // Add auto-detected parameters first (can be overridden by attributes)
    for (name, schema) in auto_params {
        chained_calls = quote! { #chained_calls .param(#name, #schema) };
    }

    for attr in fn_attrs {
        // Check for tag, summary, description, param
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
            } else if ident_str == "param" {
                // Parse #[param(name, schema = "type")] or #[param(name = "type")]
                if let Ok(param_args) = attr.parse_args_with(
                    syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
                ) {
                    let mut param_name: Option<String> = None;
                    let mut param_schema: Option<String> = None;

                    for meta in param_args {
                        match &meta {
                            // Simple ident: #[param(id, ...)]
                            Meta::Path(path) => {
                                if param_name.is_none() {
                                    if let Some(ident) = path.get_ident() {
                                        param_name = Some(ident.to_string());
                                    }
                                }
                            }
                            // Named value: #[param(schema = "uuid")] or #[param(id = "uuid")]
                            Meta::NameValue(nv) => {
                                let key = nv.path.get_ident().map(|i| i.to_string());
                                if let Some(key) = key {
                                    if key == "schema" || key == "type" {
                                        if let Expr::Lit(lit) = &nv.value {
                                            if let Lit::Str(s) = &lit.lit {
                                                param_schema = Some(s.value());
                                            }
                                        }
                                    } else if param_name.is_none() {
                                        // Treat as #[param(name = "schema")]
                                        param_name = Some(key);
                                        if let Expr::Lit(lit) = &nv.value {
                                            if let Lit::Str(s) = &lit.lit {
                                                param_schema = Some(s.value());
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    if let (Some(pname), Some(pschema)) = (param_name, param_schema) {
                        chained_calls = quote! { #chained_calls .param(#pname, #pschema) };
                    }
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
        #fn_vis fn #route_fn_name() -> #rustapi_path::Route {
            #route_helper(#path_value, #fn_name)
                #chained_calls
        }

        // Auto-register route with linkme
        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        #[#rustapi_path::__private::linkme::distributed_slice(#rustapi_path::__private::AUTO_ROUTES)]
        #[linkme(crate = #rustapi_path::__private::linkme)]
        static #auto_route_name: fn() -> #rustapi_path::Route = #route_fn_name;

        // Auto-register referenced schemas with linkme (best-effort)
        #[doc(hidden)]
        #[allow(non_snake_case)]
        fn #schema_reg_fn_name(spec: &mut #rustapi_path::__private::rustapi_openapi::OpenApiSpec) {
            #( spec.register_in_place::<#schema_types>(); )*
        }

        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        #[#rustapi_path::__private::linkme::distributed_slice(#rustapi_path::__private::AUTO_SCHEMAS)]
        #[linkme(crate = #rustapi_path::__private::linkme)]
        static #auto_schema_name: fn(&mut #rustapi_path::__private::rustapi_openapi::OpenApiSpec) = #schema_reg_fn_name;
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

/// Path parameter schema macro for OpenAPI documentation
///
/// Use this to specify the OpenAPI schema type for a path parameter when
/// the auto-inferred type is incorrect. This is particularly useful for
/// UUID parameters that might be named `id`.
///
/// # Supported schema types
/// - `"uuid"` - String with UUID format
/// - `"integer"` or `"int"` - Integer with int64 format
/// - `"string"` - Plain string
/// - `"boolean"` or `"bool"` - Boolean
/// - `"number"` - Number (float)
///
/// # Example
///
/// ```rust,ignore
/// use uuid::Uuid;
///
/// #[rustapi::get("/users/{id}")]
/// #[rustapi::param(id, schema = "uuid")]
/// async fn get_user(Path(id): Path<Uuid>) -> Json<User> {
///     // ...
/// }
///
/// // Alternative syntax:
/// #[rustapi::get("/posts/{post_id}")]
/// #[rustapi::param(post_id = "uuid")]
/// async fn get_post(Path(post_id): Path<Uuid>) -> Json<Post> {
///     // ...
/// }
/// ```
#[proc_macro_attribute]
pub fn param(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // The param attribute is processed by the route macro (get, post, etc.)
    // This macro just passes through the function unchanged
    item
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
    groups: Vec<String>,
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
                groups: Vec::new(),
            })
        }
        Meta::List(list) => {
            // Rule with params like #[validate(length(min = 3, max = 50))]
            let rule_type = list.path.get_ident()?.to_string();
            let mut params = Vec::new();
            let mut message = None;
            let mut groups = Vec::new();

            // Parse nested params
            if let Ok(nested) = list.parse_args_with(
                syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
            ) {
                for nested_meta in nested {
                    if let Meta::NameValue(nv) = &nested_meta {
                        let key = nv.path.get_ident()?.to_string();

                        if key == "groups" {
                            let vec = expr_to_string_vec(&nv.value);
                            groups.extend(vec);
                        } else if let Some(value) = expr_to_string(&nv.value) {
                            if key == "message" {
                                message = Some(value);
                            } else if key == "group" {
                                groups.push(value);
                            } else {
                                params.push((key, value));
                            }
                        }
                    } else if let Meta::Path(path) = &nested_meta {
                        // Handle flags like #[validate(ip(v4))]
                        if let Some(ident) = path.get_ident() {
                            params.push((ident.to_string(), "true".to_string()));
                        }
                    }
                }
            }

            Some(ValidationRuleInfo {
                rule_type,
                params,
                message,
                groups,
            })
        }
        Meta::NameValue(nv) => {
            // Rule like #[validate(regex = "pattern")]
            let rule_type = nv.path.get_ident()?.to_string();
            let value = expr_to_string(&nv.value)?;

            Some(ValidationRuleInfo {
                rule_type: rule_type.clone(),
                params: vec![(rule_type.clone(), value)],
                message: None,
                groups: Vec::new(),
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

/// Convert an expression to a vector of strings
fn expr_to_string_vec(expr: &Expr) -> Vec<String> {
    match expr {
        Expr::Array(arr) => {
            let mut result = Vec::new();
            for elem in &arr.elems {
                if let Some(s) = expr_to_string(elem) {
                    result.push(s);
                }
            }
            result
        }
        _ => {
            if let Some(s) = expr_to_string(expr) {
                vec![s]
            } else {
                Vec::new()
            }
        }
    }
}

/// Determine the path to rustapi_validate based on the user's dependencies.
///
/// Checks for (in order):
/// 1. `rustapi-rs` → `::rustapi_rs::__private::rustapi_validate`
/// 2. `rustapi-validate` → `::rustapi_validate`
///
/// This allows the Validate derive macro to work in both user projects
/// (which depend on rustapi-rs) and internal crates (which depend on
/// rustapi-validate directly).
fn get_validate_path() -> proc_macro2::TokenStream {
    let rustapi_rs_found = crate_name("rustapi-rs").or_else(|_| crate_name("rustapi_rs"));

    if let Ok(found) = rustapi_rs_found {
        match found {
            FoundCrate::Itself => {
                quote! { crate::__private::rustapi_validate }
            }
            FoundCrate::Name(name) => {
                let normalized = name.replace('-', "_");
                let ident = syn::Ident::new(&normalized, proc_macro2::Span::call_site());
                quote! { ::#ident::__private::rustapi_validate }
            }
        }
    } else if let Ok(found) =
        crate_name("rustapi-validate").or_else(|_| crate_name("rustapi_validate"))
    {
        match found {
            FoundCrate::Itself => quote! { crate },
            FoundCrate::Name(name) => {
                let normalized = name.replace('-', "_");
                let ident = syn::Ident::new(&normalized, proc_macro2::Span::call_site());
                quote! { ::#ident }
            }
        }
    } else {
        // Default fallback
        quote! { ::rustapi_validate }
    }
}

/// Determine the path to rustapi_core based on the user's dependencies.
///
/// Checks for (in order):
/// 1. `rustapi-rs` (which re-exports rustapi-core via glob)
/// 2. `rustapi-core` directly
fn get_core_path() -> proc_macro2::TokenStream {
    let rustapi_rs_found = crate_name("rustapi-rs").or_else(|_| crate_name("rustapi_rs"));

    if let Ok(found) = rustapi_rs_found {
        match found {
            FoundCrate::Itself => quote! { crate },
            FoundCrate::Name(name) => {
                let normalized = name.replace('-', "_");
                let ident = syn::Ident::new(&normalized, proc_macro2::Span::call_site());
                quote! { ::#ident }
            }
        }
    } else if let Ok(found) = crate_name("rustapi-core").or_else(|_| crate_name("rustapi_core")) {
        match found {
            FoundCrate::Itself => quote! { crate },
            FoundCrate::Name(name) => {
                let normalized = name.replace('-', "_");
                let ident = syn::Ident::new(&normalized, proc_macro2::Span::call_site());
                quote! { ::#ident }
            }
        }
    } else {
        quote! { ::rustapi_core }
    }
}

/// Determine the path to async_trait based on the user's dependencies.
///
/// Checks for (in order):
/// 1. `rustapi-rs` → `::rustapi_rs::__private::async_trait`
/// 2. `async-trait` directly
fn get_async_trait_path() -> proc_macro2::TokenStream {
    let rustapi_rs_found = crate_name("rustapi-rs").or_else(|_| crate_name("rustapi_rs"));

    if let Ok(found) = rustapi_rs_found {
        match found {
            FoundCrate::Itself => {
                quote! { crate::__private::async_trait }
            }
            FoundCrate::Name(name) => {
                let normalized = name.replace('-', "_");
                let ident = syn::Ident::new(&normalized, proc_macro2::Span::call_site());
                quote! { ::#ident::__private::async_trait }
            }
        }
    } else if let Ok(found) = crate_name("async-trait").or_else(|_| crate_name("async_trait")) {
        match found {
            FoundCrate::Itself => quote! { crate },
            FoundCrate::Name(name) => {
                let normalized = name.replace('-', "_");
                let ident = syn::Ident::new(&normalized, proc_macro2::Span::call_site());
                quote! { ::#ident }
            }
        }
    } else {
        quote! { ::async_trait }
    }
}

fn generate_rule_validation(
    field_name: &str,
    _field_type: &Type,
    rule: &ValidationRuleInfo,
    validate_path: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let field_ident = syn::Ident::new(field_name, proc_macro2::Span::call_site());
    let field_name_str = field_name;

    // Generate group check
    let group_check = if rule.groups.is_empty() {
        quote! { true }
    } else {
        let group_names = rule.groups.iter().map(|g| g.as_str());
        quote! {
            {
                let rule_groups = [#(#validate_path::v2::ValidationGroup::from(#group_names)),*];
                rule_groups.iter().any(|g| g.matches(&group))
            }
        }
    };

    let validation_logic = match rule.rule_type.as_str() {
        "email" => {
            let message = rule
                .message
                .as_ref()
                .map(|m| quote! { .with_message(#m) })
                .unwrap_or_default();
            quote! {
                {
                    let rule = #validate_path::v2::EmailRule::new() #message;
                    if let Err(e) = #validate_path::v2::ValidationRule::validate(&rule, &self.#field_ident) {
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
                    quote! { #validate_path::v2::LengthRule::new(#min, #max) }
                }
                (Some(min), None) => quote! { #validate_path::v2::LengthRule::min(#min) },
                (None, Some(max)) => quote! { #validate_path::v2::LengthRule::max(#max) },
                (None, None) => quote! { #validate_path::v2::LengthRule::new(0, usize::MAX) },
            };

            quote! {
                {
                    let rule = #rule_creation #message;
                    if let Err(e) = #validate_path::v2::ValidationRule::validate(&rule, &self.#field_ident) {
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
                    quote! { #validate_path::v2::RangeRule::new(#min_lit, #max_lit) }
                }
                (Some(min), None) => {
                    let min_lit: proc_macro2::TokenStream = min.parse().unwrap();
                    quote! { #validate_path::v2::RangeRule::min(#min_lit) }
                }
                (None, Some(max)) => {
                    let max_lit: proc_macro2::TokenStream = max.parse().unwrap();
                    quote! { #validate_path::v2::RangeRule::max(#max_lit) }
                }
                (None, None) => {
                    return quote! {};
                }
            };

            quote! {
                {
                    let rule = #rule_creation #message;
                    if let Err(e) = #validate_path::v2::ValidationRule::validate(&rule, &self.#field_ident) {
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
                    let rule = #validate_path::v2::RegexRule::new(#pattern) #message;
                    if let Err(e) = #validate_path::v2::ValidationRule::validate(&rule, &self.#field_ident) {
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
                    let rule = #validate_path::v2::UrlRule::new() #message;
                    if let Err(e) = #validate_path::v2::ValidationRule::validate(&rule, &self.#field_ident) {
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
                    let rule = #validate_path::v2::RequiredRule::new() #message;
                    if let Err(e) = #validate_path::v2::ValidationRule::validate(&rule, &self.#field_ident) {
                        errors.add(#field_name_str, e);
                    }
                }
            }
        }
        "credit_card" => {
            let message = rule
                .message
                .as_ref()
                .map(|m| quote! { .with_message(#m) })
                .unwrap_or_default();
            quote! {
                {
                    let rule = #validate_path::v2::CreditCardRule::new() #message;
                    if let Err(e) = #validate_path::v2::ValidationRule::validate(&rule, &self.#field_ident) {
                        errors.add(#field_name_str, e);
                    }
                }
            }
        }
        "ip" => {
            let v4 = rule.params.iter().any(|(k, _)| k == "v4");
            let v6 = rule.params.iter().any(|(k, _)| k == "v6");

            let rule_creation = if v4 && !v6 {
                quote! { #validate_path::v2::IpRule::v4() }
            } else if !v4 && v6 {
                quote! { #validate_path::v2::IpRule::v6() }
            } else {
                quote! { #validate_path::v2::IpRule::new() }
            };

            let message = rule
                .message
                .as_ref()
                .map(|m| quote! { .with_message(#m) })
                .unwrap_or_default();

            quote! {
                {
                    let rule = #rule_creation #message;
                    if let Err(e) = #validate_path::v2::ValidationRule::validate(&rule, &self.#field_ident) {
                        errors.add(#field_name_str, e);
                    }
                }
            }
        }
        "phone" => {
            let message = rule
                .message
                .as_ref()
                .map(|m| quote! { .with_message(#m) })
                .unwrap_or_default();
            quote! {
                {
                    let rule = #validate_path::v2::PhoneRule::new() #message;
                    if let Err(e) = #validate_path::v2::ValidationRule::validate(&rule, &self.#field_ident) {
                        errors.add(#field_name_str, e);
                    }
                }
            }
        }
        "contains" => {
            let needle = rule
                .params
                .iter()
                .find(|(k, _)| k == "needle")
                .map(|(_, v)| v.clone())
                .unwrap_or_default();

            let message = rule
                .message
                .as_ref()
                .map(|m| quote! { .with_message(#m) })
                .unwrap_or_default();

            quote! {
                {
                    let rule = #validate_path::v2::ContainsRule::new(#needle) #message;
                    if let Err(e) = #validate_path::v2::ValidationRule::validate(&rule, &self.#field_ident) {
                        errors.add(#field_name_str, e);
                    }
                }
            }
        }
        _ => {
            // Unknown rule - skip
            quote! {}
        }
    };

    quote! {
        if #group_check {
            #validation_logic
        }
    }
}

/// Generate async validation code for a single rule
fn generate_async_rule_validation(
    field_name: &str,
    rule: &ValidationRuleInfo,
    validate_path: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let field_ident = syn::Ident::new(field_name, proc_macro2::Span::call_site());
    let field_name_str = field_name;

    // Generate group check
    let group_check = if rule.groups.is_empty() {
        quote! { true }
    } else {
        let group_names = rule.groups.iter().map(|g| g.as_str());
        quote! {
            {
                let rule_groups = [#(#validate_path::v2::ValidationGroup::from(#group_names)),*];
                rule_groups.iter().any(|g| g.matches(&group))
            }
        }
    };

    let validation_logic = match rule.rule_type.as_str() {
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
                    let rule = #validate_path::v2::AsyncUniqueRule::new(#table, #column) #message;
                    if let Err(e) = #validate_path::v2::AsyncValidationRule::validate_async(&rule, &self.#field_ident, ctx).await {
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
                    let rule = #validate_path::v2::AsyncExistsRule::new(#table, #column) #message;
                    if let Err(e) = #validate_path::v2::AsyncValidationRule::validate_async(&rule, &self.#field_ident, ctx).await {
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
                    let rule = #validate_path::v2::AsyncApiRule::new(#endpoint) #message;
                    if let Err(e) = #validate_path::v2::AsyncValidationRule::validate_async(&rule, &self.#field_ident, ctx).await {
                        errors.add(#field_name_str, e);
                    }
                }
            }
        }
        "custom_async" => {
            // #[validate(custom_async = "function_path")]
            let function_path = rule
                .params
                .iter()
                .find(|(k, _)| k == "custom_async" || k == "function")
                .map(|(_, v)| v.clone())
                .unwrap_or_default();

            if function_path.is_empty() {
                // If path is missing, don't generate invalid code
                quote! {}
            } else {
                let func: syn::Path = syn::parse_str(&function_path).unwrap();
                let message_handling = if let Some(msg) = &rule.message {
                    quote! {
                        let e = #validate_path::v2::RuleError::new("custom_async", #msg);
                        errors.add(#field_name_str, e);
                    }
                } else {
                    quote! {
                        errors.add(#field_name_str, e);
                    }
                };

                quote! {
                    {
                        // Call the custom async function: async fn(&T, &ValidationContext) -> Result<(), RuleError>
                        if let Err(e) = #func(&self.#field_ident, ctx).await {
                            #message_handling
                        }
                    }
                }
            }
        }
        _ => {
            // Not an async rule
            quote! {}
        }
    };

    quote! {
        if #group_check {
            #validation_logic
        }
    }
}

/// Check if a rule is async
fn is_async_rule(rule: &ValidationRuleInfo) -> bool {
    matches!(
        rule.rule_type.as_str(),
        "async_unique" | "async_exists" | "async_api" | "custom_async"
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

    // Resolve crate paths dynamically based on the caller's dependencies
    let validate_path = get_validate_path();
    let core_path = get_core_path();
    let async_trait_path = get_async_trait_path();

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
                let validation = generate_async_rule_validation(&field_name, rule, &validate_path);
                async_validations.push(validation);
            } else {
                let validation =
                    generate_rule_validation(&field_name, field_type, rule, &validate_path);
                sync_validations.push(validation);
            }
        }
    }

    // Generate the Validate impl
    let validate_impl = quote! {
        impl #impl_generics #validate_path::v2::Validate for #name #ty_generics #where_clause {
            fn validate_with_group(&self, group: #validate_path::v2::ValidationGroup) -> Result<(), #validate_path::v2::ValidationErrors> {
                let mut errors = #validate_path::v2::ValidationErrors::new();

                #(#sync_validations)*

                errors.into_result()
            }
        }
    };

    // Generate the AsyncValidate impl if there are async rules
    let async_validate_impl = if has_async_rules {
        quote! {
            #[#async_trait_path::async_trait]
            impl #impl_generics #validate_path::v2::AsyncValidate for #name #ty_generics #where_clause {
                async fn validate_async_with_group(&self, ctx: &#validate_path::v2::ValidationContext, group: #validate_path::v2::ValidationGroup) -> Result<(), #validate_path::v2::ValidationErrors> {
                    let mut errors = #validate_path::v2::ValidationErrors::new();

                    #(#async_validations)*

                    errors.into_result()
                }
            }
        }
    } else {
        // Provide a default AsyncValidate impl that just returns Ok
        quote! {
            #[#async_trait_path::async_trait]
            impl #impl_generics #validate_path::v2::AsyncValidate for #name #ty_generics #where_clause {
                async fn validate_async_with_group(&self, _ctx: &#validate_path::v2::ValidationContext, _group: #validate_path::v2::ValidationGroup) -> Result<(), #validate_path::v2::ValidationErrors> {
                    Ok(())
                }
            }
        }
    };

    // Generate the Validatable impl for rustapi-core integration (exposed via rustapi-rs)
    // Paths are resolved dynamically so this works from both rustapi-rs and internal crates.
    let validatable_impl = quote! {
        impl #impl_generics #core_path::validation::Validatable for #name #ty_generics #where_clause {
            fn do_validate(&self) -> Result<(), #core_path::ApiError> {
                match #validate_path::v2::Validate::validate(self) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(#core_path::validation::convert_v2_errors(e)),
                }
            }
        }
    };

    let expanded = quote! {
        #validate_impl
        #async_validate_impl
        #validatable_impl
    };

    debug_output("Validate derive", &expanded);

    TokenStream::from(expanded)
}

// ============================================
// ApiError Derive Macro
// ============================================

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
    let rustapi_path = get_rustapi_path();
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
        impl #impl_generics #rustapi_path::prelude::TypedPath for #name #ty_generics #where_clause {
            const PATH: &'static str = #path;

            fn to_uri(&self) -> String {
                format!(#format_string, #(#format_args),*)
            }
        }
    };

    debug_output("TypedPath derive", &expanded);
    TokenStream::from(expanded)
}
