//! Integration tests for structural Rust type expressions.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use qubit_reflect::expression::{
    ArrayTypeExpression, AssociatedTypeExpression, ConcreteTypeExpression, ConstExpression,
    ConstGenericArgument, DiagnosticText, FunctionAbi, FunctionPointerExpression, FunctionSafety,
    GenericArgument, GenericDefinitionDescriptor, GenericParameterDescriptor, LifetimeExpression,
    OpaqueTypeExpression, PredicateDescriptor, RawPointerTypeExpression, ReferenceTypeExpression,
    TraitObjectExpression, TypeExpression,
};

/// Builds a concrete path expression without relying on parser implementation types.
fn concrete(path: &[&str], arguments: Vec<GenericArgument>) -> TypeExpression {
    TypeExpression::Concrete(ConcreteTypeExpression {
        path: path.iter().map(|segment| (*segment).into()).collect(),
        arguments: arguments.into_boxed_slice(),
        diagnostic: DiagnosticText::default(),
    })
}

/// Computes the structural identity hash used by equality regression tests.
fn identity_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// Verifies that nested references, slices, and generic parameters remain navigable.
#[test]
fn test_type_expression_navigates_mutable_reference_to_slice_parameter() {
    let expression = TypeExpression::Reference(ReferenceTypeExpression {
        lifetime: LifetimeExpression::Named("a".into()),
        mutable: true,
        target: Box::new(TypeExpression::Slice(Box::new(TypeExpression::Parameter(
            "T".into(),
        )))),
        diagnostic: DiagnosticText::default(),
    });

    let TypeExpression::Reference(reference) = expression else {
        panic!("expected a reference expression");
    };
    assert_eq!(reference.lifetime, LifetimeExpression::Named("a".into()));
    assert!(reference.mutable);
    let TypeExpression::Slice(element) = reference.target.as_ref() else {
        panic!("expected a slice expression");
    };
    assert_eq!(element.as_ref(), &TypeExpression::Parameter("T".into()));
}

/// Verifies that an omitted reference lifetime has one canonical representation.
#[test]
fn test_type_expression_navigates_elided_reference_lifetime() {
    let expression = TypeExpression::Reference(ReferenceTypeExpression {
        lifetime: LifetimeExpression::Elided,
        mutable: false,
        target: Box::new(TypeExpression::Parameter("T".into())),
        diagnostic: DiagnosticText::default(),
    });

    let TypeExpression::Reference(reference) = expression else {
        panic!("expected a reference expression");
    };
    assert_eq!(reference.lifetime, LifetimeExpression::Elided);
}

/// Verifies that function pointers retain ABI, safety, HRTB, and opaque return bounds.
#[test]
fn test_type_expression_navigates_function_pointer_with_opaque_iterator_return() {
    let iterator = concrete(
        &["core", "iter", "Iterator"],
        vec![GenericArgument::AssociatedType {
            name: "Item".into(),
            value: Box::new(TypeExpression::Parameter("T".into())),
        }],
    );
    let expression = TypeExpression::FunctionPointer(FunctionPointerExpression {
        abi: FunctionAbi::Rust,
        safety: FunctionSafety::Safe,
        variadic: false,
        higher_ranked_lifetimes: Box::new([LifetimeExpression::Named("a".into())]),
        parameters: Box::new([TypeExpression::Reference(ReferenceTypeExpression {
            lifetime: LifetimeExpression::Named("a".into()),
            mutable: false,
            target: Box::new(TypeExpression::Parameter("T".into())),
            diagnostic: DiagnosticText::default(),
        })]),
        return_type: Box::new(TypeExpression::Opaque(OpaqueTypeExpression {
            bounds: Box::new([PredicateDescriptor::TypeBound {
                subject: TypeExpression::SelfType,
                bounds: Box::new([iterator]),
                higher_ranked_lifetimes: Box::default(),
                diagnostic: DiagnosticText::default(),
            }]),
            diagnostic: DiagnosticText::default(),
        })),
        diagnostic: DiagnosticText::default(),
    });

    let TypeExpression::FunctionPointer(function) = expression else {
        panic!("expected a function pointer expression");
    };
    assert_eq!(function.abi, FunctionAbi::Rust);
    assert_eq!(function.safety, FunctionSafety::Safe);
    assert_eq!(
        function.higher_ranked_lifetimes.as_ref(),
        &[LifetimeExpression::Named("a".into())]
    );
    let TypeExpression::Reference(parameter) = &function.parameters[0] else {
        panic!("expected a reference parameter");
    };
    assert_eq!(parameter.lifetime, function.higher_ranked_lifetimes[0]);
    let TypeExpression::Opaque(opaque) = function.return_type.as_ref() else {
        panic!("expected an opaque return expression");
    };
    assert_eq!(opaque.bounds.len(), 1);
}

/// Verifies that a higher-ranked trait bound connects its binder to the bound's
/// reference.
#[test]
fn test_predicate_navigates_higher_ranked_trait_bound_lifetime() {
    let bound = concrete(
        &["Fn"],
        vec![GenericArgument::Type(TypeExpression::Tuple(Box::new([
            TypeExpression::Reference(ReferenceTypeExpression {
                lifetime: LifetimeExpression::Named("a".into()),
                mutable: false,
                target: Box::new(TypeExpression::Parameter("T".into())),
                diagnostic: DiagnosticText::default(),
            }),
        ])))],
    );
    let predicate = PredicateDescriptor::TypeBound {
        subject: TypeExpression::Parameter("F".into()),
        bounds: Box::new([bound]),
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
    let GenericArgument::Type(TypeExpression::Tuple(parameters)) = &callable.arguments[0] else {
        panic!("expected trait parameter types");
    };
    let TypeExpression::Reference(parameter) = &parameters[0] else {
        panic!("expected a reference trait parameter");
    };
    assert_eq!(parameter.lifetime, higher_ranked_lifetimes[0]);
}

/// Verifies that generic declarations preserve type, lifetime, const, and where predicates.
#[test]
fn test_generic_definition_navigates_const_generic_and_predicates() {
    let definition = GenericDefinitionDescriptor {
        parameters: Box::new([
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
        ]),
        predicates: Box::new([PredicateDescriptor::TypeBound {
            subject: TypeExpression::Parameter("T".into()),
            bounds: Box::new([concrete(&["Clone"], Vec::new())]),
            higher_ranked_lifetimes: Box::default(),
            diagnostic: DiagnosticText::default(),
        }]),
        diagnostic: DiagnosticText::default(),
    };

    assert_eq!(definition.parameters.len(), 3);
    let GenericParameterDescriptor::Const { default, .. } = &definition.parameters[2] else {
        panic!("expected a const generic parameter");
    };
    assert_eq!(default, &Some(ConstExpression::UnsignedInteger(4)));
    assert!(matches!(
        definition.predicates[0],
        PredicateDescriptor::TypeBound { .. }
    ));
}

/// Verifies that const arguments retain their declaration type, structural
/// value, and diagnostic.
#[test]
fn test_generic_argument_navigates_typed_const_value() {
    let argument = GenericArgument::Const(ConstGenericArgument {
        declared_type: Box::new(concrete(&["usize"], Vec::new())),
        value: ConstExpression::UnsignedInteger(4),
        normalized_diagnostic: "4usize".into(),
    });
    let alternate_diagnostic = GenericArgument::Const(ConstGenericArgument {
        declared_type: Box::new(concrete(&["usize"], Vec::new())),
        value: ConstExpression::UnsignedInteger(4),
        normalized_diagnostic: "4_usize".into(),
    });

    assert_eq!(argument, alternate_diagnostic);
    assert_eq!(
        identity_hash(&argument),
        identity_hash(&alternate_diagnostic)
    );

    let GenericArgument::Const(argument) = argument else {
        panic!("expected a const generic argument");
    };
    assert_eq!(
        argument.declared_type.as_ref(),
        &concrete(&["usize"], Vec::new())
    );
    assert_eq!(argument.value, ConstExpression::UnsignedInteger(4));
    assert_eq!(argument.normalized_diagnostic.as_ref(), "4usize");

    let signed_argument = GenericArgument::Const(ConstGenericArgument {
        declared_type: Box::new(concrete(&["i32"], Vec::new())),
        value: ConstExpression::SignedInteger(-1),
        normalized_diagnostic: "-1i32".into(),
    });
    let GenericArgument::Const(signed_argument) = signed_argument else {
        panic!("expected a signed const generic argument");
    };
    assert_eq!(signed_argument.value, ConstExpression::SignedInteger(-1));
}

/// Verifies that qualified associated types retain every projection component.
#[test]
fn test_type_expression_navigates_qualified_associated_type() {
    let expression = TypeExpression::Associated(AssociatedTypeExpression {
        self_type: Box::new(TypeExpression::Parameter("T".into())),
        trait_path: Some(Box::new(concrete(
            &["core", "iter", "Iterator"],
            vec![GenericArgument::Lifetime(LifetimeExpression::Named(
                "a".into(),
            ))],
        ))),
        item: "Item".into(),
        arguments: Box::new([GenericArgument::Type(TypeExpression::Parameter("U".into()))]),
        diagnostic: DiagnosticText::default(),
    });

    let TypeExpression::Associated(associated) = expression else {
        panic!("expected an associated type projection");
    };
    assert_eq!(
        associated.self_type.as_ref(),
        &TypeExpression::Parameter("T".into())
    );
    let Some(TypeExpression::Concrete(trait_path)) = associated.trait_path.as_deref() else {
        panic!("expected a concrete trait path");
    };
    assert_eq!(
        trait_path.path.as_ref(),
        &["core".into(), "iter".into(), "Iterator".into()]
    );
    assert_eq!(
        trait_path.arguments.as_ref(),
        &[GenericArgument::Lifetime(LifetimeExpression::Named(
            "a".into()
        ))]
    );
    assert_eq!(associated.item.as_ref(), "Item");
    assert_eq!(
        associated.arguments.as_ref(),
        &[GenericArgument::Type(TypeExpression::Parameter("U".into()))]
    );
}

/// Verifies that an unqualified associated type records the absent trait path.
#[test]
fn test_type_expression_navigates_unqualified_associated_type() {
    let expression = TypeExpression::Associated(AssociatedTypeExpression {
        self_type: Box::new(TypeExpression::Parameter("T".into())),
        trait_path: None,
        item: "Item".into(),
        arguments: Box::default(),
        diagnostic: DiagnosticText::default(),
    });

    let TypeExpression::Associated(associated) = expression else {
        panic!("expected an associated type projection");
    };
    assert!(associated.trait_path.is_none());
}

/// Verifies that raw pointer mutability and pointee type remain navigable.
#[test]
fn test_type_expression_navigates_raw_pointer() {
    let expression = TypeExpression::RawPointer(RawPointerTypeExpression {
        mutable: true,
        target: Box::new(TypeExpression::Parameter("T".into())),
        diagnostic: DiagnosticText::default(),
    });

    let TypeExpression::RawPointer(pointer) = expression else {
        panic!("expected a raw pointer expression");
    };
    assert!(pointer.mutable);
    assert_eq!(
        pointer.target.as_ref(),
        &TypeExpression::Parameter("T".into())
    );
}

/// Verifies that tuple element order and nested forms remain navigable.
#[test]
fn test_type_expression_navigates_tuple() {
    let expression = TypeExpression::Tuple(Box::new([
        TypeExpression::Parameter("T".into()),
        TypeExpression::Never,
    ]));

    let TypeExpression::Tuple(elements) = expression else {
        panic!("expected a tuple expression");
    };
    assert_eq!(
        elements.as_ref(),
        &[TypeExpression::Parameter("T".into()), TypeExpression::Never]
    );
}

/// Verifies that array, trait-object, and never forms are represented structurally.
#[test]
fn test_type_expression_navigates_array_trait_object_and_never() {
    let array = TypeExpression::Array(ArrayTypeExpression {
        element: Box::new(TypeExpression::Never),
        length: ConstExpression::Parameter("N".into()),
        diagnostic: DiagnosticText::default(),
    });
    let object = TypeExpression::TraitObject(TraitObjectExpression {
        bounds: Box::new([PredicateDescriptor::TypeBound {
            subject: TypeExpression::SelfType,
            bounds: Box::new([concrete(&["Display"], Vec::new())]),
            higher_ranked_lifetimes: Box::default(),
            diagnostic: DiagnosticText::default(),
        }]),
        diagnostic: DiagnosticText::default(),
    });

    let TypeExpression::Array(array) = array else {
        panic!("expected an array expression");
    };
    assert_eq!(array.element.as_ref(), &TypeExpression::Never);
    assert_eq!(array.length, ConstExpression::Parameter("N".into()));
    assert!(matches!(object, TypeExpression::TraitObject(_)));
}

/// Verifies that diagnostics do not participate in expression identity or hashing.
#[test]
fn test_descriptor_diagnostic_text_does_not_affect_identity() {
    let plain = concrete(&["u8"], Vec::new());
    let annotated = TypeExpression::Concrete(ConcreteTypeExpression {
        path: Box::new(["u8".into()]),
        arguments: Box::default(),
        diagnostic: "u8 from source text".into(),
    });

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
    assert_eq!(
        identity_hash(&plain_predicate),
        identity_hash(&annotated_predicate)
    );

    let plain_definition = GenericDefinitionDescriptor {
        parameters: Box::new([GenericParameterDescriptor::Lifetime {
            name: "a".into(),
            bounds: Box::default(),
            diagnostic: DiagnosticText::default(),
        }]),
        predicates: Box::default(),
        diagnostic: DiagnosticText::default(),
    };
    let annotated_definition = GenericDefinitionDescriptor {
        parameters: Box::new([GenericParameterDescriptor::Lifetime {
            name: "a".into(),
            bounds: Box::default(),
            diagnostic: "'a".into(),
        }]),
        predicates: Box::default(),
        diagnostic: "<'a>".into(),
    };
    assert_eq!(plain_definition, annotated_definition);
    assert_eq!(
        identity_hash(&plain_definition),
        identity_hash(&annotated_definition)
    );
}
