use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Expr, Fields, Lit, Meta};

pub fn derive_into_params(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let params_impl = match &input.data {
        Data::Struct(s) => expand_struct_params(s),
        _ => {
            return syn::Error::new_spanned(name, "IntoParams only supported for structs")
                .to_compile_error()
                .into();
        }
    };

    let expanded = quote! {
        impl #impl_generics ::rustapi_rs::__private::rustapi_openapi::IntoParams for #name #ty_generics #where_clause {
            fn into_params(parameter_in_provider: impl Fn() -> Option<::rustapi_rs::__private::rustapi_openapi::ParameterIn>) -> Vec<::rustapi_rs::__private::rustapi_openapi::Parameter> {
                let default_location = ::rustapi_rs::__private::rustapi_openapi::ParameterIn::Query;
                let location_enum = parameter_in_provider().unwrap_or(default_location);
                let location_str = location_enum.to_string();

                #params_impl
            }
        }
    };

    expanded.into()
}

fn expand_struct_params(data: &syn::DataStruct) -> TokenStream {
    let mut params = Vec::new();

    match &data.fields {
        Fields::Named(fields) => {
            for field in &fields.named {
                let ident = field.ident.as_ref().unwrap();
                let ident_str = ident.to_string();

                let mut param_name = ident_str;
                let mut is_option = false;

                // Check for Option wrapper
                if let syn::Type::Path(tp) = &field.ty {
                    if let Some(seg) = tp.path.segments.last() {
                        if seg.ident == "Option" {
                            is_option = true;
                        }
                    }
                }

                // Handle serde rename
                if let Some(renamed) = get_serde_rename(&field.attrs) {
                    param_name = renamed;
                }

                let required = !is_option;
                let ty = &field.ty;

                // Doc comments handling (simplified)
                let description = quote! { None };

                params.push(quote! {
                    ::rustapi_rs::__private::rustapi_openapi::Parameter {
                        name: #param_name.to_string(),
                        location: location_str.clone(),
                        required: #required,
                        description: #description,
                        schema: <#ty as ::rustapi_rs::__private::rustapi_openapi::ToSchema>::schema().1,
                    }
                });
            }
        }
        _ => {
            // Unnamed fields / Unit structs not supported for IntoParams (usually query params are named)
            return quote! { vec![] };
        }
    }

    quote! {
        vec![
            #(#params),*
        ]
    }
}

fn get_serde_rename(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("serde") {
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
