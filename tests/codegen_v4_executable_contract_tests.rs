use qubit_reflect::__private::codegen_v4::invoke::{
    EffectiveMethod, FunctionInvocationAdapter, InvocationAdapter, InvocationAdapters,
    InvocationOutput, MethodCatalogRegistration, MethodMetadata, MethodMetadataSpec,
    ValidatedInvocation,
};
use qubit_reflect::__private::codegen_v4::value::Local;

struct Fixture;

impl qubit_reflect::__private::codegen_v4::descriptor::Reflect for Fixture {
    fn __reflect_registration()
    -> &'static qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<Self> {
        static REGISTRATION: qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<
            Fixture,
        > = qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration::new();
        &REGISTRATION
    }
}

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
static CATALOG: MethodCatalogRegistration = MethodCatalogRegistration::new::<Fixture>(&METHODS);

#[test]
fn downstream_can_implement_typed_adapter_and_register_method_catalog() {
    assert_eq!(qubit_reflect::__private::codegen_v4::PROTOCOL_VERSION, 4);
    let _ = &CATALOG;
}
