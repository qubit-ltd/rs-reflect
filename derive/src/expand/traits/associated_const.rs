// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Associated-constant shape analysis used by trait expansion.

use crate::ir::GenericBoundIr;
use crate::ir::PathArgumentIr;
use crate::ir::PathArgumentsIr;
use crate::ir::TypeIr;
use crate::ir::TypeKindIr;

/// Proves that an associated constant's type has no unresolved lifetime.
pub(super) fn has_proven_static_shape(ty: &TypeIr) -> bool {
    has_proven_static_shape_in(ty, &std::collections::HashSet::new(), false)
}

fn has_proven_static_shape_in(
    ty: &TypeIr,
    bound_lifetimes: &std::collections::HashSet<String>,
    callable_elision: bool,
) -> bool {
    match &ty.kind {
        TypeKindIr::Path(path) => {
            path.qualified_self.is_none()
                && !path.segments.iter().any(|segment| segment.name == "Self")
                && path.segments.iter().all(|segment| {
                    match &segment.arguments {
                        PathArgumentsIr::None => true,
                        PathArgumentsIr::AngleBracketed(arguments) => {
                            arguments.iter().all(|argument| match argument {
                                PathArgumentIr::Lifetime(lifetime) => {
                                    lifetime == "'static"
                                        || bound_lifetimes.contains(lifetime)
                                }
                                PathArgumentIr::Type(ty)
                                | PathArgumentIr::AssociatedType {
                                    ty, ..
                                } => has_proven_static_shape_in(
                                    ty,
                                    bound_lifetimes,
                                    callable_elision,
                                ),
                                PathArgumentIr::Const(_) => true,
                                PathArgumentIr::AssociatedConst { .. }
                                | PathArgumentIr::Constraint { .. }
                                | PathArgumentIr::Other(_) => false,
                            })
                        }
                        PathArgumentsIr::Parenthesized { inputs, output } => {
                            inputs.iter().all(|input| {
                                has_proven_static_shape_in(
                                    input,
                                    bound_lifetimes,
                                    true,
                                )
                            }) && output.as_deref().is_none_or(|output| {
                                has_proven_static_shape_in(
                                    output,
                                    bound_lifetimes,
                                    true,
                                )
                            })
                        }
                    }
                })
        }
        TypeKindIr::Reference {
            lifetime, element, ..
        } => {
            (lifetime.as_deref().is_some_and(|lifetime| {
                lifetime == "'static" || bound_lifetimes.contains(lifetime)
            }) || (lifetime.is_none() && callable_elision))
                && has_proven_static_shape_in(
                    element,
                    bound_lifetimes,
                    callable_elision,
                )
        }
        TypeKindIr::Pointer { element, .. }
        | TypeKindIr::Slice(element)
        | TypeKindIr::Array { element, .. } => has_proven_static_shape_in(
            element,
            bound_lifetimes,
            callable_elision,
        ),
        TypeKindIr::Tuple(elements) => elements.iter().all(|element| {
            has_proven_static_shape_in(
                element,
                bound_lifetimes,
                callable_elision,
            )
        }),
        TypeKindIr::BareFunction {
            lifetimes,
            inputs,
            output,
            ..
        } => {
            let mut function_lifetimes = bound_lifetimes.clone();
            function_lifetimes.extend(lifetimes.iter().cloned());
            inputs.iter().all(|input| {
                has_proven_static_shape_in(input, &function_lifetimes, true)
            }) && output.as_deref().is_none_or(|output| {
                has_proven_static_shape_in(output, &function_lifetimes, true)
            })
        }
        TypeKindIr::TraitObject { bounds, .. }
        | TypeKindIr::ImplTrait { bounds } => {
            bounds.iter().all(|bound| match bound {
                GenericBoundIr::Lifetime(lifetime) => {
                    lifetime == "'static" || bound_lifetimes.contains(lifetime)
                }
                GenericBoundIr::Trait {
                    path, lifetimes, ..
                } => {
                    let mut trait_lifetimes = bound_lifetimes.clone();
                    trait_lifetimes.extend(lifetimes.iter().cloned());
                    path.segments.iter().all(|segment| {
                        match &segment.arguments {
                            PathArgumentsIr::None => true,
                            PathArgumentsIr::AngleBracketed(arguments) => {
                                arguments.iter().all(
                                    |argument| match argument {
                                        PathArgumentIr::Lifetime(lifetime) => {
                                            lifetime == "'static"
                                                || trait_lifetimes
                                                    .contains(lifetime)
                                        }
                                        PathArgumentIr::Type(ty)
                                        | PathArgumentIr::AssociatedType {
                                            ty,
                                            ..
                                        } => has_proven_static_shape_in(
                                            ty,
                                            &trait_lifetimes,
                                            callable_elision,
                                        ),
                                        PathArgumentIr::Const(_) => true,
                                        PathArgumentIr::AssociatedConst {
                                            ..
                                        }
                                        | PathArgumentIr::Constraint {
                                            ..
                                        }
                                        | PathArgumentIr::Other(_) => false,
                                    },
                                )
                            }
                            PathArgumentsIr::Parenthesized {
                                inputs,
                                output,
                            } => {
                                inputs.iter().all(|input| {
                                    has_proven_static_shape_in(
                                        input,
                                        &trait_lifetimes,
                                        true,
                                    )
                                }) && output.as_deref().is_none_or(|output| {
                                    has_proven_static_shape_in(
                                        output,
                                        &trait_lifetimes,
                                        true,
                                    )
                                })
                            }
                        }
                    })
                }
                GenericBoundIr::Other(_) => false,
            })
        }
        TypeKindIr::Never => true,
        TypeKindIr::Infer | TypeKindIr::Macro | TypeKindIr::Other => false,
    }
}

#[cfg(test)]
mod tests {
    use super::has_proven_static_shape;

    #[test]
    fn helper_is_available_as_a_narrow_parent_api() {
        let ty = crate::parse::convert_type(
            &syn::parse_str::<syn::Type>("u32")
                .expect("a primitive type must parse"),
        );
        assert!(has_proven_static_shape(&ty));
    }

    #[test]
    fn rejects_unresolved_associated_constant_lifetimes() {
        let borrowed = crate::parse::convert_type(
            &syn::parse_str::<syn::Type>("&'a u32")
                .expect("a reference must parse"),
        );
        let static_borrowed = crate::parse::convert_type(
            &syn::parse_str::<syn::Type>("&'static u32")
                .expect("a reference must parse"),
        );
        assert!(!has_proven_static_shape(&borrowed));
        assert!(has_proven_static_shape(&static_borrowed));
    }
}
