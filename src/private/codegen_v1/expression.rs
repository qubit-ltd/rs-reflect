// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Checked expression factories consumed by generated code.

use crate::expression::ArrayTypeExpression;
use crate::expression::ConcreteTypeExpression;
use crate::expression::ConstExpression;
use crate::expression::ConstGenericArgument;
use crate::expression::DiagnosticText;
use crate::expression::GenericArgument;
use crate::expression::GenericParameterDescriptor;
use crate::expression::LifetimeExpression;
use crate::expression::PredicateDescriptor;
use crate::expression::ReferenceTypeExpression;
use crate::expression::TraitBoundModifier;
use crate::expression::TypeExpression;

/// Creates a named type-parameter expression from generator-validated text.
#[doc(hidden)]
pub fn parameter(name: impl Into<Box<str>>) -> TypeExpression {
    TypeExpression::parameter(name).expect("generated type parameter names are non-empty")
}

/// Creates a named lifetime expression from generator-validated text.
#[doc(hidden)]
pub fn named_lifetime(name: impl Into<Box<str>>) -> LifetimeExpression {
    LifetimeExpression::named(name).expect("generated lifetime names are non-empty")
}

/// Creates a named const-parameter expression from generator-validated text.
#[doc(hidden)]
pub fn const_parameter(name: impl Into<Box<str>>) -> ConstExpression {
    ConstExpression::parameter(name).expect("generated const parameter names are non-empty")
}

/// Creates a const-item path from generator-validated segments.
#[doc(hidden)]
pub fn const_path<P, S>(segments: P) -> ConstExpression
where
    P: IntoIterator<Item = S>,
    S: Into<Box<str>>,
{
    ConstExpression::path(segments).expect("generated const paths are non-empty and contain no empty segments")
}

/// Creates an associated-type equality argument from generator-validated text.
#[doc(hidden)]
pub fn associated_type(name: impl Into<Box<str>>, value: TypeExpression) -> GenericArgument {
    GenericArgument::AssociatedType {
        name: crate::expression::ExpressionName::new(name).expect("generated associated type names are non-empty"),
        value: Box::new(value),
    }
}

/// Creates an associated-type bound argument from generator-validated text.
#[doc(hidden)]
pub fn associated_type_bound(name: impl Into<Box<str>>, bounds: Box<[PredicateDescriptor]>) -> GenericArgument {
    GenericArgument::AssociatedTypeBound {
        name: crate::expression::ExpressionName::new(name).expect("generated associated type names are non-empty"),
        bounds,
    }
}

/// Creates a lifetime generic parameter from generator-validated text.
#[doc(hidden)]
pub fn lifetime_parameter(
    name: impl Into<Box<str>>,
    bounds: Box<[LifetimeExpression]>,
    diagnostic: DiagnosticText,
) -> GenericParameterDescriptor {
    GenericParameterDescriptor::Lifetime {
        name: crate::expression::ExpressionName::new(name).expect("generated lifetime parameter names are non-empty"),
        bounds,
        diagnostic,
    }
}

/// Creates a type generic parameter from generator-validated text.
#[doc(hidden)]
pub fn type_parameter(
    name: impl Into<Box<str>>,
    bounds: Box<[PredicateDescriptor]>,
    default: Option<TypeExpression>,
    diagnostic: DiagnosticText,
) -> GenericParameterDescriptor {
    GenericParameterDescriptor::Type {
        name: crate::expression::ExpressionName::new(name).expect("generated type parameter names are non-empty"),
        bounds,
        default,
        diagnostic,
    }
}

/// Creates a const generic parameter from generator-validated text.
#[doc(hidden)]
pub fn const_generic_parameter(
    name: impl Into<Box<str>>,
    ty: TypeExpression,
    default: Option<ConstExpression>,
    diagnostic: DiagnosticText,
) -> GenericParameterDescriptor {
    GenericParameterDescriptor::Const {
        name: crate::expression::ExpressionName::new(name).expect("generated const parameter names are non-empty"),
        ty: Box::new(ty),
        default,
        diagnostic,
    }
}

/// Creates a concrete type expression from generator-validated path data.
#[doc(hidden)]
pub fn concrete(
    path: Box<[Box<str>]>,
    arguments: Box<[GenericArgument]>,
    diagnostic: DiagnosticText,
) -> ConcreteTypeExpression {
    let mut expression = ConcreteTypeExpression::new(path, arguments).expect("generated concrete paths are non-empty");
    expression.diagnostic = diagnostic;
    expression
}

/// Creates a typed const generic argument from generator-validated inputs.
#[doc(hidden)]
pub fn const_argument(
    declared_type: TypeExpression,
    value: ConstExpression,
    normalized_diagnostic: impl Into<Box<str>>,
) -> ConstGenericArgument {
    ConstGenericArgument::new(declared_type, value, normalized_diagnostic)
}

/// Creates an array type expression from generated structural inputs.
#[doc(hidden)]
pub fn array(element: TypeExpression, length: ConstExpression) -> ArrayTypeExpression {
    ArrayTypeExpression::new(element, length)
}

/// Creates a reference type expression from generated structural inputs.
#[doc(hidden)]
pub fn reference(lifetime: LifetimeExpression, mutable: bool, target: TypeExpression) -> ReferenceTypeExpression {
    ReferenceTypeExpression::new(lifetime, mutable, target)
}

/// Creates a non-empty type-bound predicate from generator-validated parallel
/// inputs.
#[doc(hidden)]
pub fn type_bound(
    subject: TypeExpression,
    bounds: Box<[TypeExpression]>,
    modifiers: Box<[TraitBoundModifier]>,
    higher_ranked_lifetimes: Box<[LifetimeExpression]>,
) -> PredicateDescriptor {
    PredicateDescriptor::type_bound(subject, bounds, modifiers, higher_ranked_lifetimes)
        .expect("generated type bounds are non-empty and have one modifier per bound")
}

/// Creates a non-empty lifetime-outlives predicate from generator-validated
/// inputs.
#[doc(hidden)]
pub fn lifetime_outlives(lifetime: LifetimeExpression, bounds: Box<[LifetimeExpression]>) -> PredicateDescriptor {
    PredicateDescriptor::lifetime_outlives(lifetime, bounds)
        .expect("generated lifetime-outlives predicates have at least one bound")
}
