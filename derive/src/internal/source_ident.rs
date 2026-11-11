// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Reconstructs identifiers from names retained from parsed source syntax.

/// Rebuilds a source identifier while preserving its raw spelling and span.
///
/// # Parameters
///
/// - `name`: Identifier spelling obtained from a successfully parsed source
///   identifier, including any `r#` prefix.
/// - `span`: Source span retained alongside the identifier spelling.
///
/// # Returns
///
/// Returns the identifier represented by `name` with `span` attached.
///
/// # Input invariants
///
/// The name must come from an identifier already accepted by `syn`; this
/// helper is not an arbitrary string validator.
pub(crate) fn source_ident(name: &str, span: proc_macro2::Span) -> proc_macro2::Ident {
    match name.strip_prefix("r#") {
        Some(raw) => proc_macro2::Ident::new_raw(raw, span),
        None => proc_macro2::Ident::new(name, span),
    }
}

#[cfg(test)]
mod tests {
    use super::source_ident;

    #[test]
    fn test_reconstructs_ordinary_and_raw_identifier_spellings() {
        for name in ["T", "r#type", "r#const"] {
            let identifier = source_ident(name, proc_macro2::Span::call_site());
            assert_eq!(identifier.to_string(), name);
            let parsed: syn::Ident =
                syn::parse_str(&identifier.to_string()).expect("rebuilt identifier parses as source syntax");
            assert_eq!(parsed.to_string(), name);
        }
    }
}
