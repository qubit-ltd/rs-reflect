// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow explicit-imports
//! Integration tests for structural Rust type expressions.
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

use qubit_reflect as reflect;
use qubit_reflect::expression::ArrayTypeExpression;
use qubit_reflect::expression::AssociatedTypeExpression;
use qubit_reflect::expression::ConcreteTypeExpression;
use qubit_reflect::expression::ConstExpression;
use qubit_reflect::expression::ConstGenericArgument;
use qubit_reflect::expression::DiagnosticText;
use qubit_reflect::expression::ExpressionError;
use qubit_reflect::expression::FunctionAbi;
use qubit_reflect::expression::FunctionPointerExpression;
use qubit_reflect::expression::FunctionSafety;
use qubit_reflect::expression::GenericArgument;
use qubit_reflect::expression::GenericDefinitionDescriptor;
use qubit_reflect::expression::GenericParameterDescriptor;
use qubit_reflect::expression::LifetimeExpression;
use qubit_reflect::expression::OpaqueTypeExpression;
use qubit_reflect::expression::PredicateDescriptor;
use qubit_reflect::expression::RawPointerTypeExpression;
use qubit_reflect::expression::ReferenceTypeExpression;
use qubit_reflect::expression::TraitBoundModifier;
use qubit_reflect::expression::TraitObjectExpression;
use qubit_reflect::expression::TypeExpression;

#[test]
fn concrete_type_rejects_an_empty_path() {
    assert_eq!(
        ConcreteTypeExpression::new(Vec::<Box<str>>::new(), Box::<[GenericArgument]>::default(),),
        Err(ExpressionError::EmptyConcretePath),
    );
}

#[test]
fn type_bound_rejects_mismatched_modifiers() {
    assert_eq!(
        PredicateDescriptor::type_bound(
            TypeExpression::SelfType,
            vec![TypeExpression::SelfType].into_boxed_slice(),
            Box::<[TraitBoundModifier]>::default(),
            Box::<[LifetimeExpression]>::default(),
        ),
        Err(ExpressionError::BoundModifierCount {
            bounds: 1,
            modifiers: 0,
        }),
    );
}

/// Builds a concrete path expression without relying on parser implementation
/// types.
fn concrete(path: &[&str], arguments: Vec<GenericArgument>) -> TypeExpression {
    TypeExpression::Concrete(
        ConcreteTypeExpression::new(path.iter().copied(), arguments).expect("test paths are non-empty"),
    )
}

/// Computes the structural identity hash used by equality regression tests.
fn identity_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// Verifies that nested references, slices, and generic parameters remain
/// navigable.
#[test]
fn test_type_expression_navigates_mutable_reference_to_slice_parameter() {
    let expression = TypeExpression::Reference(ReferenceTypeExpression::new(
        LifetimeExpression::Named("a".into()),
        true,
        TypeExpression::Slice(Box::new(TypeExpression::Parameter("T".into()))),
    ));

    let TypeExpression::Reference(reference) = expression else {
        panic!("expected a reference expression");
    };
    assert_eq!(reference.lifetime(), &LifetimeExpression::Named("a".into()));
    assert!(reference.is_mutable());
    let TypeExpression::Slice(element) = reference.target() else {
        panic!("expected a slice expression");
    };
    assert_eq!(element.as_ref(), &TypeExpression::Parameter("T".into()));
}

/// Verifies that an omitted reference lifetime has one canonical
/// representation.
#[test]
fn test_type_expression_navigates_elided_reference_lifetime() {
    let expression = TypeExpression::Reference(ReferenceTypeExpression::new(
        LifetimeExpression::Elided,
        false,
        TypeExpression::Parameter("T".into()),
    ));

    let TypeExpression::Reference(reference) = expression else {
        panic!("expected a reference expression");
    };
    assert_eq!(reference.lifetime(), &LifetimeExpression::Elided);
}

/// Verifies that function pointers retain ABI, safety, HRTB, and opaque return
/// bounds.
#[test]
fn test_type_expression_navigates_function_pointer_with_opaque_iterator_return() {
    let iterator = concrete(
        &["core", "iter", "Iterator"],
        vec![GenericArgument::AssociatedType {
            name: "Item".into(),
            value: Box::new(TypeExpression::Parameter("T".into())),
        }],
    );
    let expression = TypeExpression::FunctionPointer(FunctionPointerExpression::new(
        FunctionAbi::Rust,
        FunctionSafety::Safe,
        false,
        vec![LifetimeExpression::Named("a".into())].into_boxed_slice(),
        vec![TypeExpression::Reference(ReferenceTypeExpression::new(
            LifetimeExpression::Named("a".into()),
            false,
            TypeExpression::Parameter("T".into()),
        ))]
        .into_boxed_slice(),
        TypeExpression::Opaque(OpaqueTypeExpression::new(
            vec![PredicateDescriptor::TypeBound {
                subject: TypeExpression::SelfType,
                bounds: Box::new([iterator]),
                bound_modifiers: Box::new([reflect::expression::TraitBoundModifier::None]),
                higher_ranked_lifetimes: Box::default(),
                diagnostic: DiagnosticText::default(),
            }]
            .into_boxed_slice(),
        )),
    ));

    let TypeExpression::FunctionPointer(function) = expression else {
        panic!("expected a function pointer expression");
    };
    assert_eq!(function.abi(), &FunctionAbi::Rust);
    assert_eq!(function.safety(), &FunctionSafety::Safe);
    assert_eq!(
        function.higher_ranked_lifetimes().as_ref(),
        &[LifetimeExpression::Named("a".into())]
    );
    let TypeExpression::Reference(parameter) = &function.parameters()[0] else {
        panic!("expected a reference parameter");
    };
    assert_eq!(parameter.lifetime(), &function.higher_ranked_lifetimes()[0]);
    let TypeExpression::Opaque(opaque) = function.return_type() else {
        panic!("expected an opaque return expression");
    };
    assert_eq!(opaque.bounds().len(), 1);
}

/// Verifies that a higher-ranked trait bound connects its binder to the bound's
/// reference.
#[test]
fn test_predicate_navigates_higher_ranked_trait_bound_lifetime() {
    let bound = concrete(
        &["Fn"],
        vec![GenericArgument::Type(TypeExpression::Tuple(Box::new([
            TypeExpression::Reference(ReferenceTypeExpression::new(
                LifetimeExpression::Named("a".into()),
                false,
                TypeExpression::Parameter("T".into()),
            )),
        ])))],
    );
    let predicate = PredicateDescriptor::TypeBound {
        subject: TypeExpression::Parameter("F".into()),
        bounds: Box::new([bound]),
        bound_modifiers: Box::new([reflect::expression::TraitBoundModifier::None]),
        higher_ranked_lifetimes: Box::new([LifetimeExpression::Named("a".into())]),
        diagnostic: DiagnosticText::default(),
    };

    let PredicateDescriptor::TypeBound {
        bounds,
        higher_ranked_lifetimes,
        ..
    } = predicate
    else {
        panic!("expected a type-bound predicate");
    };
    let TypeExpression::Concrete(callable) = &bounds[0] else {
        panic!("expected a trait path");
    };
    let GenericArgument::Type(TypeExpression::Tuple(parameters)) = &callable.arguments()[0] else {
        panic!("expected trait parameter types");
    };
    let TypeExpression::Reference(parameter) = &parameters[0] else {
        panic!("expected a reference trait parameter");
    };
    assert_eq!(parameter.lifetime(), &higher_ranked_lifetimes[0]);
}

/// Verifies that generic declarations preserve type, lifetime, const, and where
/// predicates.
#[test]
fn test_generic_definition_navigates_const_generic_and_predicates() {
    let definition = GenericDefinitionDescriptor::new(
        vec![
            GenericParameterDescriptor::Lifetime {
                name: "a".into(),
                bounds: Box::new([LifetimeExpression::Static]),
                diagnostic: DiagnosticText::default(),
            },
            GenericParameterDescriptor::Type {
                name: "T".into(),
                bounds: Box::default(),
                default: None,
                diagnostic: DiagnosticText::default(),
            },
            GenericParameterDescriptor::Const {
                name: "N".into(),
                ty: Box::new(concrete(&["usize"], Vec::new())),
                default: Some(ConstExpression::UnsignedInteger(4)),
                diagnostic: DiagnosticText::default(),
            },
        ]
        .into_boxed_slice(),
        vec![PredicateDescriptor::TypeBound {
            subject: TypeExpression::Parameter("T".into()),
            bounds: Box::new([concrete(&["Clone"], Vec::new())]),
            bound_modifiers: Box::new([reflect::expression::TraitBoundModifier::None]),
            higher_ranked_lifetimes: Box::default(),
            diagnostic: DiagnosticText::default(),
        }]
        .into_boxed_slice(),
    );

    assert_eq!(definition.parameters().len(), 3);
    let GenericParameterDescriptor::Const { default, .. } = &definition.parameters()[2] else {
        panic!("expected a const generic parameter");
    };
    assert_eq!(default, &Some(ConstExpression::UnsignedInteger(4)));
    assert!(matches!(
        definition.predicates()[0],
        PredicateDescriptor::TypeBound { .. }
    ));
}

/// Verifies that const arguments retain their declaration type, structural
/// value, and diagnostic.
#[test]
fn test_generic_argument_navigates_typed_const_value() {
    let argument = GenericArgument::Const(ConstGenericArgument::new(
        concrete(&["usize"], Vec::new()),
        ConstExpression::UnsignedInteger(4),
        "4usize",
    ));
    let alternate_diagnostic = GenericArgument::Const(ConstGenericArgument::new(
        concrete(&["usize"], Vec::new()),
        ConstExpression::UnsignedInteger(4),
        "4_usize",
    ));

    assert_eq!(argument, alternate_diagnostic);
    assert_eq!(identity_hash(&argument), identity_hash(&alternate_diagnostic));

    let GenericArgument::Const(argument) = argument else {
        panic!("expected a const generic argument");
    };
    assert_eq!(argument.declared_type(), &concrete(&["usize"], Vec::new()));
    assert_eq!(argument.value(), &ConstExpression::UnsignedInteger(4));
    assert_eq!(argument.normalized_diagnostic(), "4usize");

    let signed_argument = GenericArgument::Const(ConstGenericArgument::new(
        concrete(&["i32"], Vec::new()),
        ConstExpression::SignedInteger(-1),
        "-1i32",
    ));
    let GenericArgument::Const(signed_argument) = signed_argument else {
        panic!("expected a signed const generic argument");
    };
    assert_eq!(signed_argument.value(), &ConstExpression::SignedInteger(-1));
}

/// Verifies that qualified associated types retain every projection component.
#[test]
fn test_type_expression_navigates_qualified_associated_type() {
    let expression = TypeExpression::Associated(AssociatedTypeExpression::new(
        TypeExpression::Parameter("T".into()),
        Some(concrete(
            &["core", "iter", "Iterator"],
            vec![GenericArgument::Lifetime(LifetimeExpression::Named("a".into()))],
        )),
        "Item",
        vec![GenericArgument::Type(TypeExpression::Parameter("U".into()))].into_boxed_slice(),
    ));

    let TypeExpression::Associated(associated) = expression else {
        panic!("expected an associated type projection");
    };
    assert_eq!(associated.self_type(), &TypeExpression::Parameter("T".into()));
    let Some(TypeExpression::Concrete(trait_path)) = associated.trait_path() else {
        panic!("expected a concrete trait path");
    };
    assert_eq!(
        trait_path.path().as_ref(),
        &["core".into(), "iter".into(), "Iterator".into()]
    );
    assert_eq!(
        trait_path.arguments().as_ref(),
        &[GenericArgument::Lifetime(LifetimeExpression::Named("a".into()))]
    );
    assert_eq!(associated.item(), "Item");
    assert_eq!(
        associated.arguments().as_ref(),
        &[GenericArgument::Type(TypeExpression::Parameter("U".into()))]
    );
}

/// Verifies that an unqualified associated type records the absent trait path.
#[test]
fn test_type_expression_navigates_unqualified_associated_type() {
    let expression = TypeExpression::Associated(AssociatedTypeExpression::new(
        TypeExpression::Parameter("T".into()),
        None,
        "Item",
        Box::<[GenericArgument]>::default(),
    ));

    let TypeExpression::Associated(associated) = expression else {
        panic!("expected an associated type projection");
    };
    assert!(associated.trait_path().is_none());
}

/// Verifies that raw pointer mutability and pointee type remain navigable.
#[test]
fn test_type_expression_navigates_raw_pointer() {
    let expression = TypeExpression::RawPointer(RawPointerTypeExpression::new(
        true,
        TypeExpression::Parameter("T".into()),
    ));

    let TypeExpression::RawPointer(pointer) = expression else {
        panic!("expected a raw pointer expression");
    };
    assert!(pointer.is_mutable());
    assert_eq!(pointer.target(), &TypeExpression::Parameter("T".into()));
}

/// Verifies that tuple element order and nested forms remain navigable.
#[test]
fn test_type_expression_navigates_tuple() {
    let expression = TypeExpression::Tuple(Box::new([TypeExpression::Parameter("T".into()), TypeExpression::Never]));

    let TypeExpression::Tuple(elements) = expression else {
        panic!("expected a tuple expression");
    };
    assert_eq!(
        elements.as_ref(),
        &[TypeExpression::Parameter("T".into()), TypeExpression::Never]
    );
}

/// Verifies that array, trait-object, and never forms are represented
/// structurally.
#[test]
fn test_type_expression_navigates_array_trait_object_and_never() {
    let array = TypeExpression::Array(ArrayTypeExpression::new(
        TypeExpression::Never,
        ConstExpression::Parameter("N".into()),
    ));
    let object = TypeExpression::TraitObject(TraitObjectExpression::new(
        vec![PredicateDescriptor::TypeBound {
            subject: TypeExpression::SelfType,
            bounds: Box::new([concrete(&["Display"], Vec::new())]),
            bound_modifiers: Box::new([reflect::expression::TraitBoundModifier::None]),
            higher_ranked_lifetimes: Box::default(),
            diagnostic: DiagnosticText::default(),
        }]
        .into_boxed_slice(),
    ));

    let TypeExpression::Array(array) = array else {
        panic!("expected an array expression");
    };
    assert_eq!(array.element(), &TypeExpression::Never);
    assert_eq!(array.length(), &ConstExpression::Parameter("N".into()));
    assert!(matches!(object, TypeExpression::TraitObject(_)));
}

/// Verifies that diagnostics do not participate in expression identity or
/// hashing.
#[test]
fn test_descriptor_diagnostic_text_does_not_affect_identity() {
    let plain = concrete(&["u8"], Vec::new());
    let annotated = TypeExpression::Concrete(
        ConcreteTypeExpression::new(["u8"], Vec::new())
            .expect("test path is non-empty")
            .with_diagnostic("u8 from source text"),
    );

    assert_ne!(DiagnosticText::default(), "u8 from source text".into());
    assert_eq!(plain, annotated);
    assert_eq!(identity_hash(&plain), identity_hash(&annotated));
    assert_ne!(format!("{annotated:?}"), format!("{plain:?}"));

    let plain_predicate = PredicateDescriptor::TypeOutlives {
        ty: TypeExpression::Parameter("T".into()),
        lifetime: LifetimeExpression::Named("a".into()),
        diagnostic: DiagnosticText::default(),
    };
    let annotated_predicate = PredicateDescriptor::TypeOutlives {
        ty: TypeExpression::Parameter("T".into()),
        lifetime: LifetimeExpression::Named("a".into()),
        diagnostic: "T: 'a".into(),
    };
    assert_eq!(plain_predicate, annotated_predicate);
    assert_eq!(identity_hash(&plain_predicate), identity_hash(&annotated_predicate));

    let plain_definition = GenericDefinitionDescriptor::new(
        vec![GenericParameterDescriptor::Lifetime {
            name: "a".into(),
            bounds: Box::default(),
            diagnostic: DiagnosticText::default(),
        }]
        .into_boxed_slice(),
        Box::<[PredicateDescriptor]>::default(),
    );
    let annotated_definition = GenericDefinitionDescriptor::new(
        vec![GenericParameterDescriptor::Lifetime {
            name: "a".into(),
            bounds: Box::default(),
            diagnostic: "'a".into(),
        }]
        .into_boxed_slice(),
        Box::<[PredicateDescriptor]>::default(),
    )
    .with_diagnostic("<'a>");
    assert_eq!(plain_definition, annotated_definition);
    assert_eq!(identity_hash(&plain_definition), identity_hash(&annotated_definition));
}
