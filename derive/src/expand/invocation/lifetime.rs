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
pub(super) fn return_contains_non_static_lifetime(return_type: &ReturnTypeIr) -> bool {
    matches!(return_type, ReturnTypeIr::Type(ty) if type_contains_non_static_lifetime(ty))
}

/// Recursively detects path arguments whose lifetime is not explicitly static.
fn type_contains_non_static_lifetime(ty: &TypeIr) -> bool {
    match &ty.kind {
        TypeKindIr::Path(path) => path_contains_non_static_lifetime(path),
        TypeKindIr::Reference { .. } => true,
        TypeKindIr::Tuple(items) => items.iter().any(type_contains_non_static_lifetime),
        TypeKindIr::Slice(element) | TypeKindIr::Array { element, .. } | TypeKindIr::Pointer { element, .. } => {
            type_contains_non_static_lifetime(element)
        }
        TypeKindIr::BareFunction {
            lifetimes,
            inputs,
            output,
            ..
        } => {
            lifetimes.iter().any(|lifetime| lifetime != "'static")
                || inputs.iter().any(type_contains_non_static_lifetime)
                || output.as_deref().is_some_and(type_contains_non_static_lifetime)
        }
        TypeKindIr::TraitObject { bounds, .. } | TypeKindIr::ImplTrait { bounds } => {
            bounds.iter().any(bound_contains_non_static_lifetime)
        }
        TypeKindIr::Never => false,
        TypeKindIr::Infer | TypeKindIr::Macro | TypeKindIr::Other => true,
    }
}

/// Recursively detects non-static lifetime arguments nested in one path.
fn path_contains_non_static_lifetime(path: &crate::ir::PathIr) -> bool {
    path.qualified_self
        .as_ref()
        .is_some_and(|qualified| type_contains_non_static_lifetime(&qualified.ty))
        || path.segments.iter().any(|segment| match &segment.arguments {
            PathArgumentsIr::None => false,
            PathArgumentsIr::AngleBracketed(arguments) => arguments.iter().any(|argument| match argument {
                PathArgumentIr::Lifetime(lifetime) => lifetime != "'static",
                PathArgumentIr::Type(ty) | PathArgumentIr::AssociatedType { ty, .. } => {
                    type_contains_non_static_lifetime(ty)
                }
                PathArgumentIr::Constraint { bounds, .. } => bounds.iter().any(bound_contains_non_static_lifetime),
                PathArgumentIr::Other(_) => true,
                PathArgumentIr::Const(_) | PathArgumentIr::AssociatedConst { .. } => false,
            }),
            PathArgumentsIr::Parenthesized { inputs, output } => {
                inputs.iter().any(type_contains_non_static_lifetime)
                    || output.as_deref().is_some_and(type_contains_non_static_lifetime)
            }
        })
}

/// Returns whether a bound retains a non-static lifetime.
fn bound_contains_non_static_lifetime(bound: &GenericBoundIr) -> bool {
    match bound {
        GenericBoundIr::Lifetime(lifetime) => lifetime != "'static",
        GenericBoundIr::Trait { path, lifetimes, .. } => {
            lifetimes.iter().any(|lifetime| lifetime != "'static") || path_contains_non_static_lifetime(path)
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
            (quote!((&'static str, u8)), true),
            (quote!([Wrapper<'a>; 1]), true),
            (quote!(*const Wrapper<'a>), true),
            (quote!(fn() -> Wrapper<'a>), true),
            (quote!(for<'a> fn() -> u8), true),
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
                type_contains_non_static_lifetime(&parsed_type(tokens.clone())),
                expected,
                "type fixture: {tokens}",
            );
        }
    }

    #[test]
    fn test_return_lifetime_detection_accepts_default_and_owned_outputs() {
        assert!(!return_contains_non_static_lifetime(&ReturnTypeIr::Unit));
        assert!(!return_contains_non_static_lifetime(&ReturnTypeIr::Type(parsed_type(
            quote!(String)
        ),)));
        assert!(return_contains_non_static_lifetime(&ReturnTypeIr::Type(parsed_type(
            quote!(Wrapper<'a>)
        ),)));
    }
}
