// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Focused crate-unit coverage for small public accessors that otherwise only
//! appear in separately instrumented integration-test code generation units.

use std::any::TypeId;
use std::error::Error;
use std::sync::LazyLock;

use crate::access::FieldAccessError;
use crate::access::FieldAccessOperation;
use crate::access::FieldIdentity;
use crate::access::FieldSetFailure;
use crate::capability::CapabilityConflictKind;
use crate::capability::TypeCapabilities;
use crate::capability::clone_descriptor;
use crate::capability::clone_key;
use crate::capability::default_descriptor;
use crate::capability::default_key;
use crate::capability::send_descriptor;
use crate::capability::send_key;
use crate::capability::sync_descriptor;
use crate::capability::sync_key;
use crate::descriptor::ConcreteGenericDescriptor;
use crate::error::RegistryError;
use crate::error::TypeMismatch;
use crate::expression::ArrayTypeExpression;
use crate::expression::AssociatedTypeExpression;
use crate::expression::ConcreteTypeExpression;
use crate::expression::ConstExpression;
use crate::expression::DiagnosticText;
use crate::expression::FunctionAbi;
use crate::expression::FunctionPointerExpression;
use crate::expression::FunctionSafety;
use crate::expression::GenericDefinitionDescriptor;
use crate::expression::GenericParameterDescriptor;
use crate::expression::LifetimeExpression;
use crate::expression::OpaqueTypeExpression;
use crate::expression::PredicateDescriptor;
use crate::expression::RawPointerTypeExpression;
use crate::expression::ReferenceTypeExpression;
use crate::expression::TraitBoundModifier;
use crate::expression::TraitObjectExpression;
use crate::expression::TypeExpression;
use crate::identity::FragmentIdentity;
use crate::identity::MemberId;
use crate::invoke::InvocationArg;
use crate::invoke::InvocationBinding;
use crate::registry::ReflectRegistry;
use crate::value::DynamicMut;
use crate::value::DynamicOwned;
use crate::value::DynamicRef;
use crate::value::Local;
use crate::value::ReflectedOwned;

static EMPTY_GENERIC: LazyLock<GenericDefinitionDescriptor> = LazyLock::new(|| GenericDefinitionDescriptor {
    parameters: Box::new([]),
    predicates: Box::new([]),
    diagnostic: DiagnosticText::default(),
});

fn fragment(fingerprint: u64) -> FragmentIdentity {
    FragmentIdentity::new("crate", "crate::module", 10, 4, "field", fingerprint)
}

#[test]
fn test_small_identity_and_error_accessors_preserve_input_facts() {
    let identity = fragment(17);
    assert_eq!(identity.declaring_crate(), "crate");
    assert_eq!(identity.module_path(), "crate::module");
    assert_eq!(identity.line(), 10);
    assert_eq!(identity.column(), 4);
    assert_eq!(identity.member_kind(), "field");
    assert_eq!(identity.content_fingerprint(), 17);
    assert!(identity.same_source_identity(&fragment(18)));
    let member = MemberId::new("Type", "field", 1, identity.clone());
    assert_eq!(member.fragment(), &identity);

    let direct = FieldIdentity::new(TypeId::of::<u8>(), "u8", 0, Some("value"));
    assert_eq!(direct.rust_name(), Some("value"));
    let variant = FieldIdentity::new_variant(TypeId::of::<u8>(), "u8", 0, None, 2, "Ready");
    assert_eq!(variant.variant_rust_name(), Some("Ready"));

    let mismatch = TypeMismatch::new(TypeId::of::<u8>(), TypeId::of::<u16>()).with_diagnostic_names("u8", "u16");
    assert_eq!(mismatch.expected(), TypeId::of::<u8>());
    assert_eq!(mismatch.actual(), TypeId::of::<u16>());
    assert_eq!(mismatch.expected_name(), Some("u8"));
    assert_eq!(mismatch.actual_name(), Some("u16"));

    let conflict = RegistryError::duplicate_fragment(fragment(1), fragment(2));
    assert!(conflict.conflicting_fragments().is_some());
}

#[test]
fn test_expression_constructors_preserve_navigable_structural_facts() {
    let concrete = ConcreteTypeExpression::new(
        ["core", "option", "Option"],
        [crate::expression::GenericArgument::Type(TypeExpression::SelfType)],
    )
    .expect("the concrete path is non-empty")
    .with_diagnostic("Option<Self>");
    assert_eq!(
        concrete.path().iter().map(AsRef::as_ref).collect::<Vec<_>>(),
        ["core", "option", "Option"]
    );
    assert_eq!(concrete.arguments().len(), 1);
    assert_eq!(concrete.diagnostic(), Some("Option<Self>"));

    let associated = AssociatedTypeExpression::new(
        TypeExpression::Parameter("T".into()),
        Some(TypeExpression::Parameter("Iterator".into())),
        "Item",
        Box::<[crate::expression::GenericArgument]>::default(),
    )
    .with_diagnostic("<T as Iterator>::Item");
    assert_eq!(associated.self_type(), &TypeExpression::Parameter("T".into()));
    assert_eq!(
        associated.trait_path(),
        Some(&TypeExpression::Parameter("Iterator".into()))
    );
    assert_eq!(associated.item(), "Item");
    assert!(associated.arguments().is_empty());
    assert_eq!(associated.diagnostic(), Some("<T as Iterator>::Item"));

    let reference = ReferenceTypeExpression::new(LifetimeExpression::Named("a".into()), true, TypeExpression::SelfType)
        .with_diagnostic("&'a mut Self");
    assert_eq!(reference.lifetime(), &LifetimeExpression::Named("a".into()));
    assert!(reference.is_mutable());
    assert_eq!(reference.target(), &TypeExpression::SelfType);
    assert_eq!(reference.diagnostic(), Some("&'a mut Self"));

    let raw = RawPointerTypeExpression::new(false, TypeExpression::SelfType).with_diagnostic("*const Self");
    assert!(!raw.is_mutable());
    assert_eq!(raw.target(), &TypeExpression::SelfType);
    assert_eq!(raw.diagnostic(), Some("*const Self"));

    let array = ArrayTypeExpression::new(TypeExpression::SelfType, ConstExpression::UnsignedInteger(3))
        .with_diagnostic("[Self; 3]");
    assert_eq!(array.element(), &TypeExpression::SelfType);
    assert_eq!(array.length(), &ConstExpression::UnsignedInteger(3));
    assert_eq!(array.diagnostic(), Some("[Self; 3]"));

    let function = FunctionPointerExpression::new(
        FunctionAbi::C,
        FunctionSafety::Unsafe,
        true,
        [LifetimeExpression::Named("a".into())],
        [TypeExpression::SelfType],
        TypeExpression::Never,
    )
    .with_diagnostic("unsafe extern C fn(Self, ...) -> !");
    assert_eq!(function.abi(), &FunctionAbi::C);
    assert_eq!(function.safety(), &FunctionSafety::Unsafe);
    assert!(function.is_variadic());
    assert_eq!(
        function.higher_ranked_lifetimes(),
        &[LifetimeExpression::Named("a".into())]
    );
    assert_eq!(function.parameters(), &[TypeExpression::SelfType]);
    assert_eq!(function.return_type(), &TypeExpression::Never);
    assert_eq!(function.diagnostic(), Some("unsafe extern C fn(Self, ...) -> !"));

    let predicate = PredicateDescriptor::type_bound(
        TypeExpression::Parameter("T".into()),
        [TypeExpression::Parameter("Display".into())],
        [TraitBoundModifier::None],
        Box::<[LifetimeExpression]>::default(),
    )
    .expect("one modifier is supplied for the non-empty bound")
    .with_diagnostic("T: Display");
    assert_eq!(predicate.diagnostic(), Some("T: Display"));
    let outlives =
        PredicateDescriptor::lifetime_outlives(LifetimeExpression::Named("a".into()), [LifetimeExpression::Static])
            .expect("the lifetime bound is non-empty");

    let trait_object = TraitObjectExpression::new([predicate.clone()]).with_diagnostic("dyn Display");
    assert_eq!(trait_object.bounds(), std::slice::from_ref(&predicate));
    assert_eq!(trait_object.diagnostic(), Some("dyn Display"));
    let opaque = OpaqueTypeExpression::new([predicate.clone()]).with_diagnostic("impl Display");
    assert_eq!(opaque.bounds(), std::slice::from_ref(&predicate));
    assert_eq!(opaque.diagnostic(), Some("impl Display"));

    let definition = GenericDefinitionDescriptor::new(
        [GenericParameterDescriptor::Lifetime {
            name: "a".into(),
            bounds: vec![LifetimeExpression::Static].into_boxed_slice(),
            diagnostic: DiagnosticText::default(),
        }],
        [outlives],
    )
    .with_diagnostic("<'a> where 'a: 'static");
    assert_eq!(definition.parameters().len(), 1);
    assert_eq!(definition.predicates().len(), 1);
    assert_eq!(definition.diagnostic(), Some("<'a> where 'a: 'static"));
}

#[test]
fn test_small_generic_and_invocation_accessors_preserve_input_facts() {
    let diagnostic = DiagnosticText::from(String::from("source"));
    assert_eq!(diagnostic.0.as_deref(), Some("source"));

    let generic = ConcreteGenericDescriptor::new(&EMPTY_GENERIC, &[]);
    assert!(std::ptr::eq(generic.definition(), &*EMPTY_GENERIC));
    assert!(generic.arguments().is_empty());

    let binding = InvocationBinding::<Local>::named("value", InvocationArg::Owned(DynamicOwned::<Local>::new(3_u8)));
    assert_eq!(binding.name(), Some("value"));
    assert!(matches!(binding.argument(), InvocationArg::Owned(_)));
}

fn rejected_field_value(value: u8) -> FieldSetFailure {
    let field = FieldIdentity::new(TypeId::of::<u16>(), "u16", 3, Some("value"));
    FieldSetFailure::before_execution(
        FieldAccessError::Unavailable {
            field: field.clone(),
            operation: FieldAccessOperation::Set,
        },
        field,
        Some("value"),
        ReflectedOwned::new(value),
    )
}

fn owned_u8(value: ReflectedOwned) -> u8 {
    match value.downcast::<u8>() {
        Ok(value) => value,
        Err(_) => panic!("test recovery must preserve the original u8"),
    }
}

#[test]
fn test_field_failure_recovery_preserves_identity_phase_and_owned_value() {
    let direct = FieldIdentity::new(TypeId::of::<u16>(), "u16", 3, Some("value"));
    assert_eq!(direct.declaring_type(), TypeId::of::<u16>());
    assert_eq!(direct.declaring_type_name(), "u16");
    assert_eq!(direct.index(), 3);
    assert_eq!(direct.variant_index(), None);
    assert_eq!(direct.to_string(), "u16::value");
    assert_eq!(FieldAccessOperation::Set.to_string(), "set");

    let variant = FieldIdentity::new_variant(TypeId::of::<u16>(), "u16", 3, None, 2, "Ready");
    assert_eq!(variant.variant_index(), Some(2));
    assert_eq!(variant.to_string(), "u16::Ready field #3");
    let inactive = FieldAccessError::inactive_variant(variant.clone(), 2, "Ready");
    assert_eq!(inactive.field(), &variant);

    let failure = rejected_field_value(7);
    assert_eq!(failure.error().field(), &direct);
    let recovery = failure.recovery().expect("validation failures retain input");
    assert_eq!(recovery.field(), &direct);
    assert_eq!(recovery.query_name(), Some("value"));
    assert_eq!(recovery.value().downcast_ref::<u8>(), Some(&7));
    assert_eq!(
        recovery
            .value_by_name("value")
            .and_then(|value| value.downcast_ref::<u8>()),
        Some(&7)
    );
    assert_eq!(
        recovery.value_at(3).and_then(|value| value.downcast_ref::<u8>()),
        Some(&7)
    );
    assert!(recovery.value_by_name("other").is_none());
    assert!(recovery.value_at(4).is_none());
    assert!(format!("{failure:?}").contains("recovery"));
    assert_eq!(failure.to_string(), failure.error().to_string());
    assert!(failure.source().is_some());
    assert_eq!(AsRef::<FieldAccessError>::as_ref(&failure), failure.error());

    let (error, recovery) = rejected_field_value(8).into_parts();
    assert_eq!(error.field(), &direct);
    assert_eq!(
        owned_u8(recovery.expect("recovery must survive decomposition").into_value()),
        8
    );
    let recovery = rejected_field_value(9)
        .into_recovery()
        .expect("validation failure retains recovery");
    let recovery = match recovery.into_value_by_name("other") {
        Ok(_) => panic!("a wrong name must retain recovery"),
        Err(recovery) => recovery,
    };
    assert_eq!(
        owned_u8(
            recovery
                .into_value_by_name("value")
                .expect("matching name extracts value")
        ),
        9
    );
    let recovery = rejected_field_value(10)
        .into_recovery()
        .expect("validation failure retains recovery");
    let recovery = match recovery.into_value_at(4) {
        Ok(_) => panic!("a wrong index must retain recovery"),
        Err(recovery) => recovery,
    };
    assert_eq!(
        owned_u8(recovery.into_value_at(3).expect("matching index extracts value")),
        10
    );

    let adapter_error = FieldAccessError::Unavailable {
        field: direct,
        operation: FieldAccessOperation::Set,
    };
    let failure = FieldSetFailure::after_execution(adapter_error.clone());
    assert!(failure.recovery().is_none());
    match failure.into_recovery() {
        Ok(_) => panic!("an adapter failure must not synthesize recovery"),
        Err(error) => assert_eq!(error, adapter_error),
    }
    assert_eq!(
        FieldSetFailure::after_execution(adapter_error.clone()).into_error(),
        adapter_error
    );
}

#[test]
fn test_dynamic_type_probes_and_capability_adapters_enforce_exact_contracts() {
    let shared = 7_u8;
    assert_eq!(
        crate::access::field_adapter::dynamic_ref_type_id(&DynamicRef::<Local>::new(&shared)),
        TypeId::of::<u8>(),
    );
    let mut mutable = 8_u16;
    assert_eq!(
        crate::access::field_adapter::dynamic_mut_type_id(&DynamicMut::<Local>::new(&mut mutable)),
        TypeId::of::<u16>(),
    );
    let owned = DynamicOwned::<Local>::new(9_u32);
    assert_eq!(
        crate::access::field_adapter::dynamic_owned_type_id(&owned),
        TypeId::of::<u32>(),
    );

    let capabilities = TypeCapabilities::try_new(vec![
        default_descriptor::<String>(),
        send_descriptor::<String>(),
        clone_descriptor::<String>(),
        sync_descriptor::<String>(),
    ])
    .expect("distinct built-in capabilities form a valid set");
    assert!(capabilities.contains(send_key()));
    assert!(capabilities.contains(sync_key()));
    let clone_adapter = capabilities.get(clone_key()).expect("clone adapter is registered");
    let source = DynamicOwned::<Local>::new(String::from("value"));
    let cloned = clone_adapter
        .clone_owned(&source)
        .expect("exact source type can be cloned");
    assert_eq!(cloned.downcast_ref::<String>().map(String::as_str), Some("value"));
    assert!(clone_adapter.clone_owned(&DynamicOwned::<Local>::new(1_u8)).is_err());
    let defaulted = capabilities
        .get(default_key())
        .expect("default adapter is registered")
        .create();
    assert_eq!(defaulted.downcast_ref::<String>().map(String::as_str), Some(""));
    assert!(format!("{:?}", capabilities.descriptors()[0]).contains("CapabilityDescriptor"));

    let duplicate = TypeCapabilities::try_new(vec![send_descriptor::<u8>(), send_descriptor::<u16>()])
        .expect_err("one stable capability ID cannot be declared twice");
    assert_eq!(duplicate.kind(), CapabilityConflictKind::DuplicateId);
    assert_eq!(duplicate.id(), send_key().id());
    assert_eq!(duplicate.first_adapter_type(), TypeId::of::<()>());
    assert_eq!(duplicate.second_adapter_type(), TypeId::of::<()>());
}

#[test]
fn test_registry_query_views_preserve_empty_and_exact_lookup_contracts() {
    let registry = ReflectRegistry::initialize().expect("the linked unit-test inventory is valid");
    let u8_descriptor = registry
        .get(TypeId::of::<u8>())
        .expect("built-in integer registration is linked");

    let by_type_name = registry.find_by_type_name(u8_descriptor.type_name());
    assert!(!by_type_name.is_empty());
    assert!(
        by_type_name
            .iter()
            .any(|candidate| std::ptr::eq(candidate, u8_descriptor))
    );
    assert_eq!(by_type_name.len(), by_type_name.into_iter().count());

    let by_query_name = registry.find_by_query_name(u8_descriptor.query_name());
    assert!(!by_query_name.is_empty());
    assert!(
        by_query_name
            .iter()
            .any(|candidate| std::ptr::eq(candidate, u8_descriptor))
    );
    assert!(registry.find_by_query_name("missing::query").is_empty());

    let definitions = registry.find_impl_definitions_by_target(&TypeExpression::Never);
    assert!(definitions.is_empty());
    assert_eq!(definitions.len(), definitions.iter().count());
    assert_eq!(definitions.into_iter().count(), 0);
    assert!(registry.implementations(TypeId::of::<u8>()).is_empty());
    assert!(registry.impl_definitions().iter().all(|definition| {
        registry
            .find_impl_definitions_by_target(definition.target_type())
            .iter()
            .any(|candidate| std::ptr::eq(candidate, *definition))
    }));

    assert!(registry.effective_view(TypeId::of::<u8>()).methods().is_empty());
    assert!(registry.trait_definition_by_path("missing::Trait").is_none());
    let traits = registry.find_trait_definitions_by_path("missing::Trait");
    assert!(traits.is_empty());
    assert_eq!(traits.len(), 0);
    assert_eq!(traits.iter().count(), 0);
    assert!(traits.only().is_none());
    assert_eq!(traits.into_iter().count(), 0);
}
