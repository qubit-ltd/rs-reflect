// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

mod macros;

// qubit-style: allow all
use proc_macro::TokenStream;

/// Derives the runtime facade's re-exported `Reflect` implementation.
#[proc_macro_attribute]
pub fn model_reflect(attribute: TokenStream, item: TokenStream) -> TokenStream {
    macros::model_reflect(attribute, item)
}

/// Reflects a trait through the runtime facade's attribute macro.
#[proc_macro_attribute]
pub fn model_reflect_trait(attribute: TokenStream, item: TokenStream) -> TokenStream {
    macros::model_reflect_trait(attribute, item)
}

/// Reflects an impl through the runtime facade's attribute macro.
#[proc_macro_attribute]
pub fn model_reflect_impl(attribute: TokenStream, item: TokenStream) -> TokenStream {
    macros::model_reflect_impl(attribute, item)
}
