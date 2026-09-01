// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::expression::GenericParameterDescriptor;
use qubit_reflect::expression::LifetimeExpression;
use qubit_reflect::expression::PredicateDescriptor;
use qubit_reflect::expression::TypeExpression;
use qubit_reflect::reflect;

struct Target;

#[reflect]
trait GatContract {
    type Output<'a, T>
    where
        Self: 'a,
        T: 'a;
}

impl GatContract for Target {
    type Output<'a, T>
        = (&'a (), T)
    where
        T: 'a;
}

fn main() {
    let payload = <Target as GatContract>::__qubit_reflect_trait_payload();
    let generic = payload.applied().associated_types()[0].generic_definition();
    assert!(matches!(
        generic.parameters().as_ref(),
        [
            GenericParameterDescriptor::Lifetime { name, .. },
            GenericParameterDescriptor::Type { name: type_name, .. },
        ] if name.as_ref() == "a" && type_name.as_ref() == "T"
    ));
    assert!(matches!(
        generic.predicates().as_ref(),
        [
            PredicateDescriptor::TypeOutlives {
                ty: TypeExpression::SelfType,
                lifetime: LifetimeExpression::Named(self_lifetime),
                ..
            },
            PredicateDescriptor::TypeOutlives {
                ty: TypeExpression::Parameter(parameter),
                lifetime: LifetimeExpression::Named(type_lifetime),
                ..
            },
        ] if self_lifetime.as_ref() == "a"
            && parameter.as_ref() == "T"
            && type_lifetime.as_ref() == "a"
    ));
}
