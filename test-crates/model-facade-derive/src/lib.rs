// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! A downstream attribute macro that delegates reflection to its runtime
//! facade.

use proc_macro::TokenStream;
use quote::quote;
use syn::DeriveInput;
use syn::parse_macro_input;

/// Derives the runtime facade's re-exported `Reflect` implementation.
#[proc_macro_attribute]
pub fn model_reflect(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as DeriveInput);
    quote! {
        #[derive(model_facade_runtime::Reflect)]
        #[reflect(crate = model_facade_runtime)]
        #item
    }
    .into()
}

/// Reflects a trait through the runtime facade's attribute macro.
#[proc_macro_attribute]
pub fn model_reflect_trait(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    let item = proc_macro2::TokenStream::from(item);
    quote! {
        #[model_facade_runtime::reflect(crate = model_facade_runtime)]
        #item
    }
    .into()
}

/// Reflects an impl through the runtime facade's attribute macro.
#[proc_macro_attribute]
pub fn model_reflect_impl(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    let item = proc_macro2::TokenStream::from(item);
    quote! {
        #[model_facade_runtime::reflect_impl(crate = model_facade_runtime)]
        #item
    }
    .into()
}
