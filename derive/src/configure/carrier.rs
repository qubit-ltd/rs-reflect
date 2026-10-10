// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Stable source-derived names for internal compiler-filtered carriers.

use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro2::TokenStream;

/// Produces a private carrier name unique to a declaration token stream.
///
/// # Parameters
///
/// - `source`: Original declaration tokens, including its members.
/// - `arguments`: Outer reflection macro arguments.
///
/// # Returns
///
/// Returns a deterministic identifier suitable for the carrier item.
#[must_use]
pub(super) fn carrier_name(source: &TokenStream, arguments: &TokenStream) -> Ident {
    let source = format!("{source}{arguments}");
    let hash = source.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    });
    Ident::new(&format!("__QuBitReflectConfigured_{hash:016x}"), Span::call_site())
}
