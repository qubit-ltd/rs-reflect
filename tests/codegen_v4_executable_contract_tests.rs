use qubit_reflect::__private::codegen_v4::expression::ExpressionData;
use qubit_reflect::__private::codegen_v4::invoke::{
    EffectiveMethod, FunctionInvocationAdapter, InvocationAdapters, InvocationOutput,
    InvocationUnavailableReason, MethodCatalogRegistration, MethodMetadata, MethodMetadataSpec,
    MethodSpecializationArgument, MethodSpecializationSpec, ValidatedInvocation,
};
use qubit_reflect::__private::codegen_v4::invoke::{ReceiverDescriptor, ReceiverKind};
use qubit_reflect::__private::codegen_v4::value::Local;

struct Fixture;

#[derive(Clone)]
struct CloneableFixture;

impl qubit_reflect::__private::codegen_v4::descriptor::Reflect for Fixture {
    fn __reflect_registration(
    ) -> &'static qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<Self> {
        static REGISTRATION: qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<
            Fixture,
        > = qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration::new();
        &REGISTRATION
    }
}

impl qubit_reflect::__private::codegen_v4::descriptor::Reflect for CloneableFixture {
    fn __reflect_registration(
    ) -> &'static qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<Self> {
        static REGISTRATION: qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<
            CloneableFixture,
        > = qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration::new();
        &REGISTRATION
    }
}

static SELF_EXPRESSION: ExpressionData = ExpressionData::SelfType;
static TRAIT_SELF_RECEIVER: ReceiverDescriptor =
    ReceiverDescriptor::trait_self(ReceiverKind::Shared, &SELF_EXPRESSION);

qubit_reflect::__private::codegen_v4::capability::register_capability!(
    CloneableFixture: CloneCapability
);

fn return_argument<'call>(
    invocation: ValidatedInvocation<'call, Local>,
) -> InvocationOutput<'call, Local> {
    let (_, arguments) = invocation.into_parts();
    let mut arguments = arguments.into_iter();
    match arguments
        .next()
        .and_then(|argument| argument.into_owned::<u32>().ok())
    {
        Some(value) => InvocationOutput::owned(value),
        None => InvocationOutput::unit(),
    }
}

static ADAPTER: FunctionInvocationAdapter<Local> = FunctionInvocationAdapter::new(return_argument);
static METADATA: MethodMetadata =
    MethodMetadata::from_spec(MethodMetadataSpec::inherent("echo", "Fixture"));
static ADAPTERS: InvocationAdapters = InvocationAdapters::builder().local(&ADAPTER).build();
static METHODS: [EffectiveMethod; 1] =
    [EffectiveMethod::for_type::<Fixture>(&METADATA).with_adapters(ADAPTERS)];
static UNAVAILABLE_METHOD: EffectiveMethod = EffectiveMethod::for_type::<Fixture>(&METADATA)
    .with_unavailable(&[
        InvocationUnavailableReason::UnsupportedReceiver,
        InvocationUnavailableReason::UnsafeMethod,
    ]);
static SPECIALIZATION_ARGUMENTS: [MethodSpecializationArgument; 3] = [
    MethodSpecializationArgument::type_argument::<u64>(),
    MethodSpecializationArgument::const_integer_literal(false, 4),
    MethodSpecializationArgument::lifetime("'static"),
];
static SPECIALIZATIONS: [MethodSpecializationSpec; 1] = [MethodSpecializationSpec::new(
    &SPECIALIZATION_ARGUMENTS,
    ADAPTERS,
)];
static SPECIALIZED_METHOD: EffectiveMethod =
    EffectiveMethod::for_type::<Fixture>(&METADATA).with_specializations(&SPECIALIZATIONS);
static METHOD_BINDING: qubit_reflect::__private::codegen_v4::registration::MethodBinding<Fixture> =
    qubit_reflect::__private::codegen_v4::registration::bind_method::<Fixture>(&METADATA, ADAPTERS)
        .with_specializations(&SPECIALIZATIONS);
static CATALOG: MethodCatalogRegistration = MethodCatalogRegistration::new::<Fixture>(&METHODS);

#[test]
fn downstream_can_implement_typed_adapter_and_register_method_catalog() {
    assert_eq!(qubit_reflect::__private::codegen_v4::PROTOCOL_VERSION, 4);
    let _ = (
        &TRAIT_SELF_RECEIVER,
        &UNAVAILABLE_METHOD,
        &SPECIALIZED_METHOD,
        &METHOD_BINDING,
        CloneableFixture,
    );
    let _ = &CATALOG;
}
