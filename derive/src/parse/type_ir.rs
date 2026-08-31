// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Conversion from `syn` types and paths into semantic IR.

use quote::ToTokens;
use syn::Path;
use syn::PathArguments;
use syn::ReturnType;
use syn::TraitBoundModifier;
use syn::Type;
use syn::TypeParamBound;
use syn::spanned::Spanned;

use crate::ir::GenericBoundIr;
use crate::ir::PathArgumentIr;
use crate::ir::PathArgumentsIr;
use crate::ir::PathIr;
use crate::ir::PathSegmentIr;
use crate::ir::QualifiedSelfIr;
use crate::ir::TraitBoundModifierIr;
use crate::ir::TypeIr;
use crate::ir::TypeKindIr;

/// Converts a Rust path while retaining source tokens and structured segments.
pub(super) fn convert_path(path: &Path) -> PathIr {
    let tokens = path.to_token_stream();
    PathIr {
        source: tokens.to_string(),
        segments: path
            .segments
            .iter()
            .map(|segment| PathSegmentIr {
                name: segment.ident.to_string(),
                arguments: convert_path_arguments(&segment.arguments),
            })
            .collect(),
        leading_colon: path.leading_colon.is_some(),
        qualified_self: None,
        tokens,
        span: path.span(),
    }
}

/// Converts a `syn::Type` at the parser boundary so later stages never store
/// it.
pub(crate) fn convert_type(ty: &Type) -> TypeIr {
    let tokens = ty.to_token_stream();
    let kind = match ty {
        Type::Path(path) => {
            let mut converted = convert_path(&path.path);
            converted.qualified_self = path.qself.as_ref().map(|qualified| QualifiedSelfIr {
                ty: Box::new(convert_type(&qualified.ty)),
                position: qualified.position,
                has_as: qualified.as_token.is_some(),
            });
            TypeKindIr::Path(converted)
        }
        Type::Reference(reference) => TypeKindIr::Reference {
            lifetime: reference
                .lifetime
                .as_ref()
                .map(ToTokens::to_token_stream)
                .map(|value| value.to_string()),
            mutable: reference.mutability.is_some(),
            element: Box::new(convert_type(&reference.elem)),
        },
        Type::Tuple(tuple) => TypeKindIr::Tuple(tuple.elems.iter().map(convert_type).collect()),
        Type::Slice(slice) => TypeKindIr::Slice(Box::new(convert_type(&slice.elem))),
        Type::Array(array) => TypeKindIr::Array {
            element: Box::new(convert_type(&array.elem)),
            length: array.len.to_token_stream(),
        },
        Type::Ptr(pointer) => TypeKindIr::Pointer {
            mutable: pointer.mutability.is_some(),
            element: Box::new(convert_type(&pointer.elem)),
        },
        Type::BareFn(function) => TypeKindIr::BareFunction {
            lifetimes: function
                .lifetimes
                .iter()
                .flat_map(|lifetimes| lifetimes.lifetimes.iter())
                .filter_map(|parameter| match parameter {
                    syn::GenericParam::Lifetime(lifetime) => {
                        Some(lifetime.lifetime.to_token_stream().to_string())
                    }
                    _ => None,
                })
                .collect(),
            inputs: function
                .inputs
                .iter()
                .map(|input| convert_type(&input.ty))
                .collect(),
            output: match &function.output {
                ReturnType::Default => None,
                ReturnType::Type(_, ty) => Some(Box::new(convert_type(ty))),
            },
            is_unsafe: function.unsafety.is_some(),
            abi: function.abi.as_ref().map(|abi| {
                abi.name
                    .as_ref()
                    .map_or_else(|| "C".to_owned(), syn::LitStr::value)
            }),
            is_variadic: function.variadic.is_some(),
        },
        Type::TraitObject(object) => TypeKindIr::TraitObject {
            bounds: object.bounds.iter().map(convert_bound).collect(),
            has_dyn: object.dyn_token.is_some(),
        },
        Type::ImplTrait(opaque) => TypeKindIr::ImplTrait {
            bounds: opaque.bounds.iter().map(convert_bound).collect(),
        },
        Type::Never(_) => TypeKindIr::Never,
        Type::Infer(_) => TypeKindIr::Infer,
        Type::Macro(_) => TypeKindIr::Macro,
        Type::Group(group) => convert_type(&group.elem).kind,
        Type::Paren(paren) => convert_type(&paren.elem).kind,
        _ => TypeKindIr::Other,
    };
    TypeIr {
        source: tokens.to_string(),
        tokens,
        kind,
        span: ty.span(),
    }
}

/// Converts all generic arguments attached to one path segment.
fn convert_path_arguments(arguments: &PathArguments) -> PathArgumentsIr {
    match arguments {
        PathArguments::None => PathArgumentsIr::None,
        PathArguments::AngleBracketed(arguments) => PathArgumentsIr::AngleBracketed(
            arguments
                .args
                .iter()
                .map(|argument| match argument {
                    syn::GenericArgument::Lifetime(lifetime) => {
                        PathArgumentIr::Lifetime(lifetime.to_token_stream().to_string())
                    }
                    syn::GenericArgument::Type(ty) => PathArgumentIr::Type(convert_type(ty)),
                    syn::GenericArgument::Const(value) => {
                        PathArgumentIr::Const(value.to_token_stream())
                    }
                    syn::GenericArgument::AssocType(binding) => PathArgumentIr::AssociatedType {
                        name: binding.ident.to_string(),
                        ty: convert_type(&binding.ty),
                    },
                    syn::GenericArgument::AssocConst(binding) => PathArgumentIr::AssociatedConst {
                        name: binding.ident.to_string(),
                        value: binding.value.to_token_stream(),
                    },
                    syn::GenericArgument::Constraint(constraint) => PathArgumentIr::Constraint {
                        name: constraint.ident.to_string(),
                        bounds: constraint.bounds.iter().map(convert_bound).collect(),
                    },
                    _ => PathArgumentIr::Other(argument.to_token_stream()),
                })
                .collect(),
        ),
        PathArguments::Parenthesized(arguments) => PathArgumentsIr::Parenthesized {
            inputs: arguments.inputs.iter().map(convert_type).collect(),
            output: match &arguments.output {
                ReturnType::Default => None,
                ReturnType::Type(_, output) => Some(Box::new(convert_type(output))),
            },
        },
    }
}

/// Converts a lifetime or trait bound into semantic IR.
pub(super) fn convert_bound(bound: &TypeParamBound) -> GenericBoundIr {
    match bound {
        TypeParamBound::Lifetime(lifetime) => {
            GenericBoundIr::Lifetime(lifetime.to_token_stream().to_string())
        }
        TypeParamBound::Trait(trait_bound) => GenericBoundIr::Trait {
            path: convert_path(&trait_bound.path),
            modifier: match trait_bound.modifier {
                TraitBoundModifier::None => TraitBoundModifierIr::None,
                TraitBoundModifier::Maybe(_) => TraitBoundModifierIr::Maybe,
            },
            lifetimes: trait_bound
                .lifetimes
                .iter()
                .flat_map(|lifetimes| lifetimes.lifetimes.iter())
                .filter_map(|parameter| match parameter {
                    syn::GenericParam::Lifetime(lifetime) => {
                        Some(lifetime.lifetime.to_token_stream().to_string())
                    }
                    _ => None,
                })
                .collect(),
        },
        _ => GenericBoundIr::Other(bound.to_token_stream()),
    }
}
