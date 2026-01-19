use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Expr, Lit, Meta};

pub fn expand_derive_api_error(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

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
                ::rustapi_core::ApiError::new(
                    ::rustapi_core::StatusCode::from_u16(#status).unwrap(),
                    #code,
                    #message
                ).into_response()
            }
        });
    }

    let expanded = quote! {
        impl ::rustapi_core::IntoResponse for #name {
            fn into_response(self) -> ::rustapi_core::Response {
                match self {
                    #(#match_arms)*
                }
            }
        }
    };

    expanded.into()
}
