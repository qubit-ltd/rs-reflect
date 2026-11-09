// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Owner-specific facts used by pure method invocation analysis.

use proc_macro2::TokenStream;

/// Owner-specific facts needed by otherwise pure method analysis.
pub(crate) struct MethodContext<'a> {
    /// Concrete target type used to analyze explicit receiver forms.
    pub(crate) target: &'a TokenStream,
    /// Substituted extension receiver type, when provided by the owner.
    pub(crate) extension_receiver: Option<TokenStream>,
    /// Whether the method is a trait default body.
    pub(crate) default_method: bool,
    /// Whether associated types needed by a default body are not proven.
    pub(crate) has_unproven_associated_type: bool,
}

impl<'a> MethodContext<'a> {
    /// Creates ordinary impl-method analysis context.
    ///
    /// # Parameters
    ///
    /// - `target`: Concrete impl target type tokens.
    ///
    /// # Returns
    ///
    /// Returns an implementation context without trait-default constraints.
    #[must_use]
    #[inline]
    pub(crate) fn implementation(target: &'a TokenStream) -> Self {
        Self {
            target,
            extension_receiver: None,
            default_method: false,
            has_unproven_associated_type: false,
        }
    }

    /// Creates default-trait-method analysis context.
    ///
    /// # Parameters
    ///
    /// - `target`: Trait self type tokens.
    /// - `has_unproven_associated_type`: Whether required owner facts are
    ///   unavailable.
    ///
    /// # Returns
    ///
    /// Returns a context for checking a trait default method.
    #[must_use]
    #[inline]
    pub(crate) fn trait_default(target: &'a TokenStream, has_unproven_associated_type: bool) -> Self {
        Self {
            target,
            extension_receiver: None,
            default_method: true,
            has_unproven_associated_type,
        }
    }
}
