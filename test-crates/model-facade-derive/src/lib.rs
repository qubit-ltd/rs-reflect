//! A downstream attribute macro that delegates reflection to its runtime facade.

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

/// Derives the runtime facade's re-exported `Reflect` implementation.
#[proc_macro_attribute]
pub fn model_reflect(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as syn::ItemStruct);
    quote! {
        #[derive(model_facade_runtime::Reflect)]
        #[reflect(crate = model_facade_runtime)]
        #item
    }
    .into()
}
