// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Shared facts required by every expansion backend.

use proc_macro_crate::FoundCrate;
use proc_macro_crate::crate_name;
use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Error;
use syn::Result as SynResult;

use crate::ir::HelperAttributeIr;

/// Immutable context shared by one declaration expansion.
#[derive(Debug)]
pub(crate) struct ExpansionContext {
    /// Caller-visible path to the runtime crate.
    facade: TokenStream,
}

impl ExpansionContext {
    #[cfg(test)]
    pub(crate) fn for_tests() -> Self {
        Self {
            facade: quote!(::qubit_reflect),
        }
    }

    /// Resolves the runtime crate from the current Cargo package.
    ///
    /// # Parameters
    ///
    /// - `_attributes`: Validated helper attributes attached to the
    ///   declaration.
    ///
    /// # Returns
    ///
    /// Returns the expansion context, or a diagnostic when the runtime crate
    /// cannot be resolved.
    ///
    /// # Errors
    ///
    /// Returns a syntax diagnostic when the runtime crate cannot be resolved.
    pub(crate) fn from_attributes(_attributes: &[HelperAttributeIr]) -> SynResult<Self> {
        Self::from_found_crate(crate_name("qubit-reflect").ok())
    }

    /// Returns the caller-visible runtime facade path.
    ///
    /// # Returns
    ///
    /// Returns the runtime facade token stream used in generated paths.
    #[must_use]
    #[inline]
    pub(crate) fn facade(&self) -> &TokenStream {
        &self.facade
    }

    /// Computes the stable content fingerprint used by fragment identities.
    ///
    /// # Parameters
    ///
    /// - `source`: Normalized source representation to fingerprint.
    ///
    /// # Returns
    ///
    /// Returns the deterministic 64-bit FNV-1a fingerprint.
    #[must_use]
    #[inline]
    pub(crate) fn fingerprint(&self, source: &str) -> u64 {
        fingerprint(source)
    }

    /// Builds the facade path from the dependency resolver result.
    ///
    /// # Parameters
    ///
    /// - `found`: Resolved package identity, or `None` when no dependency is
    ///   available.
    ///
    /// # Returns
    ///
    /// Returns the context, or a diagnostic directing the caller to configure
    /// the runtime path.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when no runtime dependency was found.
    pub(super) fn from_found_crate(found: Option<FoundCrate>) -> SynResult<Self> {
        let facade = match found {
            Some(FoundCrate::Itself) => quote!(::qubit_reflect),
            Some(FoundCrate::Name(name)) => {
                let identifier = Ident::new(&name, Span::call_site());
                quote!(::#identifier)
            }
            None => {
                return Err(Error::new(
                    Span::call_site(),
                    "cannot resolve `qubit-reflect`; add it as a direct dependency",
                ));
            }
        };
        Ok(Self { facade })
    }
}

/// Computes one deterministic FNV-1a content fingerprint.
///
/// # Parameters
///
/// - `source`: Source text whose bytes are fingerprinted in order.
///
/// # Returns
///
/// Returns the 64-bit FNV-1a fingerprint.
#[must_use]
pub(crate) fn fingerprint(source: &str) -> u64 {
    source.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}

#[cfg(test)]
mod tests {
    use proc_macro_crate::FoundCrate;

    use super::ExpansionContext;

    #[test]
    fn test_package_example_uses_library_path() {
        let context =
            ExpansionContext::from_found_crate(Some(FoundCrate::Itself)).expect("package runtime must resolve");
        assert_eq!(context.facade().to_string(), ":: qubit_reflect");
    }

    #[test]
    fn test_renamed_dependency_becomes_an_absolute_path() {
        let context = ExpansionContext::from_found_crate(Some(FoundCrate::Name("reflect_runtime".to_owned())))
            .expect("renamed dependency must resolve");
        assert_eq!(
            context.facade().to_string(),
            ":: reflect_runtime",
            "renamed dependencies should use an absolute runtime path",
        );
    }

    #[test]
    fn test_missing_runtime_reports_actionable_error() {
        let error = ExpansionContext::from_found_crate(None).expect_err("missing runtime must fail");
        assert!(
            error.to_string().contains("add it as a direct dependency"),
            "missing runtime diagnostics should explain how to add the dependency",
        );
    }
}
