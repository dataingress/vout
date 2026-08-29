use convert_case::{Case, Casing};
use quote::quote;
use syn::{Data, DeriveInput, Fields, LitStr};

pub fn expand(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let ident = input.ident;
    let rename_all = serde_rename_all(&input.attrs)?;
    let fields = match input.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    ident,
                    "ApiUnsupportedFields only supports structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                ident,
                "ApiUnsupportedFields only supports structs",
            ));
        }
    };

    let checks = fields
        .iter()
        .filter(|field| has_api_notsupported(field))
        .map(|field| {
            let field_ident = field.ident.as_ref().expect("named field");
            let field_name = serde_rename(field)?
                .or_else(|| {
                    rename_all
                        .as_deref()
                        .map(|case| rename_field(field_ident, case))
                })
                .unwrap_or_else(|| field_ident.to_string());

            Ok(quote! {
                if self.#field_ident.is_some() {
                    return Some(#field_name.to_owned());
                }
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        impl #ident {
            fn unsupported_field(&self) -> Option<String> {
                #(#checks)*
                None
            }
        }
    })
}

fn has_api_notsupported(field: &syn::Field) -> bool {
    field
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("api_notsupported"))
}

fn serde_rename(field: &syn::Field) -> syn::Result<Option<String>> {
    let mut rename = None;

    for attr in &field.attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                rename = Some(lit.value());
            }
            Ok(())
        })?;
    }

    Ok(rename)
}

fn serde_rename_all(attrs: &[syn::Attribute]) -> syn::Result<Option<String>> {
    let mut rename_all = None;

    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename_all") {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                rename_all = Some(lit.value());
            }
            Ok(())
        })?;
    }

    Ok(rename_all)
}

fn rename_field(ident: &syn::Ident, rename_all: &str) -> String {
    let field = ident.to_string().trim_start_matches('_').to_owned();

    match rename_all {
        "PascalCase" => field.to_case(Case::Pascal),
        "camelCase" => field.to_case(Case::Camel),
        "snake_case" => field.to_case(Case::Snake),
        "SCREAMING_SNAKE_CASE" => field.to_case(Case::UpperSnake),
        _ => field,
    }
}
