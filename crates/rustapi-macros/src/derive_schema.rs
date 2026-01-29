use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DataEnum, DataStruct, Fields, Ident};

pub fn expand_derive_schema(input: syn::DeriveInput) -> TokenStream {
    let name = input.ident;
    let generics = input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let (schema_impl, field_schemas_impl) = match input.data {
        Data::Struct(data) => impl_struct_schema_bodies(&name, data),
        Data::Enum(data) => (impl_enum_schema(&name, data), quote! { None }),
        Data::Union(_) => {
            return syn::Error::new_spanned(name, "Unions not supported")
                .to_compile_error();
        }
    };

    quote! {
        impl #impl_generics ::rustapi_openapi::schema::RustApiSchema for #name #ty_generics #where_clause {
            fn schema(ctx: &mut ::rustapi_openapi::schema::SchemaCtx) -> ::rustapi_openapi::schema::SchemaRef {
                #schema_impl
            }

            fn component_name() -> Option<&'static str> {
                Some(stringify!(#name))
            }

            fn field_schemas(ctx: &mut ::rustapi_openapi::schema::SchemaCtx) -> Option<::std::collections::BTreeMap<String, ::rustapi_openapi::schema::SchemaRef>> {
                #field_schemas_impl
            }
        }
    }
}

fn impl_struct_schema_bodies(name: &Ident, data: DataStruct) -> (TokenStream, TokenStream) {
    let name_str = name.to_string();

    let mut field_logic = Vec::new();
    let mut field_schemas_logic = Vec::new();

    match data.fields {
        Fields::Named(fields) => {
            for field in fields.named {
                let field_name = field.ident.unwrap();
                let field_name_str = field_name.to_string();
                let field_type = field.ty;

                let is_option = if let syn::Type::Path(tp) = &field_type {
                    tp.path.segments.last().map(|s| s.ident == "Option").unwrap_or(false)
                } else {
                    false
                };

                let required_push = if !is_option {
                    quote! { required.push(#field_name_str.to_string()); }
                } else {
                    quote! {}
                };

                field_logic.push(quote! {
                    let field_schema_ref = <#field_type as ::rustapi_openapi::schema::RustApiSchema>::schema(ctx);
                    let field_schema = match field_schema_ref {
                        ::rustapi_openapi::schema::SchemaRef::Schema(s) => s,
                        ::rustapi_openapi::schema::SchemaRef::Ref { reference } => {
                            let mut s = ::rustapi_openapi::schema::JsonSchema2020::new();
                            s.reference = Some(reference);
                            s
                        },
                        ::rustapi_openapi::schema::SchemaRef::Inline(v) => {
                            let mut s = ::rustapi_openapi::schema::JsonSchema2020::new();
                            s
                        }
                    };
                    properties.insert(#field_name_str.to_string(), field_schema);
                    #required_push
                });

                field_schemas_logic.push(quote! {
                    let field_schema_ref = <#field_type as ::rustapi_openapi::schema::RustApiSchema>::schema(ctx);
                    map.insert(#field_name_str.to_string(), field_schema_ref);
                });
            }
        }
        _ => { /* Unnamed/Unit structs skipped for field_schemas */ }
    }

    let schema_body = quote! {
        let name = #name_str;
        if let Some(_) = ctx.components.get(name) {
            return ::rustapi_openapi::schema::SchemaRef::Ref { reference: format!("#/components/schemas/{}", name) };
        }

        ctx.components.insert(name.to_string(), ::rustapi_openapi::schema::JsonSchema2020::new());

        let mut properties = ::std::collections::BTreeMap::new();
        let mut required = Vec::new();

        #(#field_logic)*

        let mut schema = ::rustapi_openapi::schema::JsonSchema2020::object();
        schema.properties = Some(properties);
        if !required.is_empty() {
            schema.required = Some(required);
        }

        ctx.components.insert(name.to_string(), schema);

        ::rustapi_openapi::schema::SchemaRef::Ref { reference: format!("#/components/schemas/{}", name) }
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

fn impl_enum_schema(name: &Ident, data: DataEnum) -> TokenStream {
    let name_str = name.to_string();

    let is_string_enum = data.variants.iter().all(|v| matches!(v.fields, Fields::Unit));

    if is_string_enum {
        let variants: Vec<String> = data.variants.iter().map(|v| v.ident.to_string()).collect();
        let push_variants = variants.iter().map(|v| quote! { #v.into() });

        return quote! {
            let name = #name_str;
            if let Some(_) = ctx.components.get(name) {
                return ::rustapi_openapi::schema::SchemaRef::Ref { reference: format!("#/components/schemas/{}", name) };
            }

            let mut schema = ::rustapi_openapi::schema::JsonSchema2020::string();
            schema.enum_values = Some(vec![ #(#push_variants),* ]);

            ctx.components.insert(name.to_string(), schema);

            ::rustapi_openapi::schema::SchemaRef::Ref { reference: format!("#/components/schemas/{}", name) }
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
                        let fs_ref = <#fty as ::rustapi_openapi::schema::RustApiSchema>::schema(ctx);
                        let fs = match fs_ref {
                            ::rustapi_openapi::schema::SchemaRef::Schema(s) => s,
                            ::rustapi_openapi::schema::SchemaRef::Ref { reference } => {
                                let mut s = ::rustapi_openapi::schema::JsonSchema2020::new();
                                s.reference = Some(reference);
                                s
                            },
                            _ => ::rustapi_openapi::schema::JsonSchema2020::new(),
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

                        let mut v_schema = ::rustapi_openapi::schema::JsonSchema2020::object();
                        v_schema.properties = Some(v_props);
                        v_schema.required = Some(v_req);

                        let mut outer_props = ::std::collections::BTreeMap::new();
                        outer_props.insert(#variant_name.to_string(), v_schema);
                        let mut outer = ::rustapi_openapi::schema::JsonSchema2020::object();
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
                            let fs_ref = <#fty as ::rustapi_openapi::schema::RustApiSchema>::schema(ctx);
                            let fs = match fs_ref {
                                ::rustapi_openapi::schema::SchemaRef::Schema(s) => s,
                                ::rustapi_openapi::schema::SchemaRef::Ref { reference } => {
                                    let mut s = ::rustapi_openapi::schema::JsonSchema2020::new();
                                    s.reference = Some(reference);
                                    s
                                },
                                _ => ::rustapi_openapi::schema::JsonSchema2020::new(),
                            };

                            let mut outer_props = ::std::collections::BTreeMap::new();
                            outer_props.insert(#variant_name.to_string(), fs);
                            let mut outer = ::rustapi_openapi::schema::JsonSchema2020::object();
                            outer.properties = Some(outer_props);
                            outer.required = Some(vec![#variant_name.to_string()]);
                            outer
                        }
                    });
                } else {
                     one_of_logic.push(quote! {
                         ::rustapi_openapi::schema::JsonSchema2020::object()
                     });
                }
            }
            Fields::Unit => {
                one_of_logic.push(quote! {
                     {
                         let mut s = ::rustapi_openapi::schema::JsonSchema2020::string();
                         s.enum_values = Some(vec![#variant_name.into()]);
                         s
                     }
                });
            }
        }
    }

    quote! {
        let name = #name_str;
        if let Some(_) = ctx.components.get(name) {
            return ::rustapi_openapi::schema::SchemaRef::Ref { reference: format!("#/components/schemas/{}", name) };
        }

        ctx.components.insert(name.to_string(), ::rustapi_openapi::schema::JsonSchema2020::new());

        let mut schema = ::rustapi_openapi::schema::JsonSchema2020::new();
        schema.one_of = Some(vec![ #(#one_of_logic),* ]);

        ctx.components.insert(name.to_string(), schema);

        ::rustapi_openapi::schema::SchemaRef::Ref { reference: format!("#/components/schemas/{}", name) }
    }
}
