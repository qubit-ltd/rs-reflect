//! Procedural macros for `qubit-reflect`.

#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use quote::quote;

/// Rejects `Reflect` derive invocations until reflection generation is implemented.
#[proc_macro_derive(Reflect)]
pub fn derive_reflect(_input: TokenStream) -> TokenStream {
    quote!(compile_error!("qubit-reflect: Reflect derive is not implemented yet");).into()
}

/// Rejects `#[reflect]` attribute invocations until reflection generation is implemented.
#[proc_macro_attribute]
pub fn reflect(_attribute: TokenStream, _item: TokenStream) -> TokenStream {
    quote!(compile_error!("qubit-reflect: #[reflect] is not implemented yet");).into()
}

/// Rejects `#[reflect_impl]` attribute invocations until reflection generation is implemented.
#[proc_macro_attribute]
pub fn reflect_impl(_attribute: TokenStream, _item: TokenStream) -> TokenStream {
    quote!(compile_error!("qubit-reflect: #[reflect_impl] is not implemented yet");).into()
}
