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

use crate::ir::GenericKindIr;
use crate::ir::MethodIr;
use crate::ir::ReceiverKindIr;
use crate::ir::ReturnTypeIr;
use crate::ir::SpecializationIr;
use crate::ir::SpecializationValueIr;
use crate::ir::TypeIr;

/// Builds replacement tokens for one validated type or const specialization.
///
/// # Parameters
///
/// - `specialization`: Validated named generic bindings.
///
/// # Returns
///
/// Returns generic identifiers paired with their concrete syntax tokens.
#[must_use]
pub(super) fn specialization_replacements(specialization: &SpecializationIr) -> Vec<(Ident, TokenStream)> {
    specialization
        .bindings
        .iter()
        .map(|binding| {
            let tokens = match &binding.value {
                SpecializationValueIr::Type(ty) => ty.tokens.clone(),
                SpecializationValueIr::Const(tokens) | SpecializationValueIr::AmbiguousPath(tokens) => tokens.clone(),
            };
            (Ident::new(&binding.name, binding.span), tokens)
        })
        .collect()
}

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

/// Rebuilds structural type IR after specialization changes a root path.
///
/// # Parameters
///
/// - `ty`: Type IR to rewrite and rebuild.
/// - `replacements`: Generic identifiers and concrete type or const tokens.
///
/// # Panics
///
/// Panics if validated specialization tokens no longer parse as Rust type
/// syntax.
fn reparse_substituted_type(ty: &mut TypeIr, replacements: &[(Ident, TokenStream)]) {
    let tokens = substitute_type_syntax(&ty.tokens, replacements);
    let parsed: syn::Type =
        syn::parse2(tokens).expect("validated specialization must retain valid associated-item type syntax");
    *ty = crate::parse::convert_type(&parsed);
}

/// Recursively substitutes one type and every nested type retained by its IR.
///
/// # Parameters
///
/// - `ty`: Structured type IR to rewrite.
/// - `replacements`: Generic identifiers and concrete type or const tokens.
pub(super) fn substitute_type_tokens(ty: &mut TypeIr, replacements: &[(Ident, TokenStream)]) {
    reparse_substituted_type(ty, replacements);
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

/// Emits concrete arguments for one validated method specialization.
///
/// # Parameters
///
/// - `specialization`: Validated named generic bindings.
/// - `generics`: Generic parameters declared by the method.
/// - `facade`: Runtime facade path used in generated expressions.
///
/// # Returns
///
/// Returns generated runtime generic-argument tokens.
#[must_use]
pub(super) fn specialization_arguments(
    specialization: &SpecializationIr,
    generics: &crate::ir::GenericsIr,
    facade: &TokenStream,
) -> TokenStream {
    let environment = crate::expand::generic_environment::GenericEnvironment::from_generics(generics);
    let arguments = generics.params.iter().filter_map(|parameter| {
        if parameter.kind == GenericKindIr::Lifetime {
            return None;
        }
        let binding = specialization
            .bindings
            .iter()
            .find(|binding| binding.name == parameter.name)?;
        match (parameter.kind, &binding.value) {
            (GenericKindIr::Type, SpecializationValueIr::Type(ty)) => {
                let expression = crate::expand::traits::type_expression(ty, &environment, facade);
                Some(quote!(#facade::__private::codegen_v3::expression::GenericArgument::Type(#expression)))
            }
            (GenericKindIr::Type, SpecializationValueIr::AmbiguousPath(tokens)) => Some(quote!(
                #facade::__private::codegen_v3::expression::GenericArgument::Type(
                    #facade::__private::codegen_v3::expression::TypeExpression::Concrete(
                        #facade::__private::codegen_v3::expression::concrete(
                            vec![stringify!(#tokens).into()].into_boxed_slice(),
                            vec![].into_boxed_slice(),
                            #facade::__private::codegen_v3::expression::DiagnosticText::from(stringify!(#tokens)),
                        ),
                    ),
                )
            )),
            (GenericKindIr::Const, value) => {
                let tokens = match value {
                    SpecializationValueIr::Const(tokens) | SpecializationValueIr::AmbiguousPath(tokens) => tokens,
                    SpecializationValueIr::Type(_) => return None,
                };
                let declared_type = parameter.const_type.as_ref()?.source.as_str();
                let declared_type_literal = syn::LitStr::new(declared_type, parameter.span);
                Some(quote!(
                    #facade::__private::codegen_v3::expression::GenericArgument::Const(
                        #facade::__private::codegen_v3::expression::ConstGenericArgument::new(
                            #facade::__private::codegen_v3::expression::TypeExpression::Concrete(
                                #facade::__private::codegen_v3::expression::concrete(
                                    vec![#declared_type_literal.into()].into_boxed_slice(),
                                    vec![].into_boxed_slice(),
                                    #facade::__private::codegen_v3::expression::DiagnosticText::from(
                                        #declared_type_literal,
                                    ),
                                ),
                            ),
                            #facade::__private::codegen_v3::expression::const_path([
                                stringify!(#tokens),
                            ]),
                            stringify!(#tokens),
                        ),
                    )
                ))
            }
            _ => None,
        }
    });
    quote!(::std::vec![#(#arguments),*].into_boxed_slice())
}

/// Materializes one method specialization as concrete method IR and call
/// arguments. The ordinary invocation analyzer and emitter can then make the
/// same availability decision as they do for a non-generic method.
///
/// # Parameters
///
/// - `method`: Generic method declaration to specialize.
/// - `specialization`: Validated named generic bindings.
///
/// # Returns
///
/// Returns concrete method IR and call arguments, or `None` for incomplete
/// bindings.
#[must_use]
pub(super) fn specialized_method(
    method: &MethodIr,
    specialization: &SpecializationIr,
) -> Option<(MethodIr, Vec<TokenStream>)> {
    let generic_arguments: Vec<_> = method
        .generics
        .params
        .iter()
        .map(|parameter| match parameter.kind {
            GenericKindIr::Type => specialization_type_argument(specialization, &parameter.name),
            GenericKindIr::Const => specialization_const_argument(specialization, &parameter.name),
            GenericKindIr::Lifetime => None,
        })
        .collect::<Option<_>>()?;
    let replacements = specialization_replacements(specialization);
    let mut concrete = method.clone();
    if let Some(receiver) = &mut concrete.receiver {
        substitute_type_tokens(&mut receiver.ty, &replacements);
    }
    for parameter in &mut concrete.parameters {
        substitute_type_tokens(&mut parameter.ty, &replacements);
    }
    if let ReturnTypeIr::Type(ty) = &mut concrete.return_type {
        substitute_type_tokens(ty, &replacements);
    }
    concrete
        .generics
        .params
        .retain(|parameter| parameter.kind == GenericKindIr::Lifetime);
    concrete.specializations.clear();
    Some((concrete, generic_arguments))
}

/// Resolves one named type argument from a validated specialization.
///
/// # Parameters
///
/// - `specialization`: Validated named generic bindings.
/// - `name`: Type parameter name to resolve.
///
/// # Returns
///
/// Returns the corresponding type tokens, or `None` for a missing/non-type
/// binding.
#[must_use]
fn specialization_type_argument(specialization: &SpecializationIr, name: &str) -> Option<TokenStream> {
    match &specialization
        .bindings
        .iter()
        .find(|binding| binding.name == name)?
        .value
    {
        SpecializationValueIr::Type(ty) => Some(ty.tokens.clone()),
        SpecializationValueIr::AmbiguousPath(tokens) => Some(tokens.clone()),
        SpecializationValueIr::Const(_) => None,
    }
}

/// Resolves one named const argument from a validated specialization.
///
/// # Parameters
///
/// - `specialization`: Validated named generic bindings.
/// - `name`: Const parameter name to resolve.
///
/// # Returns
///
/// Returns the corresponding const tokens, or `None` for a missing/non-const
/// binding.
#[must_use]
fn specialization_const_argument(specialization: &SpecializationIr, name: &str) -> Option<TokenStream> {
    match &specialization
        .bindings
        .iter()
        .find(|binding| binding.name == name)?
        .value
    {
        SpecializationValueIr::Const(tokens) | SpecializationValueIr::AmbiguousPath(tokens) => Some(tokens.clone()),
        SpecializationValueIr::Type(_) => None,
    }
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
