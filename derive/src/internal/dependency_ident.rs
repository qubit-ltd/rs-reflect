// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Dependency alias identifiers, including aliases that are Rust keywords.

/// Parses a Cargo dependency alias as a Rust identifier.
///
/// # Parameters
///
/// - `name`: Dependency alias returned by the Cargo metadata resolver.
///
/// # Returns
///
/// Returns an identifier suitable for a Rust path.
///
/// # Errors
///
/// Returns a syntax error when the alias cannot be represented as an
/// identifier.
pub(crate) fn dependency_ident(name: &str) -> syn::Result<proc_macro2::Ident> {
    syn::parse_str::<proc_macro2::Ident>(name).or_else(|_| syn::parse_str::<proc_macro2::Ident>(&format!("r#{name}")))
}

#[cfg(test)]
mod tests {
    use super::dependency_ident;

    #[test]
    fn test_parses_regular_and_keyword_dependency_aliases() {
        assert_eq!(
            dependency_ident("renamed_reflect").unwrap().to_string(),
            "renamed_reflect"
        );
        assert_eq!(dependency_ident("type").unwrap().to_string(), "r#type");
        assert!(dependency_ident("bad-name").is_err());
    }
}
