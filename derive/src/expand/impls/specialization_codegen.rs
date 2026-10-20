// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Generic implementation specialization token generation.

use proc_macro2::Ident;
use proc_macro2::TokenStream;
use quote::quote;

use crate::ir::ReceiverKindIr;

/// Substitutes generic symbols only where the Rust AST identifies a type or
/// const expression path. Member names, labels, patterns, and unrelated token
/// identifiers are never rewritten.
///
/// # Parameters
///
/// - `tokens`: Type syntax containing generic references.
/// - `replacements`: Generic identifiers and concrete type or const tokens.
///
/// # Returns
///
/// Returns the rewritten type syntax.
#[must_use]
pub(super) fn substitute_type_syntax(tokens: &TokenStream, replacements: &[(Ident, TokenStream)]) -> TokenStream {
    super::internal::generic_substituter::substitute_type_syntax(tokens, replacements)
}

/// Returns a non-core explicit receiver type that requires a registered
/// `ReceiverAdapter` capability for dynamic invocation.
///
/// # Parameters
///
/// - `receiver`: Explicit receiver declaration to classify.
/// - `target`: Concrete impl target type tokens.
///
/// # Returns
///
/// Returns the substituted extension receiver type, or `None` for core
/// receivers.
#[must_use]
pub(super) fn typed_extension_receiver_type(
    receiver: &crate::ir::ReceiverIr,
    target: &TokenStream,
) -> Option<TokenStream> {
    if receiver.kind != ReceiverKindIr::Typed
        || crate::expand::invocation::analysis::typed_pinned_receiver_mutable(receiver).is_some()
        || crate::expand::invocation::analysis::typed_owned_receiver_type(receiver, &quote!(Self)).is_some()
    {
        return None;
    }
    let self_identifier = Ident::new("Self", receiver.span);
    Some(substitute_type_syntax(
        &receiver.ty.tokens,
        &[(self_identifier, target.clone())],
    ))
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::quote;

    use super::substitute_type_syntax;

    #[test]
    fn test_substitute_type_syntax_rewrites_only_generic_type_and_const_paths() {
        let replacements = [(quote!(T), quote!(u8)), (quote!(N), quote!(4))].map(|(name, value)| {
            (
                syn::parse2(name).expect("the replacement name must be an identifier"),
                value,
            )
        });
        let cases: [(TokenStream, TokenStream); 6] = [
            (quote!(Vec<T>), quote!(Vec<u8>)),
            (quote!(<T as Iterator>::Item), quote!(<u8 as Iterator>::Item)),
            (quote!(Option<Result<T, Vec<T>>>), quote!(Option<Result<u8, Vec<u8>>>)),
            (quote!([T; N + 1]), quote!([u8; 4 + 1])),
            (quote!(Wrapper<Unmatched>), quote!(Wrapper<Unmatched>)),
            (quote!(T::T), quote!(u8::T)),
        ];

        for (input, expected) in cases {
            let actual = substitute_type_syntax(&input, &replacements);
            let actual: syn::Type = syn::parse2(actual).expect("substitution must retain valid type syntax");
            let expected: syn::Type = syn::parse2(expected).expect("the expected result must be valid type syntax");
            assert_eq!(actual, expected, "input: {input}");
        }
    }
}
