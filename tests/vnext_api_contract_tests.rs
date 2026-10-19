use qubit_reflect::__private::codegen_v4 as codegen;

struct V4Fixture {
    name: u32,
}

impl codegen::descriptor::Reflect for V4Fixture {
    fn __reflect_registration() -> &'static codegen::descriptor::TypeRegistration<Self> {
        static REGISTRATION: codegen::descriptor::TypeRegistration<V4Fixture> =
            codegen::descriptor::TypeRegistration::new();
        &REGISTRATION
    }
}

fn name_ref(value: &V4Fixture) -> &u32 {
    &value.name
}

fn name_mut(value: &mut V4Fixture) -> &mut u32 {
    &mut value.name
}

static NAME_ADAPTER: codegen::access::TypedFieldAdapter<V4Fixture, u32> =
    codegen::access::TypedFieldAdapter::new(name_ref, name_mut);
static NAME_FIELDS: [codegen::access::FieldMetadata; 1] =
    [codegen::access::FieldMetadata::typed(0, "name", &NAME_ADAPTER)];
static SHAPE: codegen::access::TypeKindMetadata =
    codegen::access::TypeKindMetadata::Struct(&NAME_FIELDS);
static METHOD_METADATA: codegen::invoke::MethodMetadata =
    codegen::invoke::MethodMetadata::inherent("run", "V4Fixture");
static METHODS: [codegen::invoke::EffectiveMethod; 1] =
    [codegen::invoke::EffectiveMethod::for_type::<V4Fixture>(&METHOD_METADATA)];

codegen::registration::submit_type!(V4Fixture);
codegen::access::submit_type_shape!(V4Fixture, &SHAPE);
codegen::invoke::submit_method_catalog!(V4Fixture, &METHODS);

#[test]
fn v4_contract_exposes_typed_descriptor_context_and_lookup() {
    assert_eq!(codegen::PROTOCOL_VERSION, 4);

    let context = codegen::descriptor::ReflectionContext::global();
    let descriptor = codegen::descriptor::TypeDescriptor::of::<V4Fixture>();
    assert!(std::ptr::eq(context, descriptor.context()));

    assert_eq!(descriptor.field("name").unwrap().name(), "name");
    assert_eq!(descriptor.method("run").unwrap().name(), "run");
    assert_eq!(
        descriptor.lookup_method("run").unwrap().only().unwrap().name(),
        "run"
    );
}
