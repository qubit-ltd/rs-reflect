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
use crate::expression::LifetimeExpression;
use crate::expression::PredicateDescriptor;
use crate::expression::ReferenceTypeExpression;
use crate::expression::TraitBoundModifier;
use crate::expression::TypeExpression;

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
pub fn reference(
    lifetime: LifetimeExpression,
    mutable: bool,
    target: TypeExpression,
) -> ReferenceTypeExpression {
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
pub fn lifetime_outlives(
    lifetime: LifetimeExpression,
    bounds: Box<[LifetimeExpression]>,
) -> PredicateDescriptor {
    PredicateDescriptor::lifetime_outlives(lifetime, bounds)
        .expect("generated lifetime-outlives predicates have at least one bound")
}
