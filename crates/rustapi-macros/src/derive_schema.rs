use proc_macro2::TokenStream;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::{Data, DataEnum, DataStruct, Fields, Ident};

/// Determine the path to rustapi_openapi module based on the user's dependencies.
/// 
/// This function checks if the user's Cargo.toml has:
/// 1. `rustapi-rs` - use `::rustapi_rs::prelude::rustapi_openapi`
/// 2. `rustapi-openapi` - use `::rustapi_openapi` directly
/// 
/// This allows the Schema derive macro to work in both:
/// - Internal crates (like rustapi-openapi itself)
/// - User projects that depend on rustapi-rs
fn get_openapi_path() -> TokenStream {
    // First try rustapi-rs (the umbrella crate most users will have)
    if let Ok(found) = crate_name("rustapi-rs") {
        match found {
            FoundCrate::Itself => {
                // We're in rustapi-rs itself
                quote! { ::rustapi_rs::prelude::rustapi_openapi }
            }
            FoundCrate::Name(name) => {
                let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
                quote! { ::#ident::prelude::rustapi_openapi }
            }
        }
    } else if let Ok(found) = crate_name("rustapi-openapi") {
        // Fallback to rustapi-openapi directly
        match found {
            FoundCrate::Itself => {
                // We're inside rustapi-openapi itself, use crate::
                quote! { crate }
            }
            FoundCrate::Name(name) => {
                let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
                quote! { ::#ident }
            }
        }
    } else {
        // Default fallback - assume rustapi_rs is available
        quote! { ::rustapi_rs::prelude::rustapi_openapi }
    }
}

/// Get serde_json path - either from rustapi_rs::prelude or directly
fn get_serde_json_path() -> TokenStream {
    // First try rustapi-rs (the umbrella crate most users will have)
    if let Ok(found) = crate_name("rustapi-rs") {
        match found {
            FoundCrate::Itself => {
                quote! { ::rustapi_rs::prelude::serde_json }
            }
            FoundCrate::Name(name) => {
                let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
                quote! { ::#ident::prelude::serde_json }
            }
        }
    } else {
        // Fallback to serde_json directly (internal crates should have it)
        quote! { ::serde_json }
    }
}

pub fn expand_derive_schema(input: syn::DeriveInput) -> TokenStream {
    let name = input.ident;
    let generics = input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let name_str = name.to_string();

    // Get the correct paths based on available crates
    let openapi_path = get_openapi_path();
    let serde_json_path = get_serde_json_path();

    // Generate name() impl body
    let type_params: Vec<Ident> = generics.type_params().map(|p| p.ident.clone()).collect();
    let name_impl_body = if type_params.is_empty() {
        quote! { std::borrow::Cow::Borrowed(#name_str) }
    } else {
        quote! {
            let mut n = String::from(#name_str);
            #(
                n.push('_');
                n.push_str(&<#type_params as #openapi_path::schema::RustApiSchema>::name());
            )*
            std::borrow::Cow::Owned(n)
        }
    };

    let (schema_impl, field_schemas_impl) = match input.data {
        Data::Struct(data) => impl_struct_schema_bodies(&openapi_path, &serde_json_path, data),
        Data::Enum(data) => (impl_enum_schema(&openapi_path, &serde_json_path, data), quote! { None }),
        Data::Union(_) => {
            return syn::Error::new_spanned(name, "Unions not supported").to_compile_error();
        }
    };

    quote! {
        impl #impl_generics #openapi_path::schema::RustApiSchema for #name #ty_generics #where_clause {
            fn schema(ctx: &mut #openapi_path::schema::SchemaCtx) -> #openapi_path::schema::SchemaRef {
                #schema_impl
            }

            fn component_name() -> Option<&'static str> {
                // Keep backward compatibility, but this is less useful for generics now
                Some(stringify!(#name))
            }

            fn name() -> std::borrow::Cow<'static, str> {
                #name_impl_body
            }

            fn field_schemas(ctx: &mut #openapi_path::schema::SchemaCtx) -> Option<::std::collections::BTreeMap<String, #openapi_path::schema::SchemaRef>> {
                #field_schemas_impl
            }
        }
    }
}

fn impl_struct_schema_bodies(openapi_path: &TokenStream, serde_json_path: &TokenStream, data: DataStruct) -> (TokenStream, TokenStream) {
    let mut field_logic = Vec::new();
    let mut field_schemas_logic = Vec::new();

    match data.fields {
        Fields::Named(fields) => {
            for field in fields.named {
                let field_name = field.ident.unwrap();
                let field_name_str = field_name.to_string();
                let field_type = field.ty;

                let is_option = if let syn::Type::Path(tp) = &field_type {
                    tp.path
                        .segments
                        .last()
                        .map(|s| s.ident == "Option")
                        .unwrap_or(false)
                } else {
                    false
                };

                let required_push = if !is_option {
                    quote! { required.push(#field_name_str.to_string()); }
                } else {
                    quote! {}
                };

                field_logic.push(quote! {
                    let field_schema_ref = <#field_type as #openapi_path::schema::RustApiSchema>::schema(ctx);
                    let field_schema = match field_schema_ref {
                        #openapi_path::schema::SchemaRef::Schema(s) => *s,
                        #openapi_path::schema::SchemaRef::Ref { reference } => {
                            let mut s = #openapi_path::schema::JsonSchema2020::new();
                            s.reference = Some(reference);
                            s
                        },
                        #openapi_path::schema::SchemaRef::Inline(v) => {
                            #serde_json_path::from_value(v).unwrap_or_default()
                        }
                    };
                    properties.insert(#field_name_str.to_string(), field_schema);
                    #required_push
                });

                field_schemas_logic.push(quote! {
                    let field_schema_ref = <#field_type as #openapi_path::schema::RustApiSchema>::schema(ctx);
                    map.insert(#field_name_str.to_string(), field_schema_ref);
                });
            }
        }
        _ => { /* Unnamed/Unit structs skipped for field_schemas */ }
    }

    let schema_body = quote! {
        let name_cow = <Self as #openapi_path::schema::RustApiSchema>::name();
        let name = name_cow.as_ref();

        if let Some(_) = ctx.components.get(name) {
            return #openapi_path::schema::SchemaRef::Ref { reference: format!("#/components/schemas/{}", name) };
        }

        ctx.components.insert(name.to_string(), #openapi_path::schema::JsonSchema2020::new());

        let mut properties = ::std::collections::BTreeMap::new();
        let mut required = Vec::new();

        #(#field_logic)*

        let mut schema = #openapi_path::schema::JsonSchema2020::object();
        schema.properties = Some(properties);
        if !required.is_empty() {
            schema.required = Some(required);
        }

        ctx.components.insert(name.to_string(), schema);

        #openapi_path::schema::SchemaRef::Ref { reference: format!("#/components/schemas/{}", name) }
    };

    let field_schemas_body = if !field_schemas_logic.is_empty() {
        quote! {
            let mut map = ::std::collections::BTreeMap::new();
            #(#field_schemas_logic)*
            Some(map)
        }
    } else {
        quote! { None }
    };

    (schema_body, field_schemas_body)
}

fn impl_enum_schema(openapi_path: &TokenStream, serde_json_path: &TokenStream, data: DataEnum) -> TokenStream {
    let is_string_enum = data
        .variants
        .iter()
        .all(|v| matches!(v.fields, Fields::Unit));

    if is_string_enum {
        let variants: Vec<String> = data.variants.iter().map(|v| v.ident.to_string()).collect();
        let push_variants = variants.iter().map(|v| quote! { #v.into() });

        return quote! {
            let name_cow = <Self as #openapi_path::schema::RustApiSchema>::name();
            let name = name_cow.as_ref();

            if let Some(_) = ctx.components.get(name) {
                return #openapi_path::schema::SchemaRef::Ref { reference: format!("#/components/schemas/{}", name) };
            }

            let mut schema = #openapi_path::schema::JsonSchema2020::string();
            schema.enum_values = Some(vec![ #(#push_variants),* ]);

            ctx.components.insert(name.to_string(), schema);

            #openapi_path::schema::SchemaRef::Ref { reference: format!("#/components/schemas/{}", name) }
        };
    }

    let mut one_of_logic = Vec::new();

    for variant in data.variants {
        let variant_name = variant.ident.to_string();
        let fields = variant.fields;

        match fields {
            Fields::Named(named) => {
                let mut props_logic = Vec::new();
                for field in named.named {
                    let fname = field.ident.unwrap().to_string();
                    let fty = field.ty;
                    props_logic.push(quote! {
                        let fs_ref = <#fty as #openapi_path::schema::RustApiSchema>::schema(ctx);
                        let fs = match fs_ref {
                            #openapi_path::schema::SchemaRef::Schema(s) => *s,
                            #openapi_path::schema::SchemaRef::Ref { reference } => {
                                let mut s = #openapi_path::schema::JsonSchema2020::new();
                                s.reference = Some(reference);
                                s
                            },
                            #openapi_path::schema::SchemaRef::Inline(v) => {
                                #serde_json_path::from_value(v).unwrap_or_default()
                            },
                        };
                        v_props.insert(#fname.to_string(), fs);
                        v_req.push(#fname.to_string());
                    });
                }

                one_of_logic.push(quote! {
                    {
                        let mut v_props = ::std::collections::BTreeMap::new();
                        let mut v_req = Vec::new();
                        #(#props_logic)*

                        let mut v_schema = #openapi_path::schema::JsonSchema2020::object();
                        v_schema.properties = Some(v_props);
                        v_schema.required = Some(v_req);

                        let mut outer_props = ::std::collections::BTreeMap::new();
                        outer_props.insert(#variant_name.to_string(), v_schema);
                        let mut outer = #openapi_path::schema::JsonSchema2020::object();
                        outer.properties = Some(outer_props);
                        outer.required = Some(vec![#variant_name.to_string()]);

                        outer
                    }
                });
            }
            Fields::Unnamed(unnamed) => {
                if unnamed.unnamed.len() == 1 {
                    let fty = &unnamed.unnamed[0].ty;
                    one_of_logic.push(quote! {
                        {
                            let fs_ref = <#fty as #openapi_path::schema::RustApiSchema>::schema(ctx);
                            let fs = match fs_ref {
                                #openapi_path::schema::SchemaRef::Schema(s) => *s,
                                #openapi_path::schema::SchemaRef::Ref { reference } => {
                                    let mut s = #openapi_path::schema::JsonSchema2020::new();
                                    s.reference = Some(reference);
                                    s
                                },
                                #openapi_path::schema::SchemaRef::Inline(v) => {
                                    #serde_json_path::from_value(v).unwrap_or_default()
                                },
                            };

                            let mut outer_props = ::std::collections::BTreeMap::new();
                            outer_props.insert(#variant_name.to_string(), fs);
                            let mut outer = #openapi_path::schema::JsonSchema2020::object();
                            outer.properties = Some(outer_props);
                            outer.required = Some(vec![#variant_name.to_string()]);
                            outer
                        }
                    });
                } else {
                    one_of_logic.push(quote! {
                        #openapi_path::schema::JsonSchema2020::object()
                    });
                }
            }
            Fields::Unit => {
                one_of_logic.push(quote! {
                     {
                         let mut s = #openapi_path::schema::JsonSchema2020::string();
                         s.enum_values = Some(vec![#variant_name.into()]);
                         s
                     }
                });
            }
        }
    }

    quote! {
        let name_cow = <Self as #openapi_path::schema::RustApiSchema>::name();
        let name = name_cow.as_ref();

        if let Some(_) = ctx.components.get(name) {
            return #openapi_path::schema::SchemaRef::Ref { reference: format!("#/components/schemas/{}", name) };
        }

        ctx.components.insert(name.to_string(), #openapi_path::schema::JsonSchema2020::new());

        let mut schema = #openapi_path::schema::JsonSchema2020::new();
        schema.one_of = Some(vec![ #(#one_of_logic),* ]);

        ctx.components.insert(name.to_string(), schema);

        #openapi_path::schema::SchemaRef::Ref { reference: format!("#/components/schemas/{}", name) }
    }
}
