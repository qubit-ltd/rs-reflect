//! Minimal downstream attribute macro that delegates to the reflection facade.

#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use quote::quote;
use syn::ItemStruct;
use syn::parse_macro_input;

/// Adds the facade-aware `Reflect` derive to one downstream struct.
#[proc_macro_attribute]
pub fn model_reflect(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    let mut structure = parse_macro_input!(item as ItemStruct);
    structure
        .attrs
        .push(syn::parse_quote!(#[derive(model_facade_runtime::Reflect)]));
    structure
        .attrs
        .push(syn::parse_quote!(#[reflect(crate = model_facade_runtime)]));
    quote!(#structure).into()
}
