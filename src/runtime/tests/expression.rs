// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::generic::GenericArgumentMetadata;
use crate::runtime::descriptor::generic::GenericArgumentMetadataKind;
use crate::runtime::descriptor::generic::GenericArguments;
use crate::runtime::descriptor::generic::GenericDefinitionDescriptor;
use crate::runtime::descriptor::generic::GenericDefinitionMetadata;
use crate::runtime::descriptor::generic::GenericParameterMetadata;
use crate::runtime::descriptor::generic::GenericParameterMetadataKind;
use crate::runtime::descriptor::generic::TypeDefinitionDescriptor;
use crate::runtime::descriptor::generic::TypeDefinitionId;
use crate::runtime::descriptor::generic::TypeDefinitionKind;
use crate::runtime::expression::ConstExpression;
use crate::runtime::expression::ExpressionData;
use crate::runtime::expression::GenericExpressionArgument;
use crate::runtime::expression::LifetimeExpression;
use crate::runtime::expression::OpaqueTypeData;
use crate::runtime::expression::PredicateData;
use crate::runtime::expression::PredicateKind;
use crate::runtime::expression::TraitBoundModifier;
use crate::runtime::expression::TraitExpressionData;
use crate::runtime::expression::TypeExpression;
use crate::runtime::expression::TypeExpressionKind;

struct EnvelopeMarker;
struct EnvelopeU8;
impl crate::runtime::descriptor::Reflect for EnvelopeU8 {
    fn __reflect_registration() -> &'static crate::runtime::descriptor::TypeRegistration<Self> {
        static REGISTRATION: crate::runtime::descriptor::TypeRegistration<EnvelopeU8> =
            crate::runtime::descriptor::TypeRegistration::new();
        &REGISTRATION
    }
}

static TYPE_PARAMETER: GenericParameterMetadata = GenericParameterMetadata {
    name: "T",
    index: 0,
    kind: GenericParameterMetadataKind::Type {
        bounds: &[],
        default: None,
    },
};
static LIFETIME_PARAMETER: GenericParameterMetadata = GenericParameterMetadata {
    name: "a",
    index: 0,
    kind: GenericParameterMetadataKind::Lifetime { bounds: &[] },
};

static CONST_PARAMETER: GenericParameterMetadata = GenericParameterMetadata {
    name: "N",
    index: 1,
    kind: GenericParameterMetadataKind::Const {
        ty: &ExpressionData::Named {
            path: "usize",
            definition: None,
            arguments: &[],
        },
        default: None,
    },
};

static GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata {
    parameters: &[TYPE_PARAMETER, CONST_PARAMETER],
    predicates: &[],
};
static DUPLICATE_PARAMETERS: GenericDefinitionMetadata = GenericDefinitionMetadata {
    parameters: &[TYPE_PARAMETER, TYPE_PARAMETER],
    predicates: &[],
};
static LIFETIME_GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata {
    parameters: &[LIFETIME_PARAMETER, TYPE_PARAMETER],
    predicates: &[],
};

static DEFINITION: super::super::descriptor::generic::TypeDefinitionMetadata =
    super::super::descriptor::generic::TypeDefinitionMetadata {
        id: TypeDefinitionId::of::<EnvelopeMarker>(),
        name: "Envelope",
        path: "fixture::Envelope",
        kind: TypeDefinitionKind::Struct(super::super::descriptor::generic::StructKind::Named),
        generics: &GENERICS,
    };

static T_EXPRESSION: ExpressionData = ExpressionData::Parameter(&TYPE_PARAMETER);
static UNLINKED_PARAMETER_EXPRESSION: ExpressionData = ExpressionData::ParameterName("U");
static VEC_EXPRESSION: ExpressionData = ExpressionData::Named {
    path: "alloc::vec::Vec",
    definition: None,
    arguments: &[GenericExpressionArgument::Type(&T_EXPRESSION)],
};
static DEFINITION_EXPRESSION: ExpressionData = ExpressionData::Named {
    path: "fixture::Envelope",
    definition: Some(&DEFINITION),
    arguments: &[],
};
static U8_EXPRESSION: ExpressionData = ExpressionData::Named {
    path: "u8",
    definition: None,
    arguments: &[],
};
static SELF_EXPRESSION: ExpressionData = ExpressionData::SelfType;
static REFERENCE_EXPRESSION: ExpressionData = ExpressionData::Reference {
    lifetime: Some(LifetimeExpression::Named("a")),
    mutable: false,
    target: &T_EXPRESSION,
};
static POINTER_EXPRESSION: ExpressionData = ExpressionData::RawPointer {
    mutable: true,
    target: &U8_EXPRESSION,
};
static CONST_POINTER_EXPRESSION: ExpressionData = ExpressionData::RawPointer {
    mutable: false,
    target: &U8_EXPRESSION,
};
static SLICE_EXPRESSION: ExpressionData = ExpressionData::Slice(&U8_EXPRESSION);
static ARRAY_EXPRESSION: ExpressionData = ExpressionData::Array {
    element: &U8_EXPRESSION,
    length: ConstExpression::Parameter("N"),
};
static TUPLE_EXPRESSION: ExpressionData = ExpressionData::Tuple(&[&T_EXPRESSION, &U8_EXPRESSION]);
static UNIT_EXPRESSION: ExpressionData = ExpressionData::Tuple(&[]);
static SINGLE_TUPLE_EXPRESSION: ExpressionData = ExpressionData::Tuple(&[&T_EXPRESSION]);
static FUNCTION_POINTER: super::super::expression::FunctionPointerData =
    super::super::expression::FunctionPointerData {
        arguments: &[&T_EXPRESSION],
        result: Some(&U8_EXPRESSION),
        unsafe_: true,
        abi: Some("C"),
        variadic: false,
        lifetimes: &[],
    };
static FUNCTION_EXPRESSION: ExpressionData = ExpressionData::FunctionPointer(&FUNCTION_POINTER);
static PLAIN_FUNCTION_POINTER: super::super::expression::FunctionPointerData =
    super::super::expression::FunctionPointerData {
        arguments: &[],
        result: None,
        unsafe_: false,
        abi: None,
        variadic: true,
        lifetimes: &[LifetimeExpression::Named("a")],
    };
static PLAIN_FUNCTION_EXPRESSION: ExpressionData = ExpressionData::FunctionPointer(&PLAIN_FUNCTION_POINTER);
static DISPLAY_TRAIT: TraitExpressionData = TraitExpressionData {
    path: "core::fmt::Display",
    arguments: &[],
    lifetimes: &[],
};
static ITERATOR_TRAIT: TraitExpressionData = TraitExpressionData {
    path: "core::iter::Iterator",
    arguments: &[],
    lifetimes: &[],
};
static BOUNDED_ASSOCIATED_PREDICATE: PredicateData = PredicateData::TraitBound {
    ty: &T_EXPRESSION,
    trait_: &DISPLAY_TRAIT,
    modifier: TraitBoundModifier::Maybe,
};
static PREDICATES: [PredicateData; 5] = [
    PredicateData::LifetimeOutlives {
        lifetime: LifetimeExpression::Named("a"),
        outlives: LifetimeExpression::Static,
    },
    PredicateData::TypeOutlives {
        ty: &T_EXPRESSION,
        lifetime: LifetimeExpression::Placeholder,
    },
    PredicateData::TypeEquality {
        left: &T_EXPRESSION,
        right: &U8_EXPRESSION,
    },
    PredicateData::ConstEvaluatable(ConstExpression::Path("N + 1")),
    BOUNDED_ASSOCIATED_PREDICATE,
];
static DISPLAY_PREDICATE: PredicateData = PredicateData::TraitBound {
    ty: &T_EXPRESSION,
    trait_: &DISPLAY_TRAIT,
    modifier: TraitBoundModifier::None,
};
static BOUNDED_TYPE_PARAMETER: GenericParameterMetadata = GenericParameterMetadata {
    name: "T",
    index: 0,
    kind: GenericParameterMetadataKind::Type {
        bounds: &[DISPLAY_PREDICATE],
        default: None,
    },
};
static BOUNDED_GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata {
    parameters: &[BOUNDED_TYPE_PARAMETER],
    predicates: &[DISPLAY_PREDICATE],
};
static ALL_PREDICATES_GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata {
    parameters: &[],
    predicates: &PREDICATES,
};
static ASSOCIATED_EXPRESSION: ExpressionData = ExpressionData::AssociatedType {
    self_type: &T_EXPRESSION,
    trait_: Some(&ITERATOR_TRAIT),
    name: "Item",
};
static TRAIT_OBJECT_DATA: super::super::expression::TraitObjectData = super::super::expression::TraitObjectData {
    traits: &[DISPLAY_TRAIT],
    lifetime: Some(LifetimeExpression::Static),
};
static TRAIT_OBJECT_EXPRESSION: ExpressionData = ExpressionData::TraitObject(&TRAIT_OBJECT_DATA);
static OPAQUE_DATA: super::super::expression::OpaqueTypeData = super::super::expression::OpaqueTypeData {
    bounds: &[PredicateData::TraitBound {
        ty: &T_EXPRESSION,
        trait_: &DISPLAY_TRAIT,
        modifier: TraitBoundModifier::None,
    }],
};
static OPAQUE_EXPRESSION: ExpressionData = ExpressionData::Opaque(&OPAQUE_DATA);
static NEVER_EXPRESSION: ExpressionData = ExpressionData::Never;
static UNQUALIFIED_ASSOCIATED_EXPRESSION: ExpressionData = ExpressionData::AssociatedType {
    self_type: &T_EXPRESSION,
    trait_: None,
    name: "Output",
};
static ARGUMENT_EXPRESSION: ExpressionData = ExpressionData::Named {
    path: "fixture::Container",
    definition: None,
    arguments: &[
        GenericExpressionArgument::Lifetime(LifetimeExpression::Placeholder),
        GenericExpressionArgument::Const(ConstExpression::Character('x')),
        GenericExpressionArgument::AssociatedType {
            name: "Item",
            value: &U8_EXPRESSION,
        },
        GenericExpressionArgument::AssociatedTypeBound {
            name: "Error",
            bounds: &[PredicateData::LifetimeOutlives {
                lifetime: LifetimeExpression::Named("a"),
                outlives: LifetimeExpression::Elided,
            }],
        },
    ],
};
static CONSTRAINT_TRAIT_ARGUMENTS: [GenericExpressionArgument; 3] = [
    GenericExpressionArgument::Type(&U8_EXPRESSION),
    GenericExpressionArgument::AssociatedType {
        name: "Item",
        value: &T_EXPRESSION,
    },
    GenericExpressionArgument::AssociatedTypeBound {
        name: "Error",
        bounds: &[PredicateData::ConstEvaluatable(ConstExpression::Boolean(true))],
    },
];
static CONSTRAINT_TRAIT: TraitExpressionData = TraitExpressionData::new(
    "fixture::Trait",
    &CONSTRAINT_TRAIT_ARGUMENTS,
    &[LifetimeExpression::Named("a")],
);
static DISPLAY_PLACEHOLDER_REFERENCE: ExpressionData = ExpressionData::Reference {
    lifetime: Some(LifetimeExpression::Placeholder),
    mutable: true,
    target: &T_EXPRESSION,
};
static DISPLAY_ELIDED_REFERENCE: ExpressionData = ExpressionData::Reference {
    lifetime: Some(LifetimeExpression::Elided),
    mutable: false,
    target: &T_EXPRESSION,
};
static DISPLAY_SIGNED_ARRAY: ExpressionData = ExpressionData::Array {
    element: &U8_EXPRESSION,
    length: ConstExpression::SignedInteger(-2),
};
static DISPLAY_UNSIGNED_ARRAY: ExpressionData = ExpressionData::Array {
    element: &U8_EXPRESSION,
    length: ConstExpression::UnsignedInteger(42),
};
static DISPLAY_PARAMETER_ARRAY: ExpressionData = ExpressionData::Array {
    element: &U8_EXPRESSION,
    length: ConstExpression::Parameter("M"),
};
static DISPLAY_BOOL_ARRAY: ExpressionData = ExpressionData::Array {
    element: &U8_EXPRESSION,
    length: ConstExpression::Boolean(false),
};
static DISPLAY_CHAR_ARRAY: ExpressionData = ExpressionData::Array {
    element: &U8_EXPRESSION,
    length: ConstExpression::Character('\n'),
};
static DISPLAY_PATH_ARRAY: ExpressionData = ExpressionData::Array {
    element: &U8_EXPRESSION,
    length: ConstExpression::Path("mod::VALUE"),
};
static DISPLAY_SINGLE_TUPLE: ExpressionData = ExpressionData::Tuple(&[&T_EXPRESSION]);
static DISPLAY_FALLBACK_OPAQUE: ExpressionData = ExpressionData::Opaque(&OpaqueTypeData {
    bounds: &[
        PredicateData::LifetimeOutlives {
            lifetime: LifetimeExpression::Named("a"),
            outlives: LifetimeExpression::Static,
        },
        PredicateData::TypeOutlives {
            ty: &T_EXPRESSION,
            lifetime: LifetimeExpression::Named("a"),
        },
    ],
});
static GENERIC_ARGUMENTS: [GenericArgumentMetadata; 2] = [
    GenericArgumentMetadata {
        parameter: Some(&TYPE_PARAMETER),
        kind: GenericArgumentMetadataKind::Type {
            expression: &T_EXPRESSION,
            concrete: None,
        },
    },
    GenericArgumentMetadata {
        parameter: Some(&CONST_PARAMETER),
        kind: GenericArgumentMetadataKind::Const(ConstExpression::UnsignedInteger(16)),
    },
];
static DUPLICATE_ARGUMENTS: [GenericArgumentMetadata; 2] = [
    GenericArgumentMetadata {
        parameter: Some(&TYPE_PARAMETER),
        kind: GenericArgumentMetadataKind::Type {
            expression: &T_EXPRESSION,
            concrete: None,
        },
    },
    GenericArgumentMetadata {
        parameter: Some(&TYPE_PARAMETER),
        kind: GenericArgumentMetadataKind::Type {
            expression: &T_EXPRESSION,
            concrete: None,
        },
    },
];

#[test]
fn definition_and_type_expression_remain_source_level() {
    let context = crate::runtime::context::handle::builder::ReflectionContextBuilder::new()
        .register(
            crate::runtime::context::handle::registration::Registration::of::<EnvelopeU8>()
                .with_type_definition_metadata(&DEFINITION)
                .with_type_instance_id(TypeDefinitionId::of::<EnvelopeMarker>()),
        )
        .build()
        .expect("fixture context builds");
    let definition = TypeDefinitionDescriptor::new(&DEFINITION, &context);
    let generic_definition = definition.generics();
    let parameter = generic_definition.parameter("T").expect("T is declared");

    assert_eq!(definition.name(), "Envelope");
    assert_eq!(definition.path(), "fixture::Envelope");
    assert_eq!(parameter.name(), "T");
    assert_eq!(parameter.index(), 0);
    let instance = definition
        .instances()
        .expect("context supplies its compiled generic instances");
    assert_eq!(instance.len(), 1);
    assert_eq!(instance[0].type_id(), std::any::TypeId::of::<EnvelopeU8>());
    assert!(std::ptr::eq(instance[0].context(), &context));

    let expression = TypeExpression::new(&VEC_EXPRESSION, &context);
    assert!(matches!(expression.kind(), TypeExpressionKind::Named(_)));
    assert_eq!(expression.display().to_string(), "alloc::vec::Vec<T>");
    assert!(std::ptr::eq(expression.context(), &context));

    let definition_expression = TypeExpression::new(&DEFINITION_EXPRESSION, &context);
    let TypeExpressionKind::Named(named) = definition_expression.kind() else {
        panic!("named type expression expected");
    };
    assert_eq!(named.find_definition().map(|found| found.id()), Some(definition.id()));
}

#[test]
fn applications_keep_unreflected_type_arguments_and_const_values() {
    let context = ReflectionContext::empty();
    let arguments = GenericArguments::new(&GENERIC_ARGUMENTS, &context);

    assert_eq!(arguments.iter().len(), 2);
    let type_argument = arguments
        .argument("T")
        .expect("T argument is present")
        .type_argument()
        .expect("T is a type argument");
    assert!(type_argument.find_concrete_type().is_none());
    assert!(matches!(
        type_argument.concrete_type(),
        Err(crate::runtime::error::ReflectionError::TypeExpression(
            crate::runtime::expression::TypeExpressionError::ConcreteDescriptorUnavailable { parameter: "T" }
        ))
    ));
    assert!(matches!(
        type_argument.expression().kind(),
        TypeExpressionKind::Parameter(_)
    ));

    let const_argument = arguments
        .argument("N")
        .expect("N argument is present")
        .const_argument()
        .expect("N is a const argument");
    assert_eq!(const_argument.expression(), ConstExpression::UnsignedInteger(16));
}

#[test]
fn generic_lookup_reports_missing_parameters() {
    let context = ReflectionContext::empty();
    let definition = GenericDefinitionDescriptor::new(&GENERICS, &context);
    let error = match definition.parameter("Missing") {
        Ok(_) => panic!("unknown parameter unexpectedly matched"),
        Err(error) => error,
    };
    assert!(matches!(error, crate::runtime::error::LookupError::Missing { .. }));
}

#[test]
fn concrete_argument_lookup_reports_missing_and_ambiguous_names() {
    let context = ReflectionContext::empty();
    let arguments = GenericArguments::new(&GENERIC_ARGUMENTS, &context);
    let missing = match arguments.argument("Missing") {
        Ok(_) => panic!("unknown argument unexpectedly matched"),
        Err(error) => error,
    };
    assert!(matches!(
        missing,
        crate::runtime::error::LookupError::Missing {
            subject: crate::runtime::error::LookupSubject::Parameter,
            ..
        }
    ));

    let ambiguous = GenericArguments::new(&DUPLICATE_ARGUMENTS, &context);
    let error = match ambiguous.argument("T") {
        Ok(_) => panic!("duplicate argument names unexpectedly resolved"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        crate::runtime::error::LookupError::Ambiguous {
            subject: crate::runtime::error::LookupSubject::Parameter,
            candidates,
            ..
        } if candidates.len() == 2
    ));
}

#[test]
fn generic_lookup_reports_ambiguous_parameter_names() {
    let context = ReflectionContext::empty();
    let definition = GenericDefinitionDescriptor::new(&DUPLICATE_PARAMETERS, &context);
    let error = match definition.parameter("T") {
        Ok(_) => panic!("duplicate parameter names unexpectedly resolved"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        crate::runtime::error::LookupError::Ambiguous {
            subject: crate::runtime::error::LookupSubject::Parameter,
            ..
        }
    ));
}

#[test]
fn generic_argument_kind_mismatch_is_reported() {
    let context = ReflectionContext::empty();
    let arguments = GenericArguments::new(&GENERIC_ARGUMENTS, &context);
    let error = match arguments.argument("N").expect("N argument is present").type_argument() {
        Ok(_) => panic!("const argument unexpectedly produced a type descriptor"),
        Err(error) => error,
    };
    let crate::runtime::error::ReflectionError::GenericArgument(error) = error else {
        panic!("expected a generic argument kind error")
    };

    assert_eq!(error.expected(), crate::runtime::expression::GenericArgumentKind::Type);
    assert_eq!(error.actual(), crate::runtime::expression::GenericArgumentKind::Const);

    let error = match arguments.argument("T").expect("T argument is present").const_argument() {
        Ok(_) => panic!("type argument unexpectedly produced a const descriptor"),
        Err(error) => error,
    };
    let crate::runtime::error::ReflectionError::GenericArgument(error) = error else {
        panic!("expected a generic argument kind error")
    };
    assert_eq!(error.expected(), crate::runtime::expression::GenericArgumentKind::Const);
    assert_eq!(error.actual(), crate::runtime::expression::GenericArgumentKind::Type);
}

#[test]
fn lifetime_parameters_stay_on_definitions_and_out_of_applications() {
    let context = ReflectionContext::empty();
    let definition = GenericDefinitionDescriptor::new(&LIFETIME_GENERICS, &context);
    assert!(matches!(
        definition.parameter("a").expect("lifetime is declared").kind(),
        crate::runtime::descriptor::generic::GenericParameterKind::Lifetime { .. }
    ));

    let arguments = GenericArguments::new(&GENERIC_ARGUMENTS, &context);
    assert!(arguments.get("a").is_none());
    assert!(arguments.get("T").is_some());
}

#[test]
fn type_expression_preserves_composite_source_shapes() {
    let context = ReflectionContext::empty();
    let cases = [
        (&REFERENCE_EXPRESSION, "&'a T"),
        (&POINTER_EXPRESSION, "*mut u8"),
        (&SLICE_EXPRESSION, "[u8]"),
        (&ARRAY_EXPRESSION, "[u8; N]"),
        (&TUPLE_EXPRESSION, "(T, u8)"),
        (&FUNCTION_EXPRESSION, "unsafe extern \"C\" fn(T) -> u8"),
        (&ASSOCIATED_EXPRESSION, "<T as core::iter::Iterator>::Item"),
        (&TRAIT_OBJECT_EXPRESSION, "dyn core::fmt::Display + 'static"),
        (&OPAQUE_EXPRESSION, "impl core::fmt::Display"),
        (&NEVER_EXPRESSION, "!"),
    ];

    for (data, expected) in cases {
        let expression = TypeExpression::new(data, &context);
        assert_eq!(expression.display().to_string(), expected);
    }
}

#[test]
fn predicates_preserve_trait_bound_facts_without_solving_them() {
    let context = ReflectionContext::empty();
    let definition = GenericDefinitionDescriptor::new(&BOUNDED_GENERICS, &context);
    let predicates = definition.predicates();
    assert_eq!(predicates.iter().len(), 1);
    assert!(matches!(
        predicates.iter().next().expect("predicate is present").kind(),
        PredicateKind::TraitBound { ty, trait_, .. }
            if matches!(ty.kind(), TypeExpressionKind::Parameter(_))
                && trait_.path() == "core::fmt::Display"
    ));
}

#[test]
fn type_expression_kinds_expose_every_structural_node_and_context() {
    let context = ReflectionContext::empty();

    assert!(matches!(
        TypeExpression::new(&T_EXPRESSION, &context).kind(),
        TypeExpressionKind::Parameter(parameter) if parameter.name() == "T"
    ));
    assert!(matches!(
        TypeExpression::new(&UNLINKED_PARAMETER_EXPRESSION, &context).kind(),
        TypeExpressionKind::ParameterName("U")
    ));
    assert!(matches!(
        TypeExpression::new(&SELF_EXPRESSION, &context).kind(),
        TypeExpressionKind::SelfType
    ));

    let TypeExpressionKind::Named(named) = TypeExpression::new(&VEC_EXPRESSION, &context).kind() else {
        panic!("named expression expected");
    };
    assert_eq!(named.path(), "alloc::vec::Vec");
    assert_eq!(named.arguments().len(), 1);
    assert!(named.find_definition().is_none());
    assert!(std::ptr::eq(named.context(), &context));

    let TypeExpressionKind::Reference {
        lifetime,
        mutable,
        target,
    } = TypeExpression::new(&REFERENCE_EXPRESSION, &context).kind()
    else {
        panic!("reference expression expected");
    };
    assert_eq!(lifetime, Some(LifetimeExpression::Named("a")));
    assert!(!mutable);
    assert!(matches!(target.kind(), TypeExpressionKind::Parameter(_)));
    assert!(std::ptr::eq(target.context(), &context));
    for (expression, expected_mutability) in [(&POINTER_EXPRESSION, true), (&CONST_POINTER_EXPRESSION, false)] {
        let TypeExpressionKind::RawPointer { mutable, target } = TypeExpression::new(expression, &context).kind()
        else {
            panic!("raw pointer expression expected");
        };
        assert_eq!(mutable, expected_mutability);
        assert!(matches!(target.kind(), TypeExpressionKind::Named(_)));
    }
    assert!(matches!(
        TypeExpression::new(&SLICE_EXPRESSION, &context).kind(),
        TypeExpressionKind::Slice(element) if matches!(element.kind(), TypeExpressionKind::Named(_))
    ));
    assert!(matches!(
        TypeExpression::new(&ARRAY_EXPRESSION, &context).kind(),
        TypeExpressionKind::Array {
            length: ConstExpression::Parameter("N"),
            ..
        }
    ));
    assert!(matches!(
        TypeExpression::new(&UNIT_EXPRESSION, &context).kind(),
        TypeExpressionKind::Tuple(elements) if elements.is_empty()
    ));
    assert!(matches!(
        TypeExpression::new(&SINGLE_TUPLE_EXPRESSION, &context).kind(),
        TypeExpressionKind::Tuple(elements) if elements.len() == 1
    ));
    assert!(matches!(
        TypeExpression::new(&ASSOCIATED_EXPRESSION, &context).kind(),
        TypeExpressionKind::AssociatedType(projection)
            if projection.name() == "Item"
                && projection.trait_().map(|trait_| trait_.path()) == Some("core::iter::Iterator")
                && std::ptr::eq(projection.context(), &context)
    ));
    assert!(matches!(
        TypeExpression::new(&UNQUALIFIED_ASSOCIATED_EXPRESSION, &context).kind(),
        TypeExpressionKind::AssociatedType(projection)
            if projection.name() == "Output" && projection.trait_().is_none()
    ));
    assert!(matches!(
        TypeExpression::new(&FUNCTION_EXPRESSION, &context).kind(),
        TypeExpressionKind::FunctionPointer(function)
            if function.is_unsafe()
                && function.abi() == Some("C")
                && !function.is_variadic()
                && function.arguments().len() == 1
                && function.result().is_some()
                && function.lifetimes().is_empty()
                && std::ptr::eq(function.context(), &context)
    ));
    assert!(matches!(
        TypeExpression::new(&TRAIT_OBJECT_EXPRESSION, &context).kind(),
        TypeExpressionKind::TraitObject(object)
            if object.traits().len() == 1
                && object.traits()[0].path() == "core::fmt::Display"
                && object.lifetime() == Some(LifetimeExpression::Static)
                && std::ptr::eq(object.context(), &context)
    ));
    assert!(matches!(
        TypeExpression::new(&OPAQUE_EXPRESSION, &context).kind(),
        TypeExpressionKind::Opaque(opaque)
            if opaque.bounds().len() == 1 && std::ptr::eq(opaque.context(), &context)
    ));
    assert!(matches!(
        TypeExpression::new(&NEVER_EXPRESSION, &context).kind(),
        TypeExpressionKind::Never
    ));
}

#[test]
fn generic_expression_arguments_and_associated_constraints_keep_source_order() {
    let context = ReflectionContext::empty();
    let TypeExpressionKind::Named(named) = TypeExpression::new(&ARGUMENT_EXPRESSION, &context).kind() else {
        panic!("named expression expected");
    };
    let arguments = named.arguments();
    assert_eq!(arguments.len(), 4);
    assert!(!arguments.is_empty());
    assert!(arguments.get(4).is_none());
    assert!(std::ptr::eq(arguments.context(), &context));

    let mut iter = arguments.iter();
    assert_eq!(iter.len(), 4);
    assert!(matches!(
        iter.next().expect("lifetime argument is present").kind(),
        crate::runtime::expression::GenericExpressionArgumentKind::Lifetime(LifetimeExpression::Placeholder)
    ));
    assert!(matches!(
        iter.next().expect("const argument is present").kind(),
        crate::runtime::expression::GenericExpressionArgumentKind::Const(ConstExpression::Character('x'))
    ));
    assert!(matches!(
        iter.next().expect("associated type equality is present").kind(),
        crate::runtime::expression::GenericExpressionArgumentKind::AssociatedType { name: "Item", value }
            if value.display().to_string() == "u8"
                && std::ptr::eq(value.context(), &context)
    ));
    let bounds = iter.next().expect("associated type bound is present");
    assert!(matches!(
        bounds.kind(),
        crate::runtime::expression::GenericExpressionArgumentKind::AssociatedTypeBound { name: "Error", bounds }
            if bounds.len() == 1 && std::ptr::eq(bounds.context(), &context)
    ));
    assert!(iter.next().is_none());

    let trait_expression = crate::runtime::expression::TraitExpression::new(&CONSTRAINT_TRAIT, &context);
    assert_eq!(trait_expression.path(), "fixture::Trait");
    assert!(trait_expression.find_definition().is_none());
    assert_eq!(trait_expression.lifetimes(), &[LifetimeExpression::Named("a")]);
    assert_eq!(trait_expression.arguments().len(), 3);
    assert!(std::ptr::eq(trait_expression.context(), &context));
    let constraints = trait_expression.associated_type_constraints();
    assert_eq!(constraints.len(), 2);
    assert!(!constraints.is_empty());
    let mut constraints = constraints.iter();
    let equality = constraints.next().expect("Item equality is present");
    assert_eq!(equality.name(), "Item");
    assert!(matches!(
        equality.kind(),
        crate::runtime::expression::AssociatedTypeConstraintKind::Equality(ty)
            if ty.display().to_string() == "T"
    ));
    let bounds = constraints.next().expect("Error bounds are present");
    assert_eq!(bounds.name(), "Error");
    assert!(matches!(
        bounds.kind(),
        crate::runtime::expression::AssociatedTypeConstraintKind::Bounds(predicates)
            if predicates.len() == 1
    ));
    assert!(constraints.next().is_none());
    let no_constraints =
        crate::runtime::expression::TraitExpression::new(&DISPLAY_TRAIT, &context).associated_type_constraints();
    assert!(no_constraints.is_empty());
    assert_eq!(no_constraints.iter().count(), 0);
}

#[test]
fn function_pointer_and_nested_bounds_expose_signature_facts() {
    let context = ReflectionContext::empty();
    let TypeExpressionKind::FunctionPointer(function) =
        TypeExpression::new(&PLAIN_FUNCTION_EXPRESSION, &context).kind()
    else {
        panic!("function pointer expected");
    };
    assert!(function.arguments().is_empty());
    assert!(function.result().is_none());
    assert!(!function.is_unsafe());
    assert_eq!(function.abi(), None);
    assert!(function.is_variadic());
    assert_eq!(function.lifetimes(), &[LifetimeExpression::Named("a")]);

    let TypeExpressionKind::Opaque(opaque) = TypeExpression::new(&OPAQUE_EXPRESSION, &context).kind() else {
        panic!("opaque expression expected");
    };
    let predicate = opaque.bounds().iter().next().expect("trait bound is present");
    assert!(matches!(
        predicate.kind(),
        PredicateKind::TraitBound {
            modifier: TraitBoundModifier::None,
            ..
        }
    ));
}

#[test]
fn every_predicate_kind_preserves_its_structural_facts() {
    let context = ReflectionContext::empty();
    let predicates = GenericDefinitionDescriptor::new(&ALL_PREDICATES_GENERICS, &context).predicates();
    assert_eq!(predicates.len(), 5);
    assert!(!predicates.is_empty());
    assert!(std::ptr::eq(predicates.context(), &context));
    let mut iter = predicates.iter();
    assert_eq!(iter.len(), 5);

    let PredicateKind::LifetimeOutlives { lifetime, outlives } =
        iter.next().expect("lifetime outlives predicate is present").kind()
    else {
        panic!("lifetime outlives predicate expected");
    };
    assert_eq!(lifetime, LifetimeExpression::Named("a"));
    assert_eq!(outlives, LifetimeExpression::Static);

    let PredicateKind::TypeOutlives { ty, lifetime } = iter.next().expect("type outlives predicate is present").kind()
    else {
        panic!("type outlives predicate expected");
    };
    assert!(matches!(ty.kind(), TypeExpressionKind::Parameter(_)));
    assert_eq!(lifetime, LifetimeExpression::Placeholder);
    assert!(std::ptr::eq(ty.context(), &context));

    let PredicateKind::TypeEquality { left, right } = iter.next().expect("type equality predicate is present").kind()
    else {
        panic!("type equality predicate expected");
    };
    assert_eq!(left.display().to_string(), "T");
    assert_eq!(right.display().to_string(), "u8");

    assert!(matches!(
        iter.next().expect("const predicate is present").kind(),
        PredicateKind::ConstEvaluatable(ConstExpression::Path("N + 1"))
    ));
    assert!(matches!(
        iter.next().expect("trait predicate is present").kind(),
        PredicateKind::TraitBound { modifier: TraitBoundModifier::Maybe, trait_, .. }
            if trait_.path() == "core::fmt::Display"
                && std::ptr::eq(trait_.context(), &context)
    ));
    assert!(iter.next().is_none());

    let empty = GenericDefinitionDescriptor::new(&GENERICS, &context).predicates();
    assert!(empty.is_empty());
    assert_eq!(empty.iter().len(), 0);
}

#[test]
fn expression_display_formats_lifetimes_consts_and_argument_forms() {
    let context = ReflectionContext::empty();
    let cases = [
        (&DISPLAY_PLACEHOLDER_REFERENCE, "&'_ mut T"),
        (&DISPLAY_ELIDED_REFERENCE, "&T"),
        (&DISPLAY_SIGNED_ARRAY, "[u8; -2]"),
        (&DISPLAY_UNSIGNED_ARRAY, "[u8; 42]"),
        (&DISPLAY_PARAMETER_ARRAY, "[u8; M]"),
        (&DISPLAY_BOOL_ARRAY, "[u8; false]"),
        (&DISPLAY_CHAR_ARRAY, "[u8; '\\n']"),
        (&DISPLAY_PATH_ARRAY, "[u8; mod::VALUE]"),
    ];
    for (data, expected) in cases {
        assert_eq!(TypeExpression::new(data, &context).display().to_string(), expected);
    }
    assert_eq!(
        TypeExpression::new(&ARGUMENT_EXPRESSION, &context)
            .display()
            .to_string(),
        "fixture::Container<'_, 'x', Item = u8, Error: …>"
    );
    assert_eq!(
        TypeExpression::new(&DISPLAY_SINGLE_TUPLE, &context)
            .display()
            .to_string(),
        "(T,)"
    );
    assert_eq!(
        TypeExpression::new(&DISPLAY_FALLBACK_OPAQUE, &context)
            .display()
            .to_string(),
        "impl 'a + _"
    );
}

#[test]
fn expression_errors_expose_kinds_messages_and_error_contract() {
    let context = ReflectionContext::empty();
    let arguments = GenericArguments::new(&GENERIC_ARGUMENTS, &context);
    let error = match arguments.argument("N").expect("const argument exists").type_argument() {
        Ok(_) => panic!("const argument cannot be viewed as a type"),
        Err(error) => error,
    };
    let crate::runtime::error::ReflectionError::GenericArgument(error) = error else {
        panic!("generic argument mismatch error expected");
    };
    assert_eq!(error.expected(), crate::runtime::expression::GenericArgumentKind::Type);
    assert_eq!(error.actual(), crate::runtime::expression::GenericArgumentKind::Const);
    assert_eq!(error.to_string(), "expected Type generic argument, found Const");
    assert!(std::error::Error::source(&error).is_none());

    let type_error = match arguments
        .argument("T")
        .expect("type argument exists")
        .type_argument()
        .expect("T is a type argument")
        .concrete_type()
    {
        Ok(_) => panic!("missing descriptor cannot produce a concrete type"),
        Err(crate::runtime::error::ReflectionError::TypeExpression(error)) => error,
        Err(other) => panic!("unexpected reflection error: {other}"),
    };
    assert_eq!(
        type_error,
        crate::runtime::expression::TypeExpressionError::ConcreteDescriptorUnavailable { parameter: "T" }
    );
    assert_eq!(
        type_error.to_string(),
        "generic type parameter T has no reflected descriptor"
    );
    assert!(std::error::Error::source(&type_error).is_none());
}
