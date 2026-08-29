//! Integration tests for structural Rust type expressions.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use qubit_reflect::expression::{
    ArrayTypeExpression, ConcreteTypeExpression, ConstExpression, DiagnosticText, FunctionAbi,
    FunctionPointerExpression, FunctionSafety, GenericArgument, GenericDefinitionDescriptor,
    GenericParameterDescriptor, LifetimeExpression, OpaqueTypeExpression, PredicateDescriptor,
    ReferenceTypeExpression, TraitObjectExpression, TypeExpression,
};

/// Builds a concrete path expression without relying on parser implementation types.
fn concrete(path: &[&str], arguments: Vec<GenericArgument>) -> TypeExpression {
    TypeExpression::Concrete(ConcreteTypeExpression {
        path: path.iter().map(|segment| (*segment).into()).collect(),
        arguments: arguments.into_boxed_slice(),
        diagnostic: DiagnosticText::default(),
    })
}

/// Verifies that nested references, slices, and generic parameters remain navigable.
#[test]
fn test_type_expression_navigates_mutable_reference_to_slice_parameter() {
    let expression = TypeExpression::Reference(ReferenceTypeExpression {
        lifetime: Some(LifetimeExpression::Named("a".into())),
        mutable: true,
        target: Box::new(TypeExpression::Slice(Box::new(TypeExpression::Parameter(
            "T".into(),
        )))),
        diagnostic: DiagnosticText::default(),
    });

    let TypeExpression::Reference(reference) = expression else {
        panic!("expected a reference expression");
    };
    assert_eq!(reference.lifetime, Some(LifetimeExpression::Named("a".into())));
    assert!(reference.mutable);
    let TypeExpression::Slice(element) = reference.target.as_ref() else {
        panic!("expected a slice expression");
    };
    assert_eq!(element.as_ref(), &TypeExpression::Parameter("T".into()));
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
            lifetime: None,
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
    assert_eq!(function.higher_ranked_lifetimes.as_ref(), &[LifetimeExpression::Named("a".into())]);
    let TypeExpression::Opaque(opaque) = function.return_type.as_ref() else {
        panic!("expected an opaque return expression");
    };
    assert_eq!(opaque.bounds.len(), 1);
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
                default: Some(ConstExpression::Integer(4)),
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
    assert_eq!(default, &Some(ConstExpression::Integer(4)));
    assert!(matches!(definition.predicates[0], PredicateDescriptor::TypeBound { .. }));
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
fn test_type_expression_diagnostic_text_does_not_affect_identity() {
    let plain = concrete(&["u8"], Vec::new());
    let annotated = TypeExpression::Concrete(ConcreteTypeExpression {
        path: Box::new(["u8".into()]),
        arguments: Box::default(),
        diagnostic: "u8 from source text".into(),
    });

    assert_eq!(plain, annotated);
    let mut plain_hasher = DefaultHasher::new();
    plain.hash(&mut plain_hasher);
    let mut annotated_hasher = DefaultHasher::new();
    annotated.hash(&mut annotated_hasher);
    assert_eq!(plain_hasher.finish(), annotated_hasher.finish());
    assert_ne!(format!("{annotated:?}"), format!("{plain:?}"));
    assert!(!format!("{annotated:?}").contains("syn"));
}
