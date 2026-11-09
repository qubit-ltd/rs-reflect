// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Static-lifetime eligibility analysis for dynamic invocation boundaries.

use crate::ir::GenericBoundIr;
use crate::ir::PathArgumentIr;
use crate::ir::PathArgumentsIr;
use crate::ir::ReturnTypeIr;
use crate::ir::TypeIr;
use crate::ir::TypeKindIr;

/// Returns whether a return declaration carries a non-static borrow.
///
/// # Parameters
///
/// - `return_type`: Parsed method return declaration.
///
/// # Returns
///
/// Returns whether its type contains any non-static lifetime.
#[must_use]
pub(super) fn return_contains_non_static_lifetime(return_type: &ReturnTypeIr) -> bool {
    matches!(return_type, ReturnTypeIr::Type(ty) if type_contains_non_static_lifetime(ty, &[], false))
}

/// Recursively detects path arguments whose lifetime is not explicitly static.
///
/// # Parameters
///
/// - `ty`: Structured type to inspect recursively.
///
/// # Returns
///
/// Returns whether a non-static or unknown lifetime occurs in the type.
#[must_use]
fn type_contains_non_static_lifetime(ty: &TypeIr, bound_lifetimes: &[String], in_function_signature: bool) -> bool {
    match &ty.kind {
        TypeKindIr::Path(path) => path_contains_non_static_lifetime(path, bound_lifetimes, in_function_signature),
        TypeKindIr::Reference { lifetime, element, .. } => {
            let lifetime_is_non_static = match lifetime {
                Some(lifetime) if lifetime == "'static" => false,
                Some(lifetime) => !bound_lifetimes.contains(lifetime),
                None => !in_function_signature,
            };
            lifetime_is_non_static || type_contains_non_static_lifetime(element, bound_lifetimes, in_function_signature)
        }
        TypeKindIr::Tuple(items) => items
            .iter()
            .any(|item| type_contains_non_static_lifetime(item, bound_lifetimes, in_function_signature)),
        TypeKindIr::Slice(element) | TypeKindIr::Array { element, .. } | TypeKindIr::Pointer { element, .. } => {
            type_contains_non_static_lifetime(element, bound_lifetimes, in_function_signature)
        }
        TypeKindIr::BareFunction {
            lifetimes,
            inputs,
            output,
            ..
        } => {
            let mut nested_bound_lifetimes = bound_lifetimes.to_vec();
            nested_bound_lifetimes.extend(lifetimes.iter().cloned());
            inputs
                .iter()
                .any(|input| type_contains_non_static_lifetime(input, &nested_bound_lifetimes, true))
                || output
                    .as_deref()
                    .is_some_and(|ty| type_contains_non_static_lifetime(ty, &nested_bound_lifetimes, true))
        }
        TypeKindIr::TraitObject { bounds, .. } | TypeKindIr::ImplTrait { bounds } => bounds
            .iter()
            .any(|bound| bound_contains_non_static_lifetime(bound, bound_lifetimes, in_function_signature)),
        TypeKindIr::Never => false,
        TypeKindIr::Infer | TypeKindIr::Macro | TypeKindIr::Other => true,
    }
}

/// Recursively detects non-static lifetime arguments nested in one path.
///
/// # Parameters
///
/// - `path`: Structured path and its arguments to inspect.
///
/// # Returns
///
/// Returns whether a non-static or unknown lifetime occurs in the path.
#[must_use]
fn path_contains_non_static_lifetime(
    path: &crate::ir::PathIr,
    bound_lifetimes: &[String],
    in_function_signature: bool,
) -> bool {
    path.qualified_self.as_ref().is_some_and(|qualified| {
        type_contains_non_static_lifetime(&qualified.ty, bound_lifetimes, in_function_signature)
    }) || path.segments.iter().any(|segment| match &segment.arguments {
        PathArgumentsIr::None => false,
        PathArgumentsIr::AngleBracketed(arguments) => arguments.iter().any(|argument| match argument {
            PathArgumentIr::Lifetime(lifetime) => lifetime != "'static" && !bound_lifetimes.contains(lifetime),
            PathArgumentIr::Type(ty) | PathArgumentIr::AssociatedType { ty, .. } => {
                type_contains_non_static_lifetime(ty, bound_lifetimes, in_function_signature)
            }
            PathArgumentIr::Constraint { bounds, .. } => bounds
                .iter()
                .any(|bound| bound_contains_non_static_lifetime(bound, bound_lifetimes, in_function_signature)),
            PathArgumentIr::Other(_) => true,
            PathArgumentIr::Const(_) | PathArgumentIr::AssociatedConst { .. } => false,
        }),
        PathArgumentsIr::Parenthesized { inputs, output } => {
            inputs
                .iter()
                .any(|input| type_contains_non_static_lifetime(input, bound_lifetimes, in_function_signature))
                || output
                    .as_deref()
                    .is_some_and(|ty| type_contains_non_static_lifetime(ty, bound_lifetimes, in_function_signature))
        }
    })
}

/// Returns whether a bound retains a non-static lifetime.
///
/// # Parameters
///
/// - `bound`: Generic bound to inspect.
///
/// # Returns
///
/// Returns whether the bound contains a non-static or unknown lifetime.
#[must_use]
fn bound_contains_non_static_lifetime(
    bound: &GenericBoundIr,
    bound_lifetimes: &[String],
    in_function_signature: bool,
) -> bool {
    match bound {
        GenericBoundIr::Lifetime(lifetime) => lifetime != "'static" && !bound_lifetimes.contains(lifetime),
        GenericBoundIr::Trait { path, lifetimes, .. } => {
            lifetimes
                .iter()
                .any(|lifetime| lifetime != "'static" && !bound_lifetimes.contains(lifetime))
                || path_contains_non_static_lifetime(path, bound_lifetimes, in_function_signature)
        }
        GenericBoundIr::Other(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::quote;

    use super::return_contains_non_static_lifetime;
    use super::type_contains_non_static_lifetime;
    use crate::ir::ReturnTypeIr;

    /// Parses source type tokens for lifetime analysis fixtures.
    ///
    /// # Parameters
    ///
    /// - `tokens`: Type syntax used by the fixture.
    ///
    /// # Returns
    ///
    /// Returns the converted type IR.
    fn parsed_type(tokens: TokenStream) -> crate::ir::TypeIr {
        let ty = syn::parse2(tokens).expect("the fixture type must parse");
        crate::parse::convert_type(&ty)
    }

    #[test]
    fn test_non_static_lifetime_detection_covers_nested_type_forms() {
        let cases = [
            (quote!(Plain), false),
            (quote!(Wrapper<'static>), false),
            (quote!(Wrapper<'a>), true),
            (quote!(<Wrapper<'a> as Iterator>::Item), true),
            (quote!((&'static str, u8)), false),
            (quote!([Wrapper<'a>; 1]), true),
            (quote!(*const Wrapper<'a>), true),
            (quote!(fn() -> Wrapper<'a>), true),
            (quote!(for<'a> fn() -> u8), false),
            (quote!(for<'a> fn(&'a u8)), false),
            (quote!(for<'a> fn() -> &'a u8), false),
            (quote!(for<'a> fn(for<'a> fn(&'a u8))), false),
            (quote!(fn(&u8)), false),
            (quote!(fn(&'env u8)), true),
            (quote!(for<'a> fn(&'env u8)), true),
            (quote!(dyn Send + 'static), false),
            (quote!(dyn Send + 'a), true),
            (quote!(impl Send + 'a), true),
            (quote!(dyn Fn(Wrapper<'a>) -> u8), true),
            (quote!(!), false),
            (quote!(_), true),
            (quote!(fixture!()), true),
        ];

        for (tokens, expected) in cases {
            assert_eq!(
                type_contains_non_static_lifetime(&parsed_type(tokens.clone()), &[], false),
                expected,
                "type fixture: {tokens}",
            );
        }
    }

    #[test]
    fn test_return_lifetime_detection_accepts_default_and_owned_outputs() {
        assert!(
            !return_contains_non_static_lifetime(&ReturnTypeIr::Unit),
            "unit returns do not carry a lifetime"
        );
        assert!(
            !return_contains_non_static_lifetime(&ReturnTypeIr::Type(parsed_type(quote!(String)),)),
            "owned returns do not carry a borrow lifetime"
        );
        assert!(
            return_contains_non_static_lifetime(&ReturnTypeIr::Type(parsed_type(quote!(Wrapper<'a>)),)),
            "a return type with a non-static argument lifetime must be detected"
        );
    }
}
