use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::generic::{
    GenericArgumentMetadata, GenericArgumentMetadataKind, GenericArguments,
    GenericDefinitionDescriptor, GenericDefinitionMetadata, GenericParameterMetadata,
    GenericParameterMetadataKind, TypeDefinitionDescriptor, TypeDefinitionId, TypeDefinitionKind,
};
use crate::runtime::expression::{
    ConstExpression, ExpressionData, GenericExpressionArgument, LifetimeExpression, PredicateData,
    PredicateKind, TraitBoundModifier, TraitExpressionData, TypeExpression, TypeExpressionKind,
};

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
static REFERENCE_EXPRESSION: ExpressionData = ExpressionData::Reference {
    lifetime: Some(LifetimeExpression::Named("a")),
    mutable: false,
    target: &T_EXPRESSION,
};
static POINTER_EXPRESSION: ExpressionData = ExpressionData::RawPointer {
    mutable: true,
    target: &U8_EXPRESSION,
};
static SLICE_EXPRESSION: ExpressionData = ExpressionData::Slice(&U8_EXPRESSION);
static ARRAY_EXPRESSION: ExpressionData = ExpressionData::Array {
    element: &U8_EXPRESSION,
    length: ConstExpression::Parameter("N"),
};
static TUPLE_EXPRESSION: ExpressionData = ExpressionData::Tuple(&[&T_EXPRESSION, &U8_EXPRESSION]);
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
static ASSOCIATED_EXPRESSION: ExpressionData = ExpressionData::AssociatedType {
    self_type: &T_EXPRESSION,
    trait_: Some(&ITERATOR_TRAIT),
    name: "Item",
};
static TRAIT_OBJECT_DATA: super::super::expression::TraitObjectData =
    super::super::expression::TraitObjectData {
        traits: &[DISPLAY_TRAIT],
        lifetime: Some(LifetimeExpression::Static),
    };
static TRAIT_OBJECT_EXPRESSION: ExpressionData = ExpressionData::TraitObject(&TRAIT_OBJECT_DATA);
static OPAQUE_DATA: super::super::expression::OpaqueTypeData =
    super::super::expression::OpaqueTypeData {
        bounds: &[PredicateData::TraitBound {
            ty: &T_EXPRESSION,
            trait_: &DISPLAY_TRAIT,
            modifier: TraitBoundModifier::None,
        }],
    };
static OPAQUE_EXPRESSION: ExpressionData = ExpressionData::Opaque(&OPAQUE_DATA);
static NEVER_EXPRESSION: ExpressionData = ExpressionData::Never;
static GENERIC_ARGUMENTS: [GenericArgumentMetadata; 2] = [
    GenericArgumentMetadata {
        parameter: &TYPE_PARAMETER,
        kind: GenericArgumentMetadataKind::Type {
            expression: &T_EXPRESSION,
            concrete: None,
        },
    },
    GenericArgumentMetadata {
        parameter: &CONST_PARAMETER,
        kind: GenericArgumentMetadataKind::Const(ConstExpression::UnsignedInteger(16)),
    },
];
static DUPLICATE_ARGUMENTS: [GenericArgumentMetadata; 2] = [
    GenericArgumentMetadata {
        parameter: &TYPE_PARAMETER,
        kind: GenericArgumentMetadataKind::Type {
            expression: &T_EXPRESSION,
            concrete: None,
        },
    },
    GenericArgumentMetadata {
        parameter: &TYPE_PARAMETER,
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
    assert_eq!(
        named.find_definition().map(|found| found.id()),
        Some(definition.id())
    );
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
        Err(
            crate::runtime::expression::TypeExpressionError::ConcreteDescriptorUnavailable {
                parameter: "T"
            }
        )
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
    assert_eq!(
        const_argument.expression(),
        ConstExpression::UnsignedInteger(16)
    );
}

#[test]
fn generic_lookup_reports_missing_parameters() {
    let context = ReflectionContext::empty();
    let definition = GenericDefinitionDescriptor::new(&GENERICS, &context);
    let error = match definition.parameter("Missing") {
        Ok(_) => panic!("unknown parameter unexpectedly matched"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        crate::runtime::error::LookupError::Missing { .. }
    ));
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
    let error = match arguments
        .argument("N")
        .expect("N argument is present")
        .type_argument()
    {
        Ok(_) => panic!("const argument unexpectedly produced a type descriptor"),
        Err(error) => error,
    };

    assert_eq!(
        error.expected(),
        crate::runtime::expression::GenericArgumentKind::Type
    );
    assert_eq!(
        error.actual(),
        crate::runtime::expression::GenericArgumentKind::Const
    );

    let error = match arguments
        .argument("T")
        .expect("T argument is present")
        .const_argument()
    {
        Ok(_) => panic!("type argument unexpectedly produced a const descriptor"),
        Err(error) => error,
    };
    assert_eq!(
        error.expected(),
        crate::runtime::expression::GenericArgumentKind::Const
    );
    assert_eq!(
        error.actual(),
        crate::runtime::expression::GenericArgumentKind::Type
    );
}

#[test]
fn lifetime_parameters_stay_on_definitions_and_out_of_applications() {
    let context = ReflectionContext::empty();
    let definition = GenericDefinitionDescriptor::new(&LIFETIME_GENERICS, &context);
    assert!(matches!(
        definition
            .parameter("a")
            .expect("lifetime is declared")
            .kind(),
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
