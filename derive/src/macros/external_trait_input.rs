// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Parser for external trait marker declarations.

use syn::Token;
use syn::parse::Parse;
use syn::parse::ParseStream;

/// Parsed declaration naming an external trait and its local marker type.
pub(super) struct ExternalTraitInput {
    /// Visibility assigned to the generated marker.
    pub(super) visibility: syn::Visibility,
    /// Identifier assigned to the marker type.
    pub(super) marker: syn::Ident,
    /// External trait path whose metadata will be registered.
    pub(super) path: syn::Path,
}

impl Parse for ExternalTraitInput {
    /// Parses `visibility Marker = TraitPath;` for external trait metadata.
    ///
    /// # Parameters
    ///
    /// - `input`: Token stream containing the external trait declaration.
    ///
    /// # Errors
    ///
    /// Returns a syntax error for malformed declarations or trailing tokens.
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let visibility = input.parse()?;
        let marker = input.parse()?;
        input.parse::<Token![=]>()?;
        let path = input.parse()?;
        input.parse::<Token![;]>()?;
        if !input.is_empty() {
            return Err(input.error("unexpected tokens after external trait declaration"));
        }
        Ok(Self {
            visibility,
            marker,
            path,
        })
    }
}
