// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::Reflect;
use crate::runtime::descriptor::TypeRegistration;
use crate::runtime::descriptor::generic::GenericDefinitionMetadata;
use crate::runtime::descriptor::generic::GenericParameterMetadata;
use crate::runtime::descriptor::method::EffectiveMethod;
use crate::runtime::descriptor::method::FunctionAbi;
use crate::runtime::descriptor::method::InvocationAdapters;
use crate::runtime::descriptor::method::MethodArguments;
use crate::runtime::descriptor::method::MethodAvailability;
use crate::runtime::descriptor::method::MethodDescriptor;
use crate::runtime::descriptor::method::MethodImplementationSource;
use crate::runtime::descriptor::method::MethodMetadata;
use crate::runtime::descriptor::method::MethodMetadataSpec;
use crate::runtime::descriptor::method::MethodNamespaceMetadata;
use crate::runtime::descriptor::method::MethodQualifiers;
use crate::runtime::descriptor::method::MethodSpecializationArgument;
use crate::runtime::descriptor::method::MethodSpecializationSpec;
use crate::runtime::descriptor::method::MethodVisibility;
use crate::runtime::descriptor::method::NO_ADAPTERS;
use crate::runtime::descriptor::method::ParameterMetadata;
use crate::runtime::descriptor::method::ParameterPattern;
use crate::runtime::descriptor::method::PassingMode;
use crate::runtime::descriptor::method::ReceiverDescriptor;
use crate::runtime::descriptor::method::ReceiverKind;
use crate::runtime::descriptor::method::ReturnKind;
use crate::runtime::descriptor::method::ReturnMetadata;
use crate::runtime::descriptor::method::imports::ExpressionData;
use crate::runtime::descriptor::method::imports::InvocationAdapter;
use crate::runtime::descriptor::method::imports::InvocationOutput;
use crate::runtime::descriptor::method::imports::Local;
use crate::runtime::error::LookupError;
use crate::runtime::invoke::Invocation;
use crate::runtime::invoke::InvocationErrorKind;
use crate::runtime::invoke::PinnedFunctionInvocationAdapter;
use crate::runtime::invoke::PinnedInvocation;
use crate::runtime::invoke::PinnedReceiver;
use crate::runtime::invoke::ThreadSafe;
use crate::runtime::invoke::ValidatedInvocation;

struct Target;

impl Reflect for Target {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        static REGISTRATION: TypeRegistration<Target> = TypeRegistration::new();
        &REGISTRATION
    }
}

struct ResultType;

impl Reflect for ResultType {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        static REGISTRATION: TypeRegistration<ResultType> = TypeRegistration::new();
        &REGISTRATION
    }
}

static TYPE_EXPRESSION: ExpressionData = ExpressionData::Named {
    path: "ResultType",
    definition: None,
    arguments: &[],
};
static U32_EXPRESSION: ExpressionData = ExpressionData::Named {
    path: "u32",
    definition: None,
    arguments: &[],
};
static PARAMETER: ParameterMetadata = ParameterMetadata::new(
    0,
    ParameterPattern::Identifier("value"),
    PassingMode::Shared,
    &TYPE_EXPRESSION,
)
.with_concrete_type::<ResultType>();
static PARAMETERS: [ParameterMetadata; 2] = [
    PARAMETER,
    ParameterMetadata::new(1, ParameterPattern::Wildcard, PassingMode::Owned, &TYPE_EXPRESSION),
];
static DUPLICATE_PARAMETERS: [ParameterMetadata; 2] = [
    ParameterMetadata::new(
        0,
        ParameterPattern::Identifier("same"),
        PassingMode::Owned,
        &TYPE_EXPRESSION,
    ),
    ParameterMetadata::new(
        1,
        ParameterPattern::Identifier("same"),
        PassingMode::Owned,
        &TYPE_EXPRESSION,
    ),
];
static OUTPUT: ReturnMetadata =
    ReturnMetadata::new(ReturnKind::Owned, &TYPE_EXPRESSION).with_concrete_type::<ResultType>();
static GENERIC_PARAMETERS: [GenericParameterMetadata; 1] =
    [GenericParameterMetadata::type_parameter("T", 0, &[], None)];
static CONST_GENERIC_PARAMETERS: [GenericParameterMetadata; 1] = [GenericParameterMetadata::const_parameter(
    "N",
    0,
    &TYPE_EXPRESSION,
    None,
)];
static CONST_GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata {
    parameters: &CONST_GENERIC_PARAMETERS,
    predicates: &[],
};
static CONST_FOR_TYPE_ARGUMENT: [MethodSpecializationArgument; 1] = [MethodSpecializationArgument::u8(1)];
static TYPE_FOR_CONST_ARGUMENT: [MethodSpecializationArgument; 1] =
    [MethodSpecializationArgument::type_argument::<u32>()];
static NO_SPECIALIZATION_ARGUMENTS: [MethodSpecializationArgument; 0] = [];
static GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata {
    parameters: &GENERIC_PARAMETERS,
    predicates: &[],
};
static METHOD: MethodMetadata = MethodMetadata::from_spec(MethodMetadataSpec::new(
    "inspect",
    MethodNamespaceMetadata::Inherent {
        implementation: "Target::inspect",
    },
    MethodVisibility::Public,
    Some(ReceiverDescriptor::for_type::<Target>(ReceiverKind::Shared).with_expression(&TYPE_EXPRESSION)),
    &PARAMETERS,
    &OUTPUT,
    MethodQualifiers::new(true, true, false, false).with_abi(Some(FunctionAbi::C)),
    &GENERICS,
    true,
    false,
));

fn descriptor(metadata: &'static MethodMetadata) -> MethodDescriptor<'static> {
    let context = Box::leak(Box::new(ReflectionContext::empty()));
    MethodDescriptor::new(EffectiveMethod::for_type::<Target>(metadata), context)
}

#[test]
fn test_descriptor_exposes_receiver_parameters_qualifiers_and_return_metadata() {
    let method = descriptor(&METHOD);
    assert_eq!(method.name(), "inspect");
    assert_eq!(method.target().type_id(), std::any::TypeId::of::<Target>());
    assert_eq!(method.implementation_source(), MethodImplementationSource::Declared);
    assert_eq!(
        method.visibility(),
        crate::runtime::descriptor::trait_descriptor::Visibility::Public
    );
    assert_eq!(method.availability(), MethodAvailability::RequiresSpecialization);

    let receiver = method.receiver().expect("declared receiver");
    assert_eq!(receiver.kind(), ReceiverKind::Shared);
    assert_eq!(
        receiver.type_metadata.expect("bound receiver type")().type_id(),
        std::any::TypeId::of::<Target>()
    );
    assert!(receiver.expression.is_some());

    let parameters = method.parameters();
    assert_eq!(parameters.len(), 2);
    assert!(!parameters.is_empty());
    assert_eq!(parameters.context().id(), method.context().id());
    let mut iter = parameters.iter();
    let first = iter.next().expect("first parameter");
    assert_eq!(first.index(), 0);
    assert_eq!(first.name(), Some("value"));
    assert_eq!(first.pattern(), ParameterPattern::Identifier("value"));
    assert_eq!(first.passing(), PassingMode::Shared);
    assert_eq!(first.context().id(), method.context().id());
    assert!(first.find_concrete_type().is_some());
    assert_eq!(first.type_expression().display().to_string(), "ResultType");
    assert_eq!(parameters.get_at(1).expect("second parameter").name(), None);
    assert!(parameters.get_at(2).is_none());
    assert!(parameters.get("value").is_some());
    assert!(parameters.get("missing").is_none());
    assert!(iter.next().is_some());
    assert!(iter.next().is_none());

    let output = method.output();
    assert_eq!(output.context().id(), method.context().id());
    assert_eq!(output.kind(), ReturnKind::Owned);
    assert_eq!(
        output
            .type_expression()
            .expect("return expression")
            .display()
            .to_string(),
        "ResultType"
    );
    assert!(output.find_concrete_type().is_some());

    let qualifiers = method.qualifiers();
    assert!(qualifiers.is_const);
    assert!(qualifiers.is_async);
    assert!(!qualifiers.is_unsafe);
    assert_eq!(qualifiers.abi, Some(FunctionAbi::C));
    assert!(method.generics().parameter("T").is_ok());
}

#[test]
fn test_parameter_name_lookup_reports_missing_and_duplicate_identifiers() {
    let method = descriptor(&METHOD);
    assert!(matches!(
        method.parameter("absent"),
        Err(LookupError::Missing { subject, .. }) if subject == crate::runtime::error::LookupSubject::Parameter
    ));

    static DUPLICATE_METHOD: MethodMetadata = MethodMetadata::from_spec(MethodMetadataSpec::new(
        "duplicates",
        MethodNamespaceMetadata::Inherent {
            implementation: "Target::duplicates",
        },
        MethodVisibility::Public,
        None,
        &DUPLICATE_PARAMETERS,
        &ReturnMetadata::unit(),
        MethodQualifiers::new(false, false, false, false),
        &GenericDefinitionMetadata {
            parameters: &[],
            predicates: &[],
        },
        false,
        false,
    ));
    assert!(matches!(
        descriptor(&DUPLICATE_METHOD).parameter("same"),
        Err(LookupError::Ambiguous { subject, candidates, .. })
            if subject == crate::runtime::error::LookupSubject::Parameter && candidates.len() == 2
    ));
    assert!(descriptor(&DUPLICATE_METHOD).parameters().get("same").is_none());
}

struct UnitAdapter;

impl InvocationAdapter<Local> for UnitAdapter {
    fn invoke<'call>(&self, _invocation: ValidatedInvocation<'call, Local>) -> InvocationOutput<'call, Local> {
        InvocationOutput::owned(37_u32)
    }
}

static UNIT_ADAPTER: UnitAdapter = UnitAdapter;
static ADAPTERS: InvocationAdapters = InvocationAdapters::builder().local(&UNIT_ADAPTER).build();
static CALLABLE_RECEIVER: ReceiverDescriptor = ReceiverDescriptor::for_type::<Target>(ReceiverKind::Shared);
static ADAPTER_OUTPUT: ReturnMetadata =
    ReturnMetadata::new(ReturnKind::Owned, &U32_EXPRESSION).with_concrete_type::<u32>();

struct ThreadSafeAdapter;

impl InvocationAdapter<ThreadSafe> for ThreadSafeAdapter {
    fn invoke<'call>(
        &self,
        _invocation: ValidatedInvocation<'call, ThreadSafe>,
    ) -> InvocationOutput<'call, ThreadSafe> {
        InvocationOutput::owned(37_u32)
    }
}

static THREAD_SAFE_ADAPTER: ThreadSafeAdapter = ThreadSafeAdapter;
static THREAD_SAFE_ONLY_ADAPTERS: InvocationAdapters =
    InvocationAdapters::builder().thread_safe(&THREAD_SAFE_ADAPTER).build();

fn pinned_unit_adapter<'call>(
    _receiver: PinnedReceiver<'call>,
    _invocation: ValidatedInvocation<'call, Local>,
) -> InvocationOutput<'call, Local> {
    InvocationOutput::owned(41_u32)
}

static PINNED_LOCAL_ADAPTER: PinnedFunctionInvocationAdapter<Local, Target> =
    PinnedFunctionInvocationAdapter::shared(pinned_unit_adapter);
static PINNED_ADAPTERS: InvocationAdapters = InvocationAdapters::builder()
    .pinned_local(&PINNED_LOCAL_ADAPTER)
    .build();

#[test]
fn test_invocation_adapter_runs_and_missing_adapter_is_a_typed_preflight_error() {
    static CALLABLE_METHOD: MethodMetadata = MethodMetadata::from_spec(MethodMetadataSpec::new(
        "value",
        MethodNamespaceMetadata::Inherent {
            implementation: "Target::value",
        },
        MethodVisibility::Public,
        Some(CALLABLE_RECEIVER),
        &[],
        &ADAPTER_OUTPUT,
        MethodQualifiers::new(false, false, false, false),
        &GenericDefinitionMetadata {
            parameters: &[],
            predicates: &[],
        },
        false,
        false,
    ));
    let callable = EffectiveMethod::for_type::<Target>(&CALLABLE_METHOD).with_adapters(ADAPTERS);
    let method = MethodDescriptor::new(callable, Box::leak(Box::new(ReflectionContext::empty())));
    let target = Target;
    let output = match method.invoke(Invocation::<Local>::on_ref(&target)) {
        Ok(output) => output,
        Err(_) => panic!("the registered adapter is callable"),
    };
    let value = match output.into_owned::<u32>() {
        Ok(value) => value,
        Err(_) => panic!("adapter returns a typed result"),
    };
    assert_eq!(value, 37);

    let unavailable = MethodDescriptor::new(
        EffectiveMethod::for_type::<Target>(&CALLABLE_METHOD).with_adapters(THREAD_SAFE_ONLY_ADAPTERS),
        Box::leak(Box::new(ReflectionContext::empty())),
    );
    let failure = match unavailable.invoke(Invocation::<Local>::on_ref(&target)) {
        Err(failure) => failure,
        Ok(_) => panic!("no adapter is registered"),
    };
    assert_eq!(*failure.error().kind(), InvocationErrorKind::ValueModeUnavailable);
}

#[test]
fn test_specialization_resolution_rejects_argument_kind_and_arity_mismatches() {
    let empty_adapters = InvocationAdapters::builder().build();
    let error =
        match MethodSpecializationSpec::new(&CONST_FOR_TYPE_ARGUMENT, empty_adapters).resolve(&GENERIC_PARAMETERS) {
            Ok(_) => panic!("const argument cannot bind a type parameter"),
            Err(error) => error,
        };
    assert_eq!(error, "const specialization argument does not target a const parameter");

    let error = match MethodSpecializationSpec::new(&TYPE_FOR_CONST_ARGUMENT, empty_adapters)
        .resolve(&CONST_GENERIC_PARAMETERS)
    {
        Ok(_) => panic!("type argument cannot bind a const parameter"),
        Err(error) => error,
    };
    assert_eq!(error, "type specialization argument does not target a type parameter");

    let error = match MethodSpecializationSpec::new(&NO_SPECIALIZATION_ARGUMENTS, empty_adapters)
        .resolve(&CONST_GENERIC_PARAMETERS)
    {
        Ok(_) => panic!("specialization must bind every parameter"),
        Err(error) => error,
    };
    assert_eq!(
        error,
        "method specialization argument count does not match its generic parameters"
    );
}

#[test]
fn test_specialization_arguments_cover_lifetimes_and_primitive_const_kinds() {
    static PARAMETERS: [GenericParameterMetadata; 15] = [
        GenericParameterMetadata::lifetime("a", 0, &[]),
        GenericParameterMetadata::const_parameter("flag", 1, &TYPE_EXPRESSION, None),
        GenericParameterMetadata::const_parameter("character", 2, &TYPE_EXPRESSION, None),
        GenericParameterMetadata::const_parameter("i8_value", 3, &TYPE_EXPRESSION, None),
        GenericParameterMetadata::const_parameter("i16_value", 4, &TYPE_EXPRESSION, None),
        GenericParameterMetadata::const_parameter("i32_value", 5, &TYPE_EXPRESSION, None),
        GenericParameterMetadata::const_parameter("i64_value", 6, &TYPE_EXPRESSION, None),
        GenericParameterMetadata::const_parameter("i128_value", 7, &TYPE_EXPRESSION, None),
        GenericParameterMetadata::const_parameter("isize_value", 8, &TYPE_EXPRESSION, None),
        GenericParameterMetadata::const_parameter("u8_value", 9, &TYPE_EXPRESSION, None),
        GenericParameterMetadata::const_parameter("u16_value", 10, &TYPE_EXPRESSION, None),
        GenericParameterMetadata::const_parameter("u32_value", 11, &TYPE_EXPRESSION, None),
        GenericParameterMetadata::const_parameter("u64_value", 12, &TYPE_EXPRESSION, None),
        GenericParameterMetadata::const_parameter("u128_value", 13, &TYPE_EXPRESSION, None),
        GenericParameterMetadata::const_parameter("usize_value", 14, &TYPE_EXPRESSION, None),
    ];
    static ARGUMENTS: [MethodSpecializationArgument; 15] = [
        MethodSpecializationArgument::lifetime("'static"),
        MethodSpecializationArgument::const_bool(true),
        MethodSpecializationArgument::const_char('R'),
        MethodSpecializationArgument::i8(-8),
        MethodSpecializationArgument::i16(-16),
        MethodSpecializationArgument::i32(-32),
        MethodSpecializationArgument::i64(-64),
        MethodSpecializationArgument::i128(-128),
        MethodSpecializationArgument::isize(-1),
        MethodSpecializationArgument::u8(8),
        MethodSpecializationArgument::u16(16),
        MethodSpecializationArgument::u32(32),
        MethodSpecializationArgument::u64(64),
        MethodSpecializationArgument::u128(128),
        MethodSpecializationArgument::usize(1),
    ];

    let resolved = MethodSpecializationSpec::new(&ARGUMENTS, NO_ADAPTERS)
        .resolve(&PARAMETERS)
        .expect("all declared arguments have compatible kinds");
    assert_eq!(
        resolved.arguments.as_slice(),
        &[
            crate::runtime::identity::GenericArgumentIdentity::Lifetime("'static".into()),
            crate::runtime::identity::GenericArgumentIdentity::Const("bool:true".into()),
            crate::runtime::identity::GenericArgumentIdentity::Const("char:'R'".into()),
            crate::runtime::identity::GenericArgumentIdentity::Const("i8:-8".into()),
            crate::runtime::identity::GenericArgumentIdentity::Const("i16:-16".into()),
            crate::runtime::identity::GenericArgumentIdentity::Const("i32:-32".into()),
            crate::runtime::identity::GenericArgumentIdentity::Const("i64:-64".into()),
            crate::runtime::identity::GenericArgumentIdentity::Const("i128:-128".into()),
            crate::runtime::identity::GenericArgumentIdentity::Const("isize:-1".into()),
            crate::runtime::identity::GenericArgumentIdentity::Const("u8:8".into()),
            crate::runtime::identity::GenericArgumentIdentity::Const("u16:16".into()),
            crate::runtime::identity::GenericArgumentIdentity::Const("u32:32".into()),
            crate::runtime::identity::GenericArgumentIdentity::Const("u64:64".into()),
            crate::runtime::identity::GenericArgumentIdentity::Const("u128:128".into()),
            crate::runtime::identity::GenericArgumentIdentity::Const("usize:1".into()),
        ]
    );
}

#[test]
fn test_specialization_selection_reports_missing_and_ambiguous_bindings() {
    static ARGUMENT_ONE: [MethodSpecializationArgument; 1] = [MethodSpecializationArgument::u8(1)];
    static ARGUMENT_TWO: [MethodSpecializationArgument; 1] = [MethodSpecializationArgument::u8(2)];
    static SPECS: [MethodSpecializationSpec; 2] = [
        MethodSpecializationSpec::new(&ARGUMENT_ONE, ADAPTERS),
        MethodSpecializationSpec::new(&ARGUMENT_TWO, ADAPTERS).with_unavailable(&[
            crate::runtime::descriptor::method::imports::InvocationUnavailableReason::AdapterUnavailable,
        ]),
    ];
    static GENERIC_METHOD: MethodMetadata = MethodMetadata::from_spec(MethodMetadataSpec::new(
        "generic",
        MethodNamespaceMetadata::Inherent {
            implementation: "Target::generic",
        },
        MethodVisibility::Public,
        Some(CALLABLE_RECEIVER),
        &[],
        &ADAPTER_OUTPUT,
        MethodQualifiers::new(false, false, false, false),
        &CONST_GENERICS,
        true,
        false,
    ));

    let effective = EffectiveMethod::for_type::<Target>(&GENERIC_METHOD).with_specializations(&SPECS);
    let resolved = effective.resolve_specializations().expect("valid specs resolve");
    let method = MethodDescriptor::with_resolved_specializations(
        effective,
        std::sync::Arc::from(resolved),
        None,
        Box::leak(Box::new(ReflectionContext::empty())),
    );
    assert_eq!(
        method.availability(),
        MethodAvailability::RequiresSpecializationSelection { count: 2 }
    );

    let specializations = method.specializations();
    assert_eq!(specializations.len(), 2);
    assert!(!specializations.is_empty());
    let mut iter = specializations.iter();
    let callable = iter.next().expect("first specialization");
    assert_eq!(callable.method().name(), "generic");
    assert!(matches!(
        callable.availability(),
        crate::runtime::descriptor::method::imports::InvocationAvailability::Callable
    ));
    assert_eq!(callable.arguments().iter().len(), 1);

    let arguments = MethodArguments::new().constant(1_u8);
    assert_eq!(arguments.len(), 1);
    assert!(!arguments.is_empty());
    assert_eq!(
        MethodArguments::new()
            .type_of::<ResultType>()
            .constant(true)
            .constant('R')
            .len(),
        3
    );
    let selected = method
        .specialization(&arguments)
        .expect("the matching specialization is unique");
    let target = Target;
    let output = match selected.invoke(Invocation::<Local>::on_ref(&target)) {
        Ok(output) => output,
        Err(_) => panic!("the selected adapter is callable"),
    };
    let value = match output.into_owned::<u32>() {
        Ok(value) => value,
        Err(_) => panic!("the selected adapter returns the declared type"),
    };
    assert_eq!(value, 37);

    let unavailable = iter.next().expect("second specialization");
    assert!(matches!(
        unavailable.availability(),
        crate::runtime::descriptor::method::imports::InvocationAvailability::Unavailable { .. }
    ));
    let failure = match unavailable.invoke(Invocation::<Local>::on_ref(&target)) {
        Err(failure) => failure,
        Ok(_) => panic!("unavailable specialization must reject invocation"),
    };
    assert_eq!(
        *failure.error().kind(),
        InvocationErrorKind::MethodUnavailable {
            reasons: &[crate::runtime::descriptor::method::imports::InvocationUnavailableReason::AdapterUnavailable],
        }
    );
    assert!(iter.next().is_none());

    assert!(matches!(
        method.specialization(&MethodArguments::new().constant(3_u8)),
        Err(LookupError::Missing {
            subject: crate::runtime::error::LookupSubject::MethodSpecialization,
            ..
        })
    ));

    static DUPLICATE_SPECS: [MethodSpecializationSpec; 2] = [
        MethodSpecializationSpec::new(&ARGUMENT_ONE, ADAPTERS),
        MethodSpecializationSpec::new(&ARGUMENT_ONE, ADAPTERS),
    ];
    let duplicate_effective =
        EffectiveMethod::for_type::<Target>(&GENERIC_METHOD).with_specializations(&DUPLICATE_SPECS);
    let duplicate_method = MethodDescriptor::with_resolved_specializations(
        duplicate_effective,
        std::sync::Arc::from(
            duplicate_effective
                .resolve_specializations()
                .expect("duplicate specs still resolve"),
        ),
        None,
        method.context(),
    );
    assert!(matches!(
        duplicate_method.specialization(&MethodArguments::new().constant(1_u8)),
        Err(LookupError::Ambiguous {
            subject: crate::runtime::error::LookupSubject::MethodSpecialization,
            candidates,
            ..
        }) if candidates.len() == 2
    ));

    static SINGLE_CALLABLE_SPECS: [MethodSpecializationSpec; 1] =
        [MethodSpecializationSpec::new(&ARGUMENT_ONE, ADAPTERS)];
    static SINGLE_UNAVAILABLE_SPECS: [MethodSpecializationSpec; 1] =
        [
            MethodSpecializationSpec::new(&ARGUMENT_ONE, ADAPTERS).with_unavailable(&[
                crate::runtime::descriptor::method::imports::InvocationUnavailableReason::AdapterUnavailable,
            ]),
        ];
    for (specs, expected) in [
        (&SINGLE_CALLABLE_SPECS[..], MethodAvailability::Callable),
        (
            &SINGLE_UNAVAILABLE_SPECS[..],
            MethodAvailability::Unavailable {
                reasons: &[
                    crate::runtime::descriptor::method::imports::InvocationUnavailableReason::AdapterUnavailable,
                ],
            },
        ),
    ] {
        let effective = EffectiveMethod::for_type::<Target>(&GENERIC_METHOD).with_specializations(specs);
        let single = MethodDescriptor::with_resolved_specializations(
            effective,
            std::sync::Arc::from(
                effective
                    .resolve_specializations()
                    .expect("single specialization resolves"),
            ),
            None,
            method.context(),
        );
        assert_eq!(single.availability(), expected);
    }
}

#[test]
fn test_registration_equality_compares_source_availability_and_adapter_identity() {
    static METADATA: MethodMetadata = MethodMetadata::inherent("unit", "Target::unit");
    let base = EffectiveMethod::for_type::<Target>(&METADATA);
    assert!(base.same_registration(&EffectiveMethod::for_type::<Target>(&METADATA)));
    assert!(!base.same_registration(&base.with_source(MethodImplementationSource::Overridden)));
    assert!(!base.same_registration(&base.with_adapters(ADAPTERS)));
    assert!(
        EffectiveMethod::for_type::<Target>(&METADATA)
            .with_adapters(ADAPTERS)
            .same_registration(&EffectiveMethod::for_type::<Target>(&METADATA).with_adapters(ADAPTERS))
    );
    assert!(
        !EffectiveMethod::for_type::<Target>(&METADATA)
            .with_adapters(ADAPTERS)
            .same_registration(
                &EffectiveMethod::for_type::<Target>(&METADATA).with_adapters(THREAD_SAFE_ONLY_ADAPTERS)
            )
    );
}

#[test]
fn test_pinned_method_invocation_checks_and_dispatches_the_bound_receiver() {
    static PINNED_RECEIVER: ReceiverDescriptor = ReceiverDescriptor::for_type::<Target>(ReceiverKind::PinnedShared);
    static NO_GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata {
        parameters: &[],
        predicates: &[],
    };
    static PINNED_METHOD: MethodMetadata = MethodMetadata::from_spec(MethodMetadataSpec::new(
        "pinned",
        MethodNamespaceMetadata::Inherent {
            implementation: "Target::pinned",
        },
        MethodVisibility::Public,
        Some(PINNED_RECEIVER),
        &[],
        &ADAPTER_OUTPUT,
        MethodQualifiers::new(false, false, false, false),
        &NO_GENERICS,
        false,
        false,
    ));
    let method = MethodDescriptor::new(
        EffectiveMethod::for_type::<Target>(&PINNED_METHOD).with_adapters(PINNED_ADAPTERS),
        Box::leak(Box::new(ReflectionContext::empty())),
    );
    let target = Box::pin(Target);
    let output = match method.invoke_pinned(PinnedInvocation::<Local, _>::shared(target.as_ref())) {
        Ok(output) => output,
        Err(_) => panic!("matching pinned receiver adapter is callable"),
    };
    let value = match output.into_owned::<u32>() {
        Ok(value) => value,
        Err(_) => panic!("pinned adapter returns the declared type"),
    };
    assert_eq!(value, 41);

    let unavailable = MethodDescriptor::new(
        EffectiveMethod::for_type::<Target>(&PINNED_METHOD).with_adapters(ADAPTERS),
        Box::leak(Box::new(ReflectionContext::empty())),
    );
    let failure = match unavailable.invoke_pinned(PinnedInvocation::<Local, _>::shared(target.as_ref())) {
        Err(failure) => failure,
        Ok(_) => panic!("a method without a pinned adapter must reject invocation"),
    };
    assert_eq!(*failure.error().kind(), InvocationErrorKind::ReceiverAdapterUnavailable);
}

#[test]
fn test_inherent_metadata_constructors_keep_their_documented_defaults() {
    let metadata = MethodMetadata::inherent("unit", "Target::unit");
    assert_eq!(metadata.name, "unit");
    assert!(matches!(
        metadata.namespace,
        MethodNamespaceMetadata::Inherent {
            implementation: "Target::unit"
        }
    ));
    assert_eq!(metadata.visibility, MethodVisibility::Public);
    assert!(metadata.receiver.is_none());
    assert!(metadata.parameters.is_empty());
    assert_eq!(metadata.output.kind, ReturnKind::Unit);
    assert!(!metadata.is_generic);
    assert!(!metadata.default_body);

    let spec = MethodMetadataSpec::inherent("another", "Target::another");
    assert_eq!(spec.name, "another");
    assert!(spec.receiver.is_none());
    assert!(spec.parameters.is_empty());
    assert_eq!(spec.output.kind, ReturnKind::Unit);
    assert!(!spec.qualifiers.is_async);
    assert!(!spec.is_generic);
}

#[test]
fn test_runtime_specialization_builders_resolve_each_public_argument_constructor() {
    static U8_TYPE_EXPRESSION: ExpressionData = ExpressionData::Named {
        path: "u8",
        definition: None,
        arguments: &[],
    };
    let parameters = Box::leak(
        vec![
            GenericParameterMetadata::type_parameter("T", 0, &[], None),
            GenericParameterMetadata::lifetime("a", 1, &[]),
            GenericParameterMetadata::const_parameter("flag", 2, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("character", 3, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("i8_value", 4, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("i16_value", 5, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("i32_value", 6, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("i64_value", 7, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("i128_value", 8, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("isize_value", 9, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("u8_value", 10, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("u16_value", 11, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("u32_value", 12, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("u64_value", 13, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("u128_value", 14, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("usize_value", 15, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("literal", 16, &U8_TYPE_EXPRESSION, None),
        ]
        .into_boxed_slice(),
    );
    let arguments = Box::leak(
        vec![
            MethodSpecializationArgument::type_argument::<Target>(),
            MethodSpecializationArgument::lifetime("'static"),
            MethodSpecializationArgument::const_bool(std::hint::black_box(true)),
            MethodSpecializationArgument::const_char(std::hint::black_box('R')),
            MethodSpecializationArgument::i8(std::hint::black_box(-8)),
            MethodSpecializationArgument::i16(std::hint::black_box(-16)),
            MethodSpecializationArgument::i32(std::hint::black_box(-32)),
            MethodSpecializationArgument::i64(std::hint::black_box(-64)),
            MethodSpecializationArgument::i128(std::hint::black_box(-128)),
            MethodSpecializationArgument::isize(std::hint::black_box(-1)),
            MethodSpecializationArgument::u8(std::hint::black_box(8)),
            MethodSpecializationArgument::u16(std::hint::black_box(16)),
            MethodSpecializationArgument::u32(std::hint::black_box(32)),
            MethodSpecializationArgument::u64(std::hint::black_box(64)),
            MethodSpecializationArgument::u128(std::hint::black_box(128)),
            MethodSpecializationArgument::usize(std::hint::black_box(1)),
            MethodSpecializationArgument::const_integer_literal(std::hint::black_box(false), std::hint::black_box(255)),
        ]
        .into_boxed_slice(),
    );
    let reasons = Box::leak(
        vec![crate::runtime::descriptor::method::imports::InvocationUnavailableReason::AdapterUnavailable]
            .into_boxed_slice(),
    );
    let spec = MethodSpecializationSpec::new(arguments, ADAPTERS).with_unavailable(std::hint::black_box(reasons));
    let resolved = spec
        .resolve(std::hint::black_box(parameters))
        .expect("all argument constructors produce matching metadata");
    assert!(matches!(
        resolved.availability,
        crate::runtime::descriptor::method::imports::InvocationAvailability::Unavailable { .. }
    ));
    assert_eq!(resolved.arguments.as_slice().len(), parameters.len());
}

#[test]
fn test_integer_expression_paths_and_normalization_cover_signed_and_unsigned_ranges() {
    static EXPRESSIONS: [ExpressionData; 12] = [
        ExpressionData::Named {
            path: "i8",
            definition: None,
            arguments: &[],
        },
        ExpressionData::Named {
            path: "i16",
            definition: None,
            arguments: &[],
        },
        ExpressionData::Named {
            path: "i32",
            definition: None,
            arguments: &[],
        },
        ExpressionData::Named {
            path: "i64",
            definition: None,
            arguments: &[],
        },
        ExpressionData::Named {
            path: "i128",
            definition: None,
            arguments: &[],
        },
        ExpressionData::Named {
            path: "isize",
            definition: None,
            arguments: &[],
        },
        ExpressionData::Named {
            path: "u8",
            definition: None,
            arguments: &[],
        },
        ExpressionData::Named {
            path: "u16",
            definition: None,
            arguments: &[],
        },
        ExpressionData::Named {
            path: "u32",
            definition: None,
            arguments: &[],
        },
        ExpressionData::Named {
            path: "u64",
            definition: None,
            arguments: &[],
        },
        ExpressionData::Named {
            path: "u128",
            definition: None,
            arguments: &[],
        },
        ExpressionData::Named {
            path: "usize",
            definition: None,
            arguments: &[],
        },
    ];
    static EXPECTED: [&str; 12] = [
        "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
    ];
    for (expression, expected) in EXPRESSIONS.iter().zip(EXPECTED) {
        assert_eq!(
            crate::runtime::descriptor::method::integer_kind_for_expression(expression)
                .map(crate::runtime::descriptor::method::ConstIntegerKind::type_name),
            Some(expected)
        );
    }

    static NAMED_EXPRESSION_WITH_ARGUMENT: ExpressionData = ExpressionData::Named {
        path: "u8",
        definition: None,
        arguments: &[crate::runtime::expression::GenericExpressionArgument::Type(
            &TYPE_EXPRESSION,
        )],
    };
    static OPAQUE_EXPRESSION: ExpressionData = ExpressionData::Tuple(&[]);
    assert!(crate::runtime::descriptor::method::integer_kind_for_expression(&NAMED_EXPRESSION_WITH_ARGUMENT).is_none());
    assert!(crate::runtime::descriptor::method::integer_kind_for_expression(&OPAQUE_EXPRESSION).is_none());

    let signed = [
        (crate::runtime::descriptor::method::ConstIntegerKind::I8, 8),
        (crate::runtime::descriptor::method::ConstIntegerKind::I16, 16),
        (crate::runtime::descriptor::method::ConstIntegerKind::I32, 32),
        (crate::runtime::descriptor::method::ConstIntegerKind::I64, 64),
        (crate::runtime::descriptor::method::ConstIntegerKind::I128, 128),
        (crate::runtime::descriptor::method::ConstIntegerKind::Isize, usize::BITS),
    ];
    for (kind, bits) in signed {
        let maximum = (1_u128 << (bits - 1)) - 1;
        let minimum_magnitude = 1_u128 << (bits - 1);
        assert!(kind.normalize(false, maximum).is_some());
        assert!(kind.normalize(true, minimum_magnitude).is_some());
        assert!(kind.normalize(false, minimum_magnitude).is_none());
        assert!(kind.normalize(true, minimum_magnitude + 1).is_none());
    }

    let unsigned = [
        (crate::runtime::descriptor::method::ConstIntegerKind::U8, 8),
        (crate::runtime::descriptor::method::ConstIntegerKind::U16, 16),
        (crate::runtime::descriptor::method::ConstIntegerKind::U32, 32),
        (crate::runtime::descriptor::method::ConstIntegerKind::U64, 64),
        (crate::runtime::descriptor::method::ConstIntegerKind::U128, 128),
        (crate::runtime::descriptor::method::ConstIntegerKind::Usize, usize::BITS),
    ];
    for (kind, bits) in unsigned {
        assert_eq!(kind.normalize(false, 0).as_deref(), Some("0"));
        assert!(kind.normalize(true, 1).is_none());
        if bits < 128 {
            assert!(kind.normalize(false, 1_u128 << bits).is_none());
        }
    }
}

#[test]
fn test_receiver_and_optional_return_metadata_preserve_absence() {
    static NO_RECEIVER: MethodMetadata = MethodMetadata::from_spec(MethodMetadataSpec::new(
        "unit",
        MethodNamespaceMetadata::Inherent {
            implementation: "Target::unit",
        },
        MethodVisibility::Private,
        None,
        &[],
        &ReturnMetadata::unit(),
        MethodQualifiers::new(false, false, true, true),
        &GenericDefinitionMetadata {
            parameters: &[],
            predicates: &[],
        },
        false,
        true,
    ));
    let method = descriptor(&NO_RECEIVER);
    assert!(method.receiver().is_none());
    assert_eq!(method.parameters().len(), 0);
    assert!(method.parameters().is_empty());
    assert_eq!(method.output().kind(), ReturnKind::Unit);
    assert!(method.output().type_expression().is_none());
    assert!(method.output().find_concrete_type().is_none());
    assert!(method.qualifiers().is_unsafe);
    assert!(method.qualifiers().is_variadic);
    assert!(method.generics().is_empty());
    assert_eq!(
        method.visibility(),
        crate::runtime::descriptor::trait_descriptor::Visibility::Private
    );
    assert!(method.availability().eq(&MethodAvailability::Unavailable {
        reasons: &[crate::runtime::descriptor::method::imports::InvocationUnavailableReason::AdapterUnavailable],
    }));
}
