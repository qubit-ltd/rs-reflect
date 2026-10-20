// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use std::collections::BTreeSet;
use std::collections::HashSet;

use qubit_reflect::__private::codegen_v4::expression::GenericDefinitionMetadata;
use qubit_reflect::__private::codegen_v4::expression::GenericExpressionArgument;
use qubit_reflect::__private::codegen_v4::expression::LifetimeExpression;
use qubit_reflect::__private::codegen_v4::expression::StructKind;
use qubit_reflect::__private::codegen_v4::expression::TypeDefinitionKind;
use qubit_reflect::__private::codegen_v4::expression::TypeExpressionData;
use qubit_reflect::__private::codegen_v4::expression::type_expression_in;
use qubit_reflect::__private::codegen_v4::registration::Registration;
use qubit_reflect::__private::codegen_v4::registration::TypeDefinitionRegistration;
use qubit_reflect::__private::codegen_v4::registration::type_definition_with_capability;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectionContext;
use qubit_reflect::capability::CapabilityOrigin;
use qubit_reflect::capability::CapabilityRegistration;
use qubit_reflect::capability::CapabilitySpec;
use qubit_reflect::capability::CapabilityTarget;
use qubit_reflect::descriptor::GenericExpressionArgumentKind;
use qubit_reflect::descriptor::TypeDescriptor;
use qubit_reflect::descriptor::TypeExpressionKind;

struct GenericCapabilityMarker;

static EMPTY_GENERIC_DEFINITION: GenericDefinitionMetadata = GenericDefinitionMetadata::new(&[], &[]);
static GENERIC_DEFINITION_REGISTRATION: TypeDefinitionRegistration<GenericCapabilityMarker> =
    TypeDefinitionRegistration::new(
        "GenericCapabilityMarker",
        "tests::GenericCapabilityMarker",
        TypeDefinitionKind::Struct(StructKind::Named),
        &EMPTY_GENERIC_DEFINITION,
    );

fn generic_capability_registration() -> Registration {
    type_definition_with_capability::<GenericCapabilityMarker, ModelCapability>(
        &GENERIC_DEFINITION_REGISTRATION,
        "marker adapter",
    )
}

qubit_reflect::__private::codegen_v4::registration::submit_type_definition!(
    "generic-capability-marker",
    generic_capability_registration
);

#[Reflect]
struct ConsumerModel {
    count: u32,
    items: Vec<String>,
    optional_items: Option<Vec<String>>,
}

#[Reflect]
struct GenericConsumerModel<T>(T);

#[Reflect]
struct BuiltinStructureConsumer {
    pair: (u8, String),
    ordered: BTreeSet<u8>,
    hashed: HashSet<String>,
    boxed_slice: Box<[ConsumerChild]>,
}

#[Reflect]
struct ConsumerChild {
    value: u32,
}

fn generic_consumer_capability_registration() -> Registration {
    Registration::from(__qubit_reflect_v4_definition_capability_GenericConsumerModel::<
        ModelCapability,
    >("generated model adapter"))
}

qubit_reflect::__private::codegen_v4::registration::submit_linked_definition_capability!(
    "generic-consumer-capability",
    generic_consumer_capability_registration
);

trait ConsumerAssoc<U> {
    type Assoc<'a>;
}

#[Reflect]
struct AssocProvider;

impl ConsumerAssoc<u8> for AssocProvider {
    type Assoc<'a> = &'a str;
}

#[Reflect]
struct AssociatedProjection<T: ConsumerAssoc<u8>> {
    direct: T::Assoc<'static>,
    qualified: <T as ConsumerAssoc<u8>>::Assoc<'static>,
}

struct ModelCapability;

impl CapabilitySpec for ModelCapability {
    type Adapter = &'static str;

    const ID: &'static str = "external_consumer.model";
}

struct WrongModelCapability;

impl CapabilitySpec for WrongModelCapability {
    type Adapter = u32;

    const ID: &'static str = ModelCapability::ID;
}

static TYPE_PARAMETER: TypeExpressionData = TypeExpressionData::ParameterName("T");
static INNER_ARGUMENTS: [GenericExpressionArgument; 1] = [GenericExpressionArgument::Type(&TYPE_PARAMETER)];
static INNER_TYPE: TypeExpressionData = TypeExpressionData::Named {
    path: "Vec",
    definition: None,
    arguments: &INNER_ARGUMENTS,
};
static OUTER_ARGUMENTS: [GenericExpressionArgument; 1] = [GenericExpressionArgument::Type(&INNER_TYPE)];
static NESTED_GENERIC_TYPE: TypeExpressionData = TypeExpressionData::Named {
    path: "Option",
    definition: None,
    arguments: &OUTER_ARGUMENTS,
};
static ASSOCIATED_TYPE_SELF: TypeExpressionData = TypeExpressionData::ParameterName("T");
static ASSOCIATED_TYPE_ARGUMENTS: [GenericExpressionArgument; 1] =
    [GenericExpressionArgument::Lifetime(LifetimeExpression::Named("a"))];
static ASSOCIATED_TYPE_PROJECTION: TypeExpressionData = TypeExpressionData::AssociatedType {
    self_type: &ASSOCIATED_TYPE_SELF,
    trait_: None,
    name: "Assoc",
    arguments: &ASSOCIATED_TYPE_ARGUMENTS,
};

#[test]
fn external_consumer_can_use_context_bound_model_and_capability_contract() {
    let first = ReflectionContext::builder()
        .include_type::<ConsumerModel>()
        .include_type::<u32>()
        .register(CapabilityRegistration::for_type::<ConsumerModel, ModelCapability>(
            "first",
        ))
        .build()
        .expect("the first context includes the model and capability");
    let second = ReflectionContext::builder()
        .include_type::<ConsumerModel>()
        .include_type::<u32>()
        .build()
        .expect("the second context includes the model without the capability");

    let first_type = TypeDescriptor::of_in::<ConsumerModel>(&first);
    let field = first_type.field("count").expect("the field is reflected");
    let type_expression = field.type_expression();
    assert_eq!(type_expression.display().to_string(), "u32");
    assert!(std::ptr::eq(type_expression.context(), &first));
    let concrete_type = field.find_concrete_type().unwrap();
    assert_eq!(concrete_type.type_id(), std::any::TypeId::of::<u32>());
    assert!(std::ptr::eq(concrete_type.context(), &first));
    assert!(std::ptr::eq(field.context(), &first));
    let items = first_type.field("items").expect("the generic field is reflected");
    let items_type = items
        .find_concrete_type()
        .expect("a reflected field has its concrete type in the same context");
    assert_eq!(items_type.type_id(), std::any::TypeId::of::<Vec<String>>());
    assert!(std::ptr::eq(items_type.context(), &first));
    assert_eq!(
        items_type
            .generic_arguments()
            .get_at(0)
            .unwrap()
            .type_argument()
            .unwrap()
            .find_concrete_type()
            .unwrap()
            .type_id(),
        std::any::TypeId::of::<String>()
    );
    let optional_items = first_type
        .field("optional_items")
        .expect("the optional generic field is reflected");
    let option_type = optional_items
        .find_concrete_type()
        .expect("optional field has a concrete descriptor");
    let vector_type = option_type
        .generic_arguments()
        .get_at(0)
        .unwrap()
        .type_argument()
        .unwrap()
        .find_concrete_type()
        .expect("optional generic argument has a concrete descriptor");
    assert_eq!(vector_type.type_id(), std::any::TypeId::of::<Vec<String>>());
    assert_eq!(
        vector_type
            .generic_arguments()
            .get_at(0)
            .unwrap()
            .type_argument()
            .unwrap()
            .find_concrete_type()
            .unwrap()
            .type_id(),
        std::any::TypeId::of::<String>()
    );
    assert!(std::ptr::eq(option_type.context(), &first));
    assert!(std::ptr::eq(vector_type.context(), &first));

    let capability = first_type
        .capability::<ModelCapability>()
        .expect("capability is registered");
    assert_eq!(capability.adapter(), &"first");
    assert!(matches!(capability.origin(), CapabilityOrigin::Explicit { .. }));
    assert!(matches!(capability.target(), CapabilityTarget::Type(target)
        if std::ptr::eq(target.context(), &first)));

    let second_type = TypeDescriptor::of_in::<ConsumerModel>(&second);
    assert!(std::ptr::eq(second_type.context(), &second));
    assert!(second_type.find_capability::<ModelCapability>().unwrap().is_none());
}

#[test]
fn external_consumer_resolves_tuple_and_set_generic_children_in_context() {
    let context = ReflectionContext::builder()
        .include_type::<BuiltinStructureConsumer>()
        .include_type::<(u8, String)>()
        .include_type::<BTreeSet<u8>>()
        .include_type::<HashSet<String>>()
        .include_type::<Box<[ConsumerChild]>>()
        .include_type::<[ConsumerChild]>()
        .include_type::<ConsumerChild>()
        .include_type::<u8>()
        .include_type::<String>()
        .build()
        .expect("tuple/set descriptors and child types are valid registrations");
    let descriptor = TypeDescriptor::of_in::<BuiltinStructureConsumer>(&context);

    let pair = descriptor.field("pair").expect("tuple field is described");
    let pair_type = pair
        .find_concrete_type()
        .expect("tuple field has a concrete descriptor");
    assert_eq!(pair_type.type_id(), std::any::TypeId::of::<(u8, String)>());
    assert!(std::ptr::eq(pair_type.context(), &context));
    let pair_arguments = pair_type.generic_arguments();
    assert_eq!(pair_arguments.len(), 2);
    assert_eq!(pair_arguments.context().id(), context.id());
    assert_eq!(
        pair_arguments
            .get_at(0)
            .expect("first tuple element argument exists")
            .type_argument()
            .expect("first tuple element is a type")
            .find_concrete_type()
            .expect("u8 element type resolves in the context")
            .type_id(),
        std::any::TypeId::of::<u8>()
    );
    assert_eq!(
        pair_arguments
            .get_at(1)
            .expect("second tuple element argument exists")
            .type_argument()
            .expect("second tuple element is a type")
            .find_concrete_type()
            .expect("String element type resolves in the context")
            .type_id(),
        std::any::TypeId::of::<String>()
    );

    for (name, expected_collection, expected_element) in [
        (
            "ordered",
            std::any::TypeId::of::<BTreeSet<u8>>(),
            std::any::TypeId::of::<u8>(),
        ),
        (
            "hashed",
            std::any::TypeId::of::<HashSet<String>>(),
            std::any::TypeId::of::<String>(),
        ),
    ] {
        let field = descriptor.field(name).expect("set field is described");
        let set = field.find_concrete_type().expect("set field has a concrete descriptor");
        assert_eq!(set.type_id(), expected_collection);
        assert!(std::ptr::eq(set.context(), &context));
        let arguments = set.generic_arguments();
        assert_eq!(arguments.len(), 1);
        assert_eq!(arguments.context().id(), context.id());
        assert_eq!(
            arguments
                .get_at(0)
                .expect("set element argument exists")
                .type_argument()
                .expect("set element is a type")
                .find_concrete_type()
                .expect("set element type resolves in the context")
                .type_id(),
            expected_element
        );
    }

    let boxed_slice = descriptor.field("boxed_slice").expect("boxed slice field is described");
    let boxed_slice_type = boxed_slice
        .find_concrete_type()
        .expect("boxed slice field has a concrete descriptor");
    assert_eq!(
        boxed_slice_type.type_id(),
        std::any::TypeId::of::<Box<[ConsumerChild]>>()
    );
    assert!(std::ptr::eq(boxed_slice_type.context(), &context));
    let arguments = boxed_slice_type.generic_arguments();
    assert_eq!(arguments.len(), 1);
    assert_eq!(arguments.context().id(), context.id());
    let pointee = arguments
        .get_at(0)
        .expect("boxed pointee argument exists")
        .type_argument()
        .expect("boxed pointee is a type argument");
    let pointee_type = pointee
        .find_concrete_type()
        .expect("slice pointee descriptor resolves in the context");
    assert_eq!(pointee_type.type_id(), std::any::TypeId::of::<[ConsumerChild]>());
    assert!(std::ptr::eq(pointee.context(), &context));
    assert!(std::ptr::eq(pointee_type.context(), &context));
    let slice_arguments = pointee_type.generic_arguments();
    assert_eq!(slice_arguments.len(), 1);
    assert_eq!(slice_arguments.context().id(), context.id());
    let element_type = slice_arguments
        .get_at(0)
        .expect("slice element argument exists")
        .type_argument()
        .expect("slice element is a type argument")
        .find_concrete_type()
        .expect("slice element descriptor resolves in the context");
    assert_eq!(element_type.type_id(), std::any::TypeId::of::<ConsumerChild>());
    assert!(std::ptr::eq(element_type.context(), &context));
}

#[test]
fn codegen_type_expression_binding_preserves_nested_generic_structure_and_context() {
    let context = ReflectionContext::builder().build().expect("empty context builds");
    let expression = type_expression_in(&NESTED_GENERIC_TYPE, &context);

    assert_eq!(expression.display().to_string(), "Option<Vec<T>>");
    assert!(std::ptr::eq(expression.context(), &context));
    let TypeExpressionKind::Named(option) = expression.kind() else {
        panic!("outer expression is named");
    };
    assert!(option.find_definition().is_none());
    assert!(std::ptr::eq(option.context(), &context));

    let Some(argument) = option.arguments().get(0) else {
        panic!("Option has one generic argument");
    };
    assert!(std::ptr::eq(argument.context(), &context));
    let GenericExpressionArgumentKind::Type(vector_expression) = argument.kind() else {
        panic!("outer generic argument is a type expression");
    };
    assert_eq!(vector_expression.display().to_string(), "Vec<T>");
    assert!(std::ptr::eq(vector_expression.context(), &context));
    let TypeExpressionKind::Named(vector) = vector_expression.kind() else {
        panic!("nested expression is named");
    };
    assert!(vector.find_definition().is_none());
    let parameter_argument = vector.arguments().get(0).expect("Vec has a parameter argument");
    let GenericExpressionArgumentKind::Type(parameter) = parameter_argument.kind() else {
        panic!("Vec argument is a type parameter expression");
    };
    assert!(matches!(parameter.kind(), TypeExpressionKind::ParameterName("T")));
    assert!(std::ptr::eq(parameter.context(), &context));
}

#[test]
fn codegen_type_expression_binding_preserves_associated_type_arguments() {
    let context = ReflectionContext::builder().build().expect("empty context builds");
    let expression = type_expression_in(&ASSOCIATED_TYPE_PROJECTION, &context);
    assert_eq!(expression.display().to_string(), "T::Assoc<'a>");
    let TypeExpressionKind::AssociatedType(projection) = expression.kind() else {
        panic!("expression is an associated type projection");
    };
    assert_eq!(projection.name(), "Assoc");
    assert!(std::ptr::eq(projection.context(), &context));
    let argument = projection.arguments().get(0).expect("one lifetime argument");
    assert!(std::ptr::eq(argument.context(), &context));
    assert!(matches!(
        argument.kind(),
        GenericExpressionArgumentKind::Lifetime(LifetimeExpression::Named("a"))
    ));
}

#[test]
fn derive_codegen_retains_direct_and_qualified_associated_type_arguments() {
    let context = ReflectionContext::builder()
        .include_type::<AssociatedProjection<AssocProvider>>()
        .build()
        .expect("associated projection model registers");
    let descriptor = context.type_of::<AssociatedProjection<AssocProvider>>();

    for (field_name, expected, has_concrete_type) in [
        ("direct", "T::Assoc<'static>", true),
        ("qualified", "<T as ConsumerAssoc<u8>>::Assoc<'static>", false),
    ] {
        let field = descriptor.field(field_name).expect("projection field exists");
        let expression = field.type_expression();
        assert_eq!(expression.display().to_string(), expected);
        assert!(std::ptr::eq(expression.context(), &context));
        let TypeExpressionKind::AssociatedType(projection) = expression.kind() else {
            panic!("field expression is an associated type projection");
        };
        assert_eq!(projection.name(), "Assoc");
        assert!(std::ptr::eq(projection.context(), &context));
        let argument = projection.arguments().get(0).expect("GAT has one argument");
        assert!(matches!(
            argument.kind(),
            GenericExpressionArgumentKind::Lifetime(LifetimeExpression::Static)
        ));
        assert!(std::ptr::eq(argument.context(), &context));
        if has_concrete_type {
            let concrete = field
                .find_concrete_type()
                .expect("a direct associated field has its concrete type");
            assert_eq!(concrete.type_id(), std::any::TypeId::of::<&'static str>());
            assert!(std::ptr::eq(concrete.context(), &context));
        } else {
            assert!(field.find_concrete_type().is_none());
        }
    }
}

#[test]
fn generated_marker_attaches_capability_to_generic_definition() {
    let context = ReflectionContext::builder()
        .include_linked()
        .build()
        .expect("the linked generic definition and capability register together");
    let definition = context
        .type_definition_by_name("tests::GenericCapabilityMarker")
        .expect("marker definition is registered");
    let capability = definition
        .capability::<ModelCapability>()
        .expect("marker capability is available on the definition");
    assert_eq!(capability.adapter(), &"marker adapter");
    assert!(matches!(
        capability.target(),
        qubit_reflect::capability::CapabilityTarget::Definition(target)
            if target.id() == definition.id() && std::ptr::eq(target.context(), &context)
    ));
}

#[test]
fn derive_generated_marker_hook_registers_generic_definition_capability() {
    let context = ReflectionContext::builder()
        .include_linked()
        .build()
        .expect("the generated marker hook registers definition metadata and capability");
    let definition = context
        .type_definition_by_name("external_consumer_vnext_contract_tests::GenericConsumerModel")
        .expect("generic definition is registered by derive");
    let capability = definition
        .capability::<ModelCapability>()
        .expect("generated marker hook binds capability to definition");
    assert_eq!(capability.adapter(), &"generated model adapter");
    assert!(matches!(
        capability.target(),
        CapabilityTarget::Definition(target)
            if target.id() == definition.id() && std::ptr::eq(target.context(), &context)
    ));
}

#[test]
fn external_consumer_can_register_capability_for_generic_definition() {
    let source = ReflectionContext::builder()
        .include_type::<GenericConsumerModel<u8>>()
        .build()
        .expect("generic model can be reflected");
    let definition_id = source
        .type_of::<GenericConsumerModel<u8>>()
        .definition()
        .expect("generic application has a source definition")
        .id();

    let context = ReflectionContext::builder()
        .include_type::<GenericConsumerModel<u8>>()
        .register(CapabilityRegistration::for_definition::<ModelCapability>(
            definition_id,
            "generic",
        ))
        .build()
        .expect("definition capability registration builds");
    let definition = context
        .type_of::<GenericConsumerModel<u8>>()
        .definition()
        .expect("generic application has a context-bound definition");
    let capability = definition
        .capability::<ModelCapability>()
        .expect("definition capability is available");

    assert_eq!(capability.adapter(), &"generic");
    assert!(matches!(capability.target(), CapabilityTarget::Definition(target)
        if target.id() == definition.id() && std::ptr::eq(target.context(), &context)));
    assert!(matches!(capability.origin(), CapabilityOrigin::Explicit { .. }));
}

#[test]
fn external_consumer_reports_wrong_adapter_and_keeps_global_context_isolated() {
    let explicit = ReflectionContext::builder()
        .include_type::<ConsumerModel>()
        .register(CapabilityRegistration::for_type::<ConsumerModel, WrongModelCapability>(
            42,
        ))
        .build()
        .expect("the registered adapter is valid for its own capability spec");
    let explicit_type = explicit.type_of::<ConsumerModel>();
    assert!(matches!(
        explicit_type.capability::<ModelCapability>(),
        Err(qubit_reflect::capability::CapabilityError::ContractMismatch { .. })
    ));

    let global = ReflectionContext::global();
    let global_type = TypeDescriptor::of_in::<ConsumerModel>(global);
    assert!(std::ptr::eq(global_type.context(), global));
    assert!(global_type.find_capability::<ModelCapability>().unwrap().is_none());
    assert!(
        explicit_type
            .find_capability::<WrongModelCapability>()
            .unwrap()
            .is_some()
    );
}
