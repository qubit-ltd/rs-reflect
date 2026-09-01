// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Shared facts required by every expansion backend.

use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;

use crate::ir::HelperAttributeIr;
use crate::ir::HelperValueIr;

/// Immutable context shared by one declaration expansion.
#[derive(Debug)]
pub(crate) struct ExpansionContext {
    facade: TokenStream,
}

impl ExpansionContext {
    /// Resolves an explicit runtime facade or the Cargo dependency name.
    pub(crate) fn from_attributes(attributes: &[HelperAttributeIr]) -> syn::Result<Self> {
        if let Some(facade) = attributes.iter().find_map(|attribute| {
            if let HelperValueIr::RuntimeCrate(path) = &attribute.value {
                Some(path.tokens.clone())
            } else {
                None
            }
        }) {
            return Ok(Self { facade });
        }
        Self::from_found_crate(proc_macro_crate::crate_name("qubit-reflect").ok())
    }

    /// Returns the caller-visible runtime facade path.
    pub(crate) fn facade(&self) -> &TokenStream {
        &self.facade
    }

    /// Computes the stable content fingerprint used by fragment identities.
    pub(crate) fn fingerprint(&self, source: &str) -> u64 {
        fingerprint(source)
    }

    fn from_found_crate(found: Option<proc_macro_crate::FoundCrate>) -> syn::Result<Self> {
        let facade = match found {
            Some(proc_macro_crate::FoundCrate::Itself) => quote!(crate),
            Some(proc_macro_crate::FoundCrate::Name(name)) => {
                let identifier = proc_macro2::Ident::new(&name, Span::call_site());
                quote!(::#identifier)
            }
            None => {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "cannot resolve `qubit-reflect`; use `#[reflect(crate = path)]`",
                ));
            }
        };
        Ok(Self { facade })
    }
}

/// Computes one deterministic FNV-1a content fingerprint.
pub(crate) fn fingerprint(source: &str) -> u64 {
    source.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;
    use quote::quote;

    use super::ExpansionContext;
    use crate::ir::HelperAttributeIr;
    use crate::ir::HelperName;
    use crate::ir::HelperTarget;
    use crate::ir::HelperValueIr;
    use crate::ir::PathIr;

    #[test]
    fn explicit_facade_has_priority() {
        let attributes = [HelperAttributeIr {
            name: HelperName::RuntimeCrate,
            value: HelperValueIr::RuntimeCrate(PathIr {
                source: "framework::reflect".to_owned(),
                segments: Vec::new(),
                leading_colon: false,
                qualified_self: None,
                tokens: quote!(framework::reflect),
                span: Span::call_site(),
            }),
            target: HelperTarget::Type,
            span: Span::call_site(),
            value_span: Span::call_site(),
        }];
        let context = ExpansionContext::from_attributes(&attributes).expect("explicit facade must resolve");
        assert_eq!(context.facade().to_string(), "framework :: reflect");
    }

    #[test]
    fn renamed_dependency_becomes_an_absolute_path() {
        let context =
            ExpansionContext::from_found_crate(Some(proc_macro_crate::FoundCrate::Name("reflect_runtime".to_owned())))
                .expect("renamed dependency must resolve");
        assert_eq!(context.facade().to_string(), ":: reflect_runtime");
    }

    #[test]
    fn missing_facade_reports_actionable_error() {
        let error = ExpansionContext::from_found_crate(None).expect_err("missing runtime must fail");
        assert!(error.to_string().contains("#[reflect(crate = path)]"));
    }
}
