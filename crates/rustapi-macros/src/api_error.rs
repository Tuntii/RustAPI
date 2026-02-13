use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Expr, Lit, Meta};

fn get_core_path() -> proc_macro2::TokenStream {
    let rustapi_rs_found = crate_name("rustapi-rs").or_else(|_| crate_name("rustapi_rs"));

    if let Ok(found) = rustapi_rs_found {
        match found {
            FoundCrate::Itself => quote! { ::rustapi_rs::__private::core },
            FoundCrate::Name(name) => {
                let normalized = name.replace('-', "_");
                let ident = syn::Ident::new(&normalized, proc_macro2::Span::call_site());
                quote! { ::#ident::__private::core }
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

pub fn expand_derive_api_error(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let core_path = get_core_path();

    let variants = match &input.data {
        Data::Enum(data) => &data.variants,
        _ => {
            return syn::Error::new_spanned(input, "ApiError can only be derived for enums")
                .to_compile_error()
                .into()
        }
    };

    let mut match_arms = Vec::new();

    for variant in variants {
        let variant_name = &variant.ident;
        let attrs = &variant.attrs;

        // Parse #[error(...)] attributes
        let mut status = None;
        let mut code = None;
        let mut message = None;

        for attr in attrs {
            if attr.path().is_ident("error") {
                if let Ok(nested) = attr.parse_args_with(
                    syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
                ) {
                    for meta in nested {
                        if let Meta::NameValue(nv) = meta {
                            if nv.path.is_ident("status") {
                                if let Expr::Lit(lit) = &nv.value {
                                    if let Lit::Int(i) = &lit.lit {
                                        status = Some(i.base10_parse::<u16>().unwrap());
                                    }
                                }
                            } else if nv.path.is_ident("code") {
                                if let Expr::Lit(lit) = &nv.value {
                                    if let Lit::Str(s) = &lit.lit {
                                        code = Some(s.value());
                                    }
                                }
                            } else if nv.path.is_ident("message") {
                                if let Expr::Lit(lit) = &nv.value {
                                    if let Lit::Str(s) = &lit.lit {
                                        message = Some(s.value());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let status = status.unwrap_or(500);
        let code = code.unwrap_or_else(|| "internal_server_error".to_string());
        let message = message.unwrap_or_else(|| "Internal Server Error".to_string());

        match_arms.push(quote! {
            #name::#variant_name => {
                #core_path::ApiError::new(
                    #core_path::StatusCode::from_u16(#status).unwrap(),
                    #code,
                    #message
                ).into_response()
            }
        });
    }

    let expanded = quote! {
        impl #core_path::IntoResponse for #name {
            fn into_response(self) -> #core_path::Response {
                match self {
                    #(#match_arms)*
                }
            }
        }
    };

    expanded.into()
}
