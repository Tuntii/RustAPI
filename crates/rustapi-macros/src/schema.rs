use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Attribute, Data, DataEnum, DataStruct, DeriveInput, Expr, Fields,
    FieldsNamed, Lit, Meta,
};

pub fn derive_to_schema(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let schema_impl = match &input.data {
        Data::Struct(s) => expand_struct_schema(s, &name_str),
        Data::Enum(e) => expand_enum_schema(e, &name_str),
        Data::Union(_) => {
            return syn::Error::new_spanned(name, "Unions are not supported for ToSchema")
                .to_compile_error()
                .into();
        }
    };

    let expanded = quote! {
        impl #impl_generics ::rustapi_rs::__private::rustapi_openapi::ToSchema for #name #ty_generics #where_clause {
            fn name() -> String {
                #name_str.to_string()
            }

            fn schema() -> (String, ::rustapi_rs::__private::rustapi_openapi::schema::RefOr<::rustapi_rs::__private::rustapi_openapi::schema::Schema>) {
                #schema_impl
            }
        }
    };

    expanded.into()
}

fn expand_struct_schema(data: &DataStruct, name: &str) -> TokenStream {
    match &data.fields {
        Fields::Named(fields) => expand_named_fields(fields, name),
        Fields::Unnamed(_) => quote! {
            // Tuple structs treated as array likely, or just empty?
            // For now, treat as just named schema with no props?
            // Actually, let's support them as simple objects if possible, or error.
            // Simplified implementation:
             (
                #name.to_string(),
                ::rustapi_rs::__private::rustapi_openapi::schema::Schema {
                    schema_type: Some(::rustapi_rs::__private::rustapi_openapi::schema::SchemaType::Object),
                    description: None,
                    ..Default::default()
                }.into()
            )
        },
        Fields::Unit => quote! {
             (
                #name.to_string(),
                ::rustapi_rs::__private::rustapi_openapi::schema::Schema {
                    schema_type: Some(::rustapi_rs::__private::rustapi_openapi::schema::SchemaType::Object), // or null?
                    description: None,
                    ..Default::default()
                }.into()
            )
        },
    }
}

fn expand_named_fields(fields: &FieldsNamed, name: &str) -> TokenStream {
    let mut props = Vec::new();
    let mut required = Vec::new();

    for field in &fields.named {
        let field_name = field.ident.as_ref().unwrap();
        let mut field_name_str = field_name.to_string();

        let mut is_option = false;

        // Check for Option wrapper to determine if required
        // This is a naive check (string matching), could be improved with type analysis
        // assuming standard Option usage
        if let syn::Type::Path(tp) = &field.ty {
            if let Some(seg) = tp.path.segments.last() {
                if seg.ident == "Option" {
                    is_option = true;
                }
            }
        }

        // Handle serde rename
        if let Some(renamed) = get_serde_rename(&field.attrs) {
            field_name_str = renamed;
        }

        if !is_option {
            required.push(quote! { #field_name_str.to_string() });
        }

        let ty = &field.ty;

        // Property schema generation
        // We defer to <T as ToSchema>::schema()
        // But for Option<T>, we want T's schema.
        // For Vec<T>, we want Array of T.
        // Our ToSchema impl for Option/Vec already handles structure,
        // but we need to supply the reference.

        // If field type implements ToSchema, we can just use it?
        // Yes, ToSchema::schema() returns (name, RefOr<Schema>).
        // If RefOr is Ref, we are good.
        // If RefOr is T (inline), we embed it.

        props.push(quote! {
            map.insert(
                #field_name_str.to_string(),
                <#ty as ::rustapi_rs::__private::rustapi_openapi::ToSchema>::schema().1
            );
        });
    }

    let required_quote = if required.is_empty() {
        quote! { None }
    } else {
        quote! { Some(vec![#(#required),*]) }
    };

    quote! {
        let mut map = ::std::collections::HashMap::new();
        #(#props)*

        (
            #name.to_string(),
            ::rustapi_rs::__private::rustapi_openapi::schema::Schema {
                schema_type: Some(::rustapi_rs::__private::rustapi_openapi::schema::SchemaType::Object),
                properties: Some(map),
                required: #required_quote,
                ..Default::default()
            }.into()
        )
    }
}

fn expand_enum_schema(data: &DataEnum, name: &str) -> TokenStream {
    let mut variants = Vec::new();

    // Simple enum (C-like) support for now (strings)
    for variant in &data.variants {
        let mut variant_name = variant.ident.to_string();
        // Handle serde rename
        if let Some(renamed) = get_serde_rename(&variant.attrs) {
            variant_name = renamed;
        }

        variants.push(quote! {
            ::serde_json::Value::String(#variant_name.to_string())
        });
    }

    quote! {
        (
            #name.to_string(),
            ::rustapi_rs::__private::rustapi_openapi::schema::Schema {
                schema_type: Some(::rustapi_rs::__private::rustapi_openapi::schema::SchemaType::String),
                enum_values: Some(vec![#(#variants),*]),
                ..Default::default()
            }.into()
        )
    }
}

fn get_serde_rename(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("serde") {
            // Parse #[serde(...)]
            // Looking for rename = "name"
            if let Ok(nested) = attr.parse_args_with(
                syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
            ) {
                for meta in nested {
                    if let Meta::NameValue(nv) = meta {
                        if nv.path.is_ident("rename") {
                            if let Expr::Lit(lit) = nv.value {
                                if let Lit::Str(s) = lit.lit {
                                    return Some(s.value());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}
