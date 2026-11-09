// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Parsing for trait macro helpers and indexed supertrait bindings.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Meta;
use syn::Result as SynResult;
use syn::Token;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse2;

use crate::ir::TraitSupertraitBindingIr;

/// Parsed helper attributes and provider bindings for a reflected trait.
pub(super) struct TraitMacroArguments {
    /// Helper metadata forwarded to the trait declaration parser.
    pub(super) helpers: TokenStream,
    /// Indexed providers that resolve generic supertrait applications.
    pub(super) bindings: Vec<TraitSupertraitBindingIr>,
}

impl Parse for TraitMacroArguments {
    /// Parses ordinary helper metadata and `supertrait_binding(index => path)`
    /// entries.
    ///
    /// # Parameters
    ///
    /// - `input`: Trait macro argument tokens.
    ///
    /// # Errors
    ///
    /// Returns a syntax diagnostic for malformed bindings or helper metadata.
    fn parse(input: ParseStream<'_>) -> SynResult<Self> {
        let mut helpers = TokenStream::new();
        let mut bindings = Vec::new();
        while !input.is_empty() {
            let lookahead = input.fork();
            let is_binding = lookahead
                .parse::<syn::Ident>()
                .is_ok_and(|name| name == "supertrait_binding");
            if is_binding {
                let name = input.parse::<syn::Ident>()?;
                let content;
                syn::parenthesized!(content in input);
                let index = content.parse::<syn::LitInt>()?.base10_parse::<usize>()?;
                content.parse::<Token![=>]>()?;
                let provider = content.parse::<syn::Path>()?;
                if !content.is_empty() {
                    return Err(content.error("expected `supertrait_binding(index => path)`"));
                }
                bindings.push(TraitSupertraitBindingIr {
                    index,
                    provider: crate::parse::type_ir::convert_path(&provider),
                });
                let _ = name;
            } else {
                let helper = input.parse::<Meta>()?;
                helpers.extend(quote!(#helper));
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
            if !helpers.is_empty() {
                helpers.extend(quote!(,));
            }
        }
        Ok(Self { helpers, bindings })
    }
}

/// Parses trait helper arguments into forwarded metadata and indexed bindings.
///
/// # Parameters
///
/// - `tokens`: Macro argument tokens from the source attribute.
///
/// # Errors
///
/// Returns a syntax diagnostic if any trait helper argument is malformed.
pub(super) fn parse_trait_arguments(tokens: TokenStream) -> SynResult<(TokenStream, Vec<TraitSupertraitBindingIr>)> {
    if tokens.is_empty() {
        return Ok((TokenStream::new(), Vec::new()));
    }
    let arguments: TraitMacroArguments = parse2(tokens)?;
    Ok((arguments.helpers, arguments.bindings))
}
