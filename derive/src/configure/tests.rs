// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Unit tests for reflection carrier construction.

use proc_macro2::TokenStream;
use quote::quote;
use super::pipeline::make_carrier;
use crate::ir::MacroKind;

/// Converts one carrier result to normalized tokens for focused assertions.
fn carrier_tokens(kind: MacroKind, arguments: TokenStream, item: TokenStream) -> String {
    make_carrier(kind, arguments, item)
        .expect("the declaration should produce a configure carrier")
        .to_string()
}

#[test]
fn test_trait_carrier_projects_conditional_reflection_helpers() {
    let carrier = carrier_tokens(
        MacroKind::Trait,
        quote!(),
        quote! {
            pub trait Service {
                #[cfg(feature = "extended")]
                #[reflect(rename = "read")]
                fn read_value(&self);
            }
        },
    );

    assert!(carrier.contains("trait Service"), "{carrier}");
    assert!(carrier.contains("reflect_configure_member"), "{carrier}");
    assert!(carrier.contains("cfg (feature = \"extended\")"), "{carrier}");
    assert!(carrier.contains("rename = \"read\""), "{carrier}");
    assert!(carrier.contains("__member_0"), "{carrier}");
}

#[test]
fn test_impl_carrier_preserves_method_tokens_for_reconstruction() {
    let carrier = carrier_tokens(
        MacroKind::Impl,
        quote!(),
        quote! {
            impl Service for Example {
                fn read_value(&self) -> u32 { 7 }
            }
        },
    );

    assert!(carrier.contains("impl Service for Example"), "{carrier}");
    assert!(carrier.contains("fn read_value (& self) -> u32 { 7 }"), "{carrier}");
    assert!(carrier.contains("reflect_configure_header"), "{carrier}");
}

#[test]
fn test_trait_carrier_retains_macro_members_without_reflection_helpers() {
    let carrier = carrier_tokens(
        MacroKind::Trait,
        quote!(),
        quote! {
            trait GeneratedService {
                generated_methods!();
            }
        },
    );

    assert!(carrier.contains("generated_methods ! ()"), "{carrier}");
    assert!(carrier.contains("__member_0"), "{carrier}");
}

#[test]
fn test_impl_carrier_rejects_non_impl_items() {
    let error = make_carrier(
        MacroKind::Impl,
        quote!(),
        quote!(
            struct NotAnImpl;
        ),
    )
    .expect_err("reflect_impl only accepts impl blocks");

    assert!(
        error.to_string().contains("can only be applied to an impl block"),
        "non-impl input must report the attribute macro restriction",
    );
}

#[test]
fn test_carrier_rejects_derive_macro_kind() {
    let error = make_carrier(
        MacroKind::Derive,
        quote!(),
        quote!(
            struct Example;
        ),
    )
    .expect_err("derive macros do not use declaration carriers");

    assert!(
        error.to_string().contains("only supports trait and impl macros"),
        "derive macro kind must report unsupported carrier usage",
    );
}
