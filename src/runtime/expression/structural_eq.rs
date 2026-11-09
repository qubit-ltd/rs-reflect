// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::data::ConstExpression;
use super::data::ExpressionData;
use super::data::GenericExpressionArgument;
use super::data::PredicateData;
use super::data::TraitExpressionData;

/// Compares two retained source expressions recursively.
///
/// # Parameters
///
/// - `left`: The first source expression.
/// - `right`: The second source expression.
///
/// # Returns
///
/// Returns whether both expressions retain the same source structure.
pub(super) fn expression_eq(left: &ExpressionData, right: &ExpressionData) -> bool {
    match (left, right) {
        (
            ExpressionData::Named {
                path: a, arguments: aa, ..
            },
            ExpressionData::Named {
                path: b, arguments: ba, ..
            },
        ) => a == b && arguments_eq(aa, ba),
        (ExpressionData::Parameter(a), ExpressionData::Parameter(b)) => a.name == b.name,
        (ExpressionData::Parameter(a), ExpressionData::ParameterName(b))
        | (ExpressionData::ParameterName(b), ExpressionData::Parameter(a)) => a.name == *b,
        (ExpressionData::ParameterName(a), ExpressionData::ParameterName(b)) => a == b,
        (ExpressionData::SelfType, ExpressionData::SelfType) | (ExpressionData::Never, ExpressionData::Never) => true,
        (
            ExpressionData::AssociatedType {
                self_type: a,
                trait_: at,
                name: an,
                arguments: aa,
            },
            ExpressionData::AssociatedType {
                self_type: b,
                trait_: bt,
                name: bn,
                arguments: ba,
            },
        ) => expression_eq(a, b) && option_eq(*at, *bt, trait_eq) && an == bn && arguments_eq(aa, ba),
        (
            ExpressionData::Reference {
                lifetime: al,
                mutable: am,
                target: at,
            },
            ExpressionData::Reference {
                lifetime: bl,
                mutable: bm,
                target: bt,
            },
        ) => al == bl && am == bm && expression_eq(at, bt),
        (
            ExpressionData::RawPointer {
                mutable: am,
                target: at,
            },
            ExpressionData::RawPointer {
                mutable: bm,
                target: bt,
            },
        ) => am == bm && expression_eq(at, bt),
        (ExpressionData::Slice(a), ExpressionData::Slice(b)) => expression_eq(a, b),
        (
            ExpressionData::Array {
                element: ae,
                length: al,
            },
            ExpressionData::Array {
                element: be,
                length: bl,
            },
        ) => al == bl && expression_eq(ae, be),
        (ExpressionData::Tuple(a), ExpressionData::Tuple(b)) => sequence_eq(a, b, |x, y| expression_eq(x, y)),
        (ExpressionData::FunctionPointer(a), ExpressionData::FunctionPointer(b)) => {
            a.lifetimes == b.lifetimes
                && sequence_eq(a.arguments, b.arguments, |x, y| expression_eq(x, y))
                && option_eq(a.result, b.result, expression_eq)
                && a.unsafe_ == b.unsafe_
                && a.abi == b.abi
                && a.variadic == b.variadic
        }
        (ExpressionData::TraitObject(a), ExpressionData::TraitObject(b)) => {
            a.lifetime == b.lifetime && sequence_eq(a.traits, b.traits, trait_eq)
        }
        (ExpressionData::Opaque(a), ExpressionData::Opaque(b)) => sequence_eq(a.bounds, b.bounds, predicate_eq),
        _ => false,
    }
}

/// Compares trait paths, arguments, and lifetime bounds.
///
/// # Parameters
///
/// - `a`: The first trait expression.
/// - `b`: The second trait expression.
///
/// # Returns
///
/// Returns whether the trait expressions match structurally.
fn trait_eq(a: &TraitExpressionData, b: &TraitExpressionData) -> bool {
    a.path == b.path && arguments_eq(a.arguments, b.arguments) && a.lifetimes == b.lifetimes
}

/// Compares ordered generic argument sequences.
///
/// # Parameters
///
/// - `a`: The first argument sequence.
/// - `b`: The second argument sequence.
///
/// # Returns
///
/// Returns whether both sequences contain the same arguments in order.
fn arguments_eq(a: &[GenericExpressionArgument], b: &[GenericExpressionArgument]) -> bool {
    sequence_eq(a, b, argument_eq)
}

/// Compares one generic argument and all of its retained payload.
///
/// # Parameters
///
/// - `a`: The first generic argument.
/// - `b`: The second generic argument.
///
/// # Returns
///
/// Returns whether both arguments have matching variants and payloads.
fn argument_eq(a: &GenericExpressionArgument, b: &GenericExpressionArgument) -> bool {
    match (a, b) {
        (GenericExpressionArgument::Type(a), GenericExpressionArgument::Type(b)) => expression_eq(a, b),
        (GenericExpressionArgument::Lifetime(a), GenericExpressionArgument::Lifetime(b)) => a == b,
        (GenericExpressionArgument::Const(a), GenericExpressionArgument::Const(b)) => a == b,
        (
            GenericExpressionArgument::AssociatedType { name: an, value: av },
            GenericExpressionArgument::AssociatedType { name: bn, value: bv },
        ) => an == bn && expression_eq(av, bv),
        (
            GenericExpressionArgument::AssociatedTypeBound { name: an, bounds: ab },
            GenericExpressionArgument::AssociatedTypeBound { name: bn, bounds: bb },
        ) => an == bn && sequence_eq(ab, bb, predicate_eq),
        _ => false,
    }
}

/// Compares predicate variants and their complete source payloads.
///
/// # Parameters
///
/// - `a`: The first predicate.
/// - `b`: The second predicate.
///
/// # Returns
///
/// Returns whether both predicates match structurally.
fn predicate_eq(a: &PredicateData, b: &PredicateData) -> bool {
    match (a, b) {
        (
            PredicateData::TraitBound {
                ty: at,
                trait_: ar,
                modifier: am,
            },
            PredicateData::TraitBound {
                ty: bt,
                trait_: br,
                modifier: bm,
            },
        ) => expression_eq(at, bt) && trait_eq(ar, br) && am == bm,
        (
            PredicateData::LifetimeOutlives {
                lifetime: al,
                outlives: ao,
            },
            PredicateData::LifetimeOutlives {
                lifetime: bl,
                outlives: bo,
            },
        ) => al == bl && ao == bo,
        (
            PredicateData::TypeOutlives { ty: at, lifetime: al },
            PredicateData::TypeOutlives { ty: bt, lifetime: bl },
        ) => expression_eq(at, bt) && al == bl,
        (PredicateData::TypeEquality { left: al, right: ar }, PredicateData::TypeEquality { left: bl, right: br }) => {
            expression_eq(al, bl) && expression_eq(ar, br)
        }
        (PredicateData::ConstEvaluatable(a), PredicateData::ConstEvaluatable(b)) => const_eq(a, b),
        _ => false,
    }
}

/// Compares retained const-expression facts.
///
/// # Parameters
///
/// - `a`: The first const expression.
/// - `b`: The second const expression.
///
/// # Returns
///
/// Returns whether both const expressions have the same variant and value.
fn const_eq(a: &ConstExpression, b: &ConstExpression) -> bool {
    a == b
}

/// Compares two optional values using the supplied equality operation.
///
/// # Type Parameters
///
/// - `T`: The type stored by each option.
///
/// # Parameters
///
/// - `a`: The first optional value.
/// - `b`: The second optional value.
/// - `eq`: The equality operation for present values.
///
/// # Returns
///
/// Returns whether both options are absent or their values compare equal.
fn option_eq<T>(a: Option<T>, b: Option<T>, eq: impl FnOnce(T, T) -> bool) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

/// Compares two ordered slices using the supplied element comparison.
///
/// # Type Parameters
///
/// - `T`: The type stored by each slice.
///
/// # Parameters
///
/// - `a`: The first slice.
/// - `b`: The second slice.
/// - `eq`: The equality operation for corresponding elements.
///
/// # Returns
///
/// Returns whether both slices have equal lengths and matching elements.
fn sequence_eq<T>(a: &[T], b: &[T], mut eq: impl FnMut(&T, &T) -> bool) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| eq(a, b))
}
