// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Conservative dyn-compatibility analysis helpers.

use proc_macro2::TokenStream;
use syn::{ItemTrait, Type};

use crate::ir::TraitDeclarationIr;

pub(super) fn is_provably_dyn_compatible(
    item: &ItemTrait,
    declaration: &TraitDeclarationIr,
) -> bool {
    super::is_provably_dyn_compatible(item, declaration)
}

pub(super) fn receiver_type_is_dyn_dispatchable(ty: &Type) -> bool {
    super::receiver_type_is_dyn_dispatchable(ty)
}

pub(super) fn contains_unprojected_self(tokens: TokenStream) -> bool {
    super::tokens_contain_unprojected_self(tokens)
}

#[cfg(test)]
mod tests {
    use quote::quote;

    #[test]
    fn dyn_analysis_module_is_loaded() {
        let _entry = super::is_provably_dyn_compatible;
    }

    #[test]
    fn recognizes_dispatchable_receivers_and_projection_exceptions() {
        assert!(super::receiver_type_is_dyn_dispatchable(
            &syn::parse_str("Self").unwrap()
        ));
        assert!(super::receiver_type_is_dyn_dispatchable(
            &syn::parse_str("&Self").unwrap()
        ));
        assert!(!super::receiver_type_is_dyn_dispatchable(
            &syn::parse_str("Vec<Self>").unwrap()
        ));
        assert!(super::contains_unprojected_self(quote!(Self)));
        assert!(!super::contains_unprojected_self(quote!(Self::Assoc)));
        assert!(super::contains_unprojected_self(quote!(Option<Self>)));
    }
}
