// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Parser for method binding macro inputs.

use syn::ExprPath;
use syn::Result;
use syn::Token;
use syn::Type;
use syn::TypeFnPtr;
use syn::parse::Parse;
use syn::parse::ParseStream;

use super::trait_binding_hint::TraitBindingHint;

/// Parsed inputs controlling one method binding expansion.
pub(super) struct BindingInput {
    /// Fully qualified method path to bind.
    pub(super) method: ExprPath,
    /// Safe Rust ABI function signature expected by the shim.
    pub(super) signature: TypeFnPtr,
    /// Whether the adapter awaits an asynchronous method result.
    pub(super) asynchronous: bool,
    /// Whether generated calls use the thread-safe invocation mode.
    pub(super) thread_safe: bool,
    /// Optional reflected or external trait identity for trait methods.
    pub(super) trait_binding: Option<TraitBindingHint>,
}

impl Parse for BindingInput {
    /// Parses a method path, adapter mode, signature, and optional trait hint.
    ///
    /// # Parameters
    ///
    /// - `input`: Binding macro arguments to parse.
    ///
    /// # Errors
    ///
    /// Returns a syntax diagnostic for unsupported modes, signatures, hints, or
    /// trailing tokens.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let method = input.parse()?;
        input.parse::<Token![as]>()?;
        let thread_safe = input.peek(syn::Ident) && input.fork().parse::<syn::Ident>()? == "thread_safe";
        if thread_safe {
            let mode = input.parse::<syn::Ident>()?;
            if mode != "thread_safe" {
                return Err(syn::Error::new_spanned(mode, "expected `thread_safe` adapter mode"));
            }
        }
        let asynchronous = input.peek(Token![async]);
        if asynchronous {
            input.parse::<Token![async]>()?;
        }
        let ty = input.parse::<Type>()?;
        let Type::FnPtr(signature) = ty else {
            return Err(syn::Error::new_spanned(
                ty,
                "expected a function pointer signature such as `fn(&T) -> U`",
            ));
        };
        let trait_binding = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            let name = input.parse::<syn::Ident>()?;
            input.parse::<Token![=]>()?;
            let path = input.parse::<syn::Path>()?;
            match name.to_string().as_str() {
                "reflected_trait" => Some(TraitBindingHint::Reflected(path)),
                "external_trait" => Some(TraitBindingHint::External(path)),
                _ => {
                    return Err(syn::Error::new_spanned(
                        name,
                        "expected `reflected_trait = TraitPath` or `external_trait = Handle`",
                    ));
                }
            }
        } else {
            None
        };
        if !input.is_empty() {
            return Err(input.error("unexpected tokens after the method signature"));
        }
        Ok(Self {
            method,
            signature,
            asynchronous,
            thread_safe,
            trait_binding,
        })
    }
}
