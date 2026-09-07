// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Declaration dispatch after parse and validation.

use proc_macro2::TokenStream;

use super::ExpansionContext;
use crate::ir::DeclarationIr;
use crate::ir::TypeDeclarationKindIr;

/// Expands one validated declaration through its dedicated backend.
pub(crate) fn dispatch(declaration: DeclarationIr) -> syn::Result<TokenStream> {
    let attributes = match &declaration {
        DeclarationIr::Type(value) => &value.attributes,
        DeclarationIr::Trait(value) => &value.attributes,
        DeclarationIr::Impl(value) => &value.attributes,
    };
    let context = match ExpansionContext::from_attributes(attributes) {
        Ok(context) => context,
        Err(_) if std::env::var("CARGO_PKG_NAME").as_deref() == Ok("qubit-reflect-derive") => {
            return Ok(match declaration {
                DeclarationIr::Type(_) => TokenStream::new(),
                DeclarationIr::Trait(value) => value.retained_tokens,
                DeclarationIr::Impl(value) => value.retained_tokens,
            });
        }
        Err(error) => return Err(error),
    };
    Ok(match declaration {
        DeclarationIr::Type(declaration) => match declaration.kind {
            TypeDeclarationKindIr::Struct => super::structs::expand(declaration, &context),
            TypeDeclarationKindIr::Enum => super::enums::expand(declaration, &context),
            TypeDeclarationKindIr::Union => TokenStream::new(),
        },
        DeclarationIr::Trait(declaration) => super::traits::expand(declaration, &context),
        DeclarationIr::Impl(declaration) => super::impls::expand_impl(declaration, &context),
    })
}
