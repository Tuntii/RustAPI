//! Procedural macros for RustAPI
//!
//! This crate provides the attribute macros used in RustAPI:
//!
//! - `#[rustapi::main]` - Main entry point macro
//! - `#[rustapi::get("/path")]` - GET route handler
//! - `#[rustapi::post("/path")]` - POST route handler
//! - `#[rustapi::put("/path")]` - PUT route handler
//! - `#[rustapi::patch("/path")]` - PATCH route handler
//! - `#[rustapi::delete("/path")]` - DELETE route handler
//!
//! ## Debugging
//!
//! Set `RUSTAPI_DEBUG=1` environment variable during compilation to see
//! expanded macro output for debugging purposes.

use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::{
    parse_macro_input, FnArg, GenericArgument, ItemFn, LitStr, PathArguments, ReturnType, Type,
};

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
