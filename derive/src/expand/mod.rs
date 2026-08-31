// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Token expansion for validated reflection declarations.

mod construction;
mod enums;
mod generics;
mod impls;
mod structs;
mod traits;

use proc_macro2::TokenStream;

use crate::ir::DeclarationIr;
use crate::ir::HelperAttributeIr;
use crate::ir::HelperValueIr;

/// Expands one validated declaration while retaining unsupported staged
/// declarations verbatim.
pub(crate) fn expand(declaration: DeclarationIr) -> TokenStream {
    match declaration {
        DeclarationIr::Type(declaration) => match declaration.kind {
            crate::ir::TypeDeclarationKindIr::Struct => structs::expand(declaration),
            crate::ir::TypeDeclarationKindIr::Enum => enums::expand(declaration),
            crate::ir::TypeDeclarationKindIr::Union => TokenStream::new(),
        },
        DeclarationIr::Trait(declaration) => traits::expand(declaration),
        DeclarationIr::Impl(declaration) => impls::expand_impl(declaration),
    }
}

/// Resolves the runtime facade without coupling the proc-macro crate to it.
fn facade_path() -> Option<TokenStream> {
    match proc_macro_crate::crate_name("qubit-reflect") {
        Ok(proc_macro_crate::FoundCrate::Itself) => Some(quote::quote!(crate)),
        Ok(proc_macro_crate::FoundCrate::Name(name)) => {
            let ident = proc_macro2::Ident::new(&name, proc_macro2::Span::call_site());
            Some(quote::quote!(::#ident))
        }
        Err(_) => None,
    }
}

/// Resolves an explicit downstream facade before falling back to Cargo's
/// dependency-name lookup for direct `qubit-reflect` users.
fn facade_path_for(attributes: &[HelperAttributeIr]) -> Option<TokenStream> {
    attributes
        .iter()
        .find_map(|attribute| match &attribute.value {
            HelperValueIr::RuntimeCrate(path) => Some(path.tokens.clone()),
            _ => None,
        })
        .or_else(facade_path)
}
