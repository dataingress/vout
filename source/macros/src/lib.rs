use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod api_unsupported_fields;

#[proc_macro_derive(ApiUnsupportedFields, attributes(api_notsupported, serde))]
pub fn derive_api_unsupported_fields(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match api_unsupported_fields::expand(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
