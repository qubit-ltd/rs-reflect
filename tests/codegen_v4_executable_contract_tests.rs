// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::sync::OnceLock;

use qubit_reflect::__private::codegen_v4::access::TypeKindMetadata;
use qubit_reflect::__private::codegen_v4::access::Visibility;
use qubit_reflect::__private::codegen_v4::capability::CloneCapability;
use qubit_reflect::__private::codegen_v4::capability::DefaultCapability;
use qubit_reflect::__private::codegen_v4::construct::ConstructField;
use qubit_reflect::__private::codegen_v4::construct::ConstructProvider;
use qubit_reflect::__private::codegen_v4::construct::ConstructionAvailability;
use qubit_reflect::__private::codegen_v4::construct::ConstructionFieldPolicy;
use qubit_reflect::__private::codegen_v4::construct::ConstructionPlan;
use qubit_reflect::__private::codegen_v4::construct::ConstructionProviders;
use qubit_reflect::__private::codegen_v4::construct::ConstructionShape;
use qubit_reflect::__private::codegen_v4::construct::UpdateFieldPolicy;
use qubit_reflect::__private::codegen_v4::expression::ExpressionData;
use qubit_reflect::__private::codegen_v4::expression::GenericDefinitionMetadata;
use qubit_reflect::__private::codegen_v4::impls::ImplApplicationSpec;
use qubit_reflect::__private::codegen_v4::impls::ImplDefinitionSpec;
use qubit_reflect::__private::codegen_v4::impls::Kind;
use qubit_reflect::__private::codegen_v4::impls::Qualifiers;
use qubit_reflect::__private::codegen_v4::invoke::EffectiveMethod;
use qubit_reflect::__private::codegen_v4::invoke::FunctionInvocationAdapter;
use qubit_reflect::__private::codegen_v4::invoke::InvocationAdapters;
use qubit_reflect::__private::codegen_v4::invoke::InvocationOutput;
use qubit_reflect::__private::codegen_v4::invoke::InvocationUnavailableReason;
use qubit_reflect::__private::codegen_v4::invoke::MethodCatalogRegistration;
use qubit_reflect::__private::codegen_v4::invoke::MethodMetadata;
use qubit_reflect::__private::codegen_v4::invoke::MethodMetadataSpec;
use qubit_reflect::__private::codegen_v4::invoke::MethodSpecializationArgument;
use qubit_reflect::__private::codegen_v4::invoke::MethodSpecializationSpec;
use qubit_reflect::__private::codegen_v4::invoke::ReceiverDescriptor;
use qubit_reflect::__private::codegen_v4::invoke::ReceiverKind;
use qubit_reflect::__private::codegen_v4::invoke::ValidatedInvocation;
use qubit_reflect::__private::codegen_v4::value::DynamicMut;
use qubit_reflect::__private::codegen_v4::value::DynamicOwned;
use qubit_reflect::__private::codegen_v4::value::DynamicRef;
use qubit_reflect::__private::codegen_v4::value::Local;
use qubit_reflect::__private::codegen_v4::value::ThreadSafe;
use qubit_reflect::ReflectionContext;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::capability::CapabilityInvocationError;
use qubit_reflect::construct::NamedFields;
use qubit_reflect::value::DynamicRef as ApiDynamicRef;
use qubit_reflect::value::Local as ApiLocal;
use qubit_reflect::value::ThreadSafe as ApiThreadSafe;

struct Fixture;

#[derive(Clone, Default)]
struct CloneableFixture;

struct ConstructibleFixture {
    value: u32,
}

impl qubit_reflect::__private::codegen_v4::descriptor::Reflect for Fixture {
    fn __reflect_registration() -> &'static qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<Self> {
        static REGISTRATION: qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<Fixture> =
            qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration::new();
        &REGISTRATION
    }
}

impl qubit_reflect::__private::codegen_v4::descriptor::Reflect for CloneableFixture {
    fn __reflect_registration() -> &'static qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<Self> {
        static REGISTRATION: qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<CloneableFixture> =
            qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration::new();
        &REGISTRATION
    }
}

impl qubit_reflect::__private::codegen_v4::descriptor::Reflect for ConstructibleFixture {
    fn __reflect_registration() -> &'static qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<Self> {
        static REGISTRATION: OnceLock<
            qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<ConstructibleFixture>,
        > = OnceLock::new();
        REGISTRATION.get_or_init(|| {
            qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration::with_shape(
                &CONSTRUCTIBLE_SHAPE,
                None,
                &CONSTRUCTIBLE_PLANS,
            )
        })
    }
}

fn construct_constructible(fields: Vec<Option<DynamicOwned<Local>>>) -> ConstructibleFixture {
    let [Some(value)] = fields.as_slice() else {
        panic!("validated construction input must have one required field");
    };
    ConstructibleFixture {
        value: *value.downcast_ref::<u32>().expect("field type must be validated"),
    }
}

fn update_constructible(
    mut base: ConstructibleFixture,
    replacements: Vec<Option<DynamicOwned<Local>>>,
) -> ConstructibleFixture {
    if let [Some(value)] = replacements.as_slice() {
        base.value = *value.downcast_ref::<u32>().expect("field type must be validated");
    }
    base
}

static CONSTRUCTIBLE_SHAPE: TypeKindMetadata = TypeKindMetadata::Struct(&[]);
static CONSTRUCTIBLE_PROVIDER: ConstructProvider<ConstructibleFixture, Local> =
    ConstructProvider::new(construct_constructible, update_constructible);
static CONSTRUCTIBLE_PROVIDERS: ConstructionProviders =
    ConstructionProviders::new::<ConstructibleFixture>(Some(&CONSTRUCTIBLE_PROVIDER));
static CONSTRUCTIBLE_FIELDS: [ConstructField; 1] = [ConstructField::typed::<u32>(
    "value",
    Visibility::Public,
    ConstructionFieldPolicy::Required,
    UpdateFieldPolicy::Replaceable,
)];
static CONSTRUCTIBLE_PLANS: [ConstructionPlan; 1] = [ConstructionPlan::for_type::<ConstructibleFixture>(
    ConstructionShape::Named,
    &CONSTRUCTIBLE_FIELDS,
    CONSTRUCTIBLE_PROVIDERS,
    ConstructionAvailability::Available,
)];

static SELF_EXPRESSION: ExpressionData = ExpressionData::SelfType;
static TRAIT_SELF_RECEIVER: ReceiverDescriptor = ReceiverDescriptor::trait_self(ReceiverKind::Shared, &SELF_EXPRESSION);

qubit_reflect::__private::codegen_v4::capability::register_capability!(
    CloneableFixture: CloneCapability
);

fn return_argument<'call>(invocation: ValidatedInvocation<'call, Local>) -> InvocationOutput<'call, Local> {
    let (_, arguments) = invocation.into_parts();
    let mut arguments = arguments.into_iter();
    match arguments.next().and_then(|argument| argument.into_owned::<u32>().ok()) {
        Some(value) => InvocationOutput::owned(value),
        None => InvocationOutput::unit(),
    }
}

static ADAPTER: FunctionInvocationAdapter<Local> = FunctionInvocationAdapter::new(return_argument);
static METADATA: MethodMetadata = MethodMetadata::from_spec(MethodMetadataSpec::inherent("echo", "Fixture"));
static ADAPTERS: InvocationAdapters = InvocationAdapters::builder().local(&ADAPTER).build();
static METHODS: [EffectiveMethod; 1] = [EffectiveMethod::for_type::<Fixture>(&METADATA).with_adapters(ADAPTERS)];
static UNAVAILABLE_METHOD: EffectiveMethod = EffectiveMethod::for_type::<Fixture>(&METADATA).with_unavailable(&[
    InvocationUnavailableReason::UnsupportedReceiver,
    InvocationUnavailableReason::UnsafeMethod,
]);
static SPECIALIZATION_ARGUMENTS: [MethodSpecializationArgument; 3] = [
    MethodSpecializationArgument::type_argument::<u64>(),
    MethodSpecializationArgument::const_integer_literal(false, 4),
    MethodSpecializationArgument::lifetime("'static"),
];
static SPECIALIZATIONS: [MethodSpecializationSpec; 1] =
    [MethodSpecializationSpec::new(&SPECIALIZATION_ARGUMENTS, ADAPTERS)];
static SPECIALIZED_METHOD: EffectiveMethod =
    EffectiveMethod::for_type::<Fixture>(&METADATA).with_specializations(&SPECIALIZATIONS);
static METHOD_BINDING: qubit_reflect::__private::codegen_v4::registration::MethodBinding<Fixture> =
    qubit_reflect::__private::codegen_v4::registration::bind_method::<Fixture>(&METADATA, ADAPTERS)
        .with_specializations(&SPECIALIZATIONS);
static CATALOG: MethodCatalogRegistration = MethodCatalogRegistration::new::<Fixture>(&METHODS);

static PROVIDED_METHODS: OnceLock<Box<[EffectiveMethod]>> = OnceLock::new();

fn provide_fixture_methods() -> &'static [EffectiveMethod] {
    PROVIDED_METHODS.get_or_init(|| METHODS.to_vec().into_boxed_slice())
}

qubit_reflect::__private::codegen_v4::registration::submit_type!(Fixture);
qubit_reflect::__private::codegen_v4::registration::submit_type!(CloneableFixture);
qubit_reflect::__private::codegen_v4::registration::submit_type!(ConstructibleFixture);
qubit_reflect::__private::codegen_v4::invoke::submit_method_catalog_provider!(Fixture, provide_fixture_methods);

#[test]
fn downstream_can_implement_typed_adapter_and_register_method_catalog() {
    assert_eq!(qubit_reflect::__private::codegen_v4::PROTOCOL_VERSION, 4);
    let descriptor = qubit_reflect::__private::codegen_v4::descriptor::TypeDescriptor::of::<Fixture>();
    assert_eq!(descriptor.method("echo").unwrap().name(), "echo");
    let _ = (
        &TRAIT_SELF_RECEIVER,
        &UNAVAILABLE_METHOD,
        &SPECIALIZED_METHOD,
        &METHOD_BINDING,
        CloneableFixture,
    );
    let _ = &CATALOG;
}

#[test]
fn typed_value_adapters_preserve_values_across_mismatch_recovery() {
    let owned = DynamicOwned::<Local>::new(String::from("owned"));
    let failure = owned
        .downcast::<u32>()
        .expect_err("a mismatched downcast must retain the owned value");
    let mut owned = failure.into_value();
    owned.downcast_mut::<String>().unwrap().push_str(" recovered");
    assert_eq!(owned.downcast::<String>().unwrap(), "owned recovered");

    let text = String::from("shared");
    let shared = DynamicRef::<Local>::new(&text);
    let failure = shared
        .downcast::<u32>()
        .expect_err("a mismatched shared downcast must retain its borrow");
    assert_eq!(failure.into_value().downcast::<String>().unwrap(), &text);

    let mut text = String::from("mutable");
    let dynamic = DynamicMut::<Local>::new(&mut text);
    let failure = dynamic
        .downcast::<u32>()
        .expect_err("a mismatched mutable downcast must retain its borrow");
    failure
        .into_value()
        .downcast::<String>()
        .unwrap()
        .push_str(" recovered");
    assert_eq!(text, "mutable recovered");

    let owned = DynamicOwned::<ThreadSafe>::new(String::from("thread safe"));
    assert_eq!(owned.downcast::<String>().unwrap(), "thread safe");
}

#[test]
fn linked_thread_safe_capability_adapters_execute_and_report_type_mismatch() {
    let registration = qubit_reflect::__private::codegen_v4::capability::register_thread_safe_clone::<CloneableFixture>(
        "test",
        "0",
        "tests",
        file!(),
        line!(),
    );
    let default_registration = qubit_reflect::__private::codegen_v4::capability::register_thread_safe_default::<
        CloneableFixture,
    >("test", "0", "tests", file!(), line!());
    let context = ReflectionContext::builder()
        .include_type::<CloneableFixture>()
        .register(registration)
        .register(default_registration)
        .build()
        .expect("the linked clone capability registration is valid");
    let adapter = context
        .type_of::<CloneableFixture>()
        .capability::<CloneCapability>()
        .unwrap();
    let value = CloneableFixture;
    let cloned = adapter
        .adapter()
        .clone_dynamic(ApiDynamicRef::<ApiLocal>::new(&value))
        .expect("the local adapter clones its exact target type");
    assert!(cloned.is::<CloneableFixture>());

    let wrong = 17_u32;
    let error = match adapter
        .adapter()
        .clone_dynamic(ApiDynamicRef::<ApiThreadSafe>::new(&wrong))
    {
        Ok(_) => panic!("a typed clone adapter must reject a different Rust type"),
        Err(error) => error,
    };
    assert!(matches!(error, CapabilityInvocationError::TypeMismatch { .. }));

    let default_adapter = context
        .type_of::<CloneableFixture>()
        .capability::<DefaultCapability>()
        .unwrap();
    assert!(
        default_adapter
            .adapter()
            .create_dynamic::<ApiLocal>()
            .unwrap()
            .is::<CloneableFixture>()
    );
    assert!(
        default_adapter
            .adapter()
            .create_dynamic::<ApiThreadSafe>()
            .unwrap()
            .is::<CloneableFixture>()
    );
}

#[test]
fn typed_construction_provider_executes_through_its_target_descriptor() {
    let constructed: ConstructibleFixture = TypeDescriptor::of::<ConstructibleFixture>()
        .construct(NamedFields::<ApiLocal>::new().field("value", 29_u32))
        .expect("the typed provider constructs its reflected target");
    assert_eq!(constructed.value, 29);
}

#[test]
fn typed_impl_registration_factories_return_registrations_for_static_metadata() {
    static GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata::new(&[], &[]);
    struct ImplMarker;
    static DEFINITION: ImplDefinitionSpec<ImplMarker> = ImplDefinitionSpec::new(
        Kind::Inherent,
        &SELF_EXPRESSION,
        None,
        &GENERICS,
        Qualifiers::new(false),
        &[],
        &[],
        &[],
    );
    static APPLICATION: ImplApplicationSpec<Fixture> =
        ImplApplicationSpec::for_target::<ImplMarker>(&DEFINITION, None, &[], &[], &[], &[], &[]);

    let definition = qubit_reflect::__private::codegen_v4::impls::register_impl_definition(&DEFINITION);
    let application = qubit_reflect::__private::codegen_v4::impls::register_impl_application(&APPLICATION);
    let context = ReflectionContext::builder()
        .include_type::<Fixture>()
        .register(definition)
        .register(application)
        .build()
        .expect("typed implementation metadata can be registered together");
    assert_eq!(context.type_of::<Fixture>().impls().unwrap().len(), 1);
}
