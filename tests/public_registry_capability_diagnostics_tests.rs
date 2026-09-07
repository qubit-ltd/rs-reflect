// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Public diagnostics for capability conflicts during registry construction.

use std::any::TypeId;
use std::sync::LazyLock;
use std::sync::OnceLock;

use qubit_reflect::__private::codegen_v3::descriptor::opaque_root;
use qubit_reflect::__private::codegen_v3::registration::CapabilityRegistration;
use qubit_reflect::__private::codegen_v3::registration::FragmentKind;
use qubit_reflect::__private::codegen_v3::registration::FragmentPayload;
use qubit_reflect::__private::codegen_v3::registration::RegistrationFragment;
use qubit_reflect::__private::codegen_v3::registration::RuntimeIdentity;
use qubit_reflect::__private::codegen_v3::registration::StaticFragmentIdentity;
use qubit_reflect::__private::testing::build_registry;
use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::capability::CapabilityConflict;
use qubit_reflect::capability::CapabilityConflictKind;
use qubit_reflect::capability::CapabilityDescriptor;
use qubit_reflect::capability::CapabilityKey;
use qubit_reflect::capability::TypeCapabilities;
use qubit_reflect::descriptor::TypeDefinitionDescriptor;
use qubit_reflect::descriptor::TypeDefinitionId;
use qubit_reflect::error::RegistryError;
use qubit_reflect::expression::GenericDefinitionDescriptor;
use qubit_reflect::identity::CapabilityId;
use qubit_reflect::identity::FragmentIdentity;
use qubit_reflect::registry::CapabilityTarget;

struct ConcreteTarget;
struct IntrinsicTarget;
struct DefinitionMarker;

fn key<A: 'static>(id: &'static str) -> CapabilityKey<A> {
    CapabilityKey::new(CapabilityId::new(id).expect("fixture capability ID"))
}

fn source(declaring_crate: &'static str, line: u32) -> FragmentIdentity {
    FragmentIdentity::new(declaring_crate, "diagnostics", line, 1, "capability", u64::from(line))
}

fn concrete_identity() -> RuntimeIdentity {
    RuntimeIdentity::Capabilities(CapabilityTarget::Type(TypeId::of::<ConcreteTarget>()))
}

fn mismatch_left_payload() -> FragmentPayload {
    FragmentPayload::Capability(CapabilityRegistration::for_type_id(
        TypeId::of::<ConcreteTarget>(),
        vec![CapabilityDescriptor::with_adapter(key("example.contract"), 7_u32)],
    ))
}

fn mismatch_right_payload() -> FragmentPayload {
    FragmentPayload::Capability(CapabilityRegistration::for_type_id(
        TypeId::of::<ConcreteTarget>(),
        vec![CapabilityDescriptor::with_adapter(key("example.contract"), 9_u64)],
    ))
}

static MISMATCH_LEFT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Capability,
    StaticFragmentIdentity::new("a-source", "diagnostics", 10, 1, "capability", 10),
    concrete_identity,
    mismatch_left_payload,
);
static MISMATCH_RIGHT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Capability,
    StaticFragmentIdentity::new("b-source", "diagnostics", 20, 1, "capability", 20),
    concrete_identity,
    mismatch_right_payload,
);

fn duplicate_left_payload() -> FragmentPayload {
    FragmentPayload::Capability(CapabilityRegistration::for_type_id(
        TypeId::of::<ConcreteTarget>(),
        vec![CapabilityDescriptor::with_adapter(key("example.duplicate"), 1_u32)],
    ))
}

fn duplicate_right_payload() -> FragmentPayload {
    FragmentPayload::Capability(CapabilityRegistration::for_type_id(
        TypeId::of::<ConcreteTarget>(),
        vec![CapabilityDescriptor::with_adapter(key("example.duplicate"), 2_u32)],
    ))
}

static DUPLICATE_LEFT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Capability,
    StaticFragmentIdentity::new("a-source", "diagnostics", 30, 1, "capability", 30),
    concrete_identity,
    duplicate_left_payload,
);
static DUPLICATE_RIGHT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Capability,
    StaticFragmentIdentity::new("b-source", "diagnostics", 40, 1, "capability", 40),
    concrete_identity,
    duplicate_right_payload,
);

fn fact_left_payload() -> FragmentPayload {
    FragmentPayload::Capability(CapabilityRegistration::for_type_id(
        TypeId::of::<ConcreteTarget>(),
        vec![CapabilityDescriptor::without_adapter(key::<u32>("example.fact"))],
    ))
}

fn executable_right_payload() -> FragmentPayload {
    FragmentPayload::Capability(CapabilityRegistration::for_type_id(
        TypeId::of::<ConcreteTarget>(),
        vec![CapabilityDescriptor::with_adapter(key("example.fact"), 3_u32)],
    ))
}

static FACT_LEFT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Capability,
    StaticFragmentIdentity::new("a-source", "diagnostics", 50, 1, "capability", 50),
    concrete_identity,
    fact_left_payload,
);
static EXECUTABLE_RIGHT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Capability,
    StaticFragmentIdentity::new("b-source", "diagnostics", 60, 1, "capability", 60),
    concrete_identity,
    executable_right_payload,
);

static DEFINITION_GENERICS: LazyLock<GenericDefinitionDescriptor> =
    LazyLock::new(|| GenericDefinitionDescriptor::new([], []));
static DEFINITION: LazyLock<TypeDefinitionDescriptor> = LazyLock::new(|| {
    TypeDefinitionDescriptor::opaque(
        TypeDefinitionId::of::<DefinitionMarker>(),
        "diagnostics::DefinitionTarget",
        "DefinitionTarget",
        &DEFINITION_GENERICS,
    )
});

fn definition() -> &'static TypeDefinitionDescriptor {
    &DEFINITION
}

fn definition_identity() -> RuntimeIdentity {
    RuntimeIdentity::Capabilities(CapabilityTarget::TypeDefinition(definition().id()))
}

fn definition_left_payload() -> FragmentPayload {
    FragmentPayload::Capability(CapabilityRegistration::for_definition(
        definition(),
        vec![CapabilityDescriptor::with_adapter(key("example.definition"), 5_u32)],
    ))
}

fn definition_right_payload() -> FragmentPayload {
    FragmentPayload::Capability(CapabilityRegistration::for_definition(
        definition(),
        vec![CapabilityDescriptor::with_adapter(key("example.definition"), 8_u64)],
    ))
}

static DEFINITION_LEFT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Capability,
    StaticFragmentIdentity::new("a-source", "diagnostics", 70, 1, "capability", 70),
    definition_identity,
    definition_left_payload,
);
static DEFINITION_RIGHT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Capability,
    StaticFragmentIdentity::new("b-source", "diagnostics", 80, 1, "capability", 80),
    definition_identity,
    definition_right_payload,
);

fn intrinsic_capabilities() -> Result<&'static TypeCapabilities, CapabilityConflict> {
    static RESULT: OnceLock<Result<TypeCapabilities, CapabilityConflict>> = OnceLock::new();
    RESULT
        .get_or_init(|| {
            TypeCapabilities::try_new(vec![
                CapabilityDescriptor::with_adapter(key("example.intrinsic"), 1_u16),
                CapabilityDescriptor::with_adapter(key("example.intrinsic"), 2_u16),
            ])
        })
        .as_ref()
        .map_err(Clone::clone)
}

impl Reflect for IntrinsicTarget {
    fn type_descriptor() -> &'static TypeDescriptor {
        static DESCRIPTOR: TypeDescriptor =
            opaque_root::<IntrinsicTarget>("IntrinsicTarget").with_capabilities(intrinsic_capabilities);
        &DESCRIPTOR
    }
}

fn intrinsic_identity() -> RuntimeIdentity {
    RuntimeIdentity::Type(TypeId::of::<IntrinsicTarget>())
}

fn intrinsic_payload() -> FragmentPayload {
    FragmentPayload::Type(IntrinsicTarget::type_descriptor())
}

static INTRINSIC_FRAGMENT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Type,
    StaticFragmentIdentity::new("intrinsic-source", "diagnostics", 90, 1, "type", 90),
    intrinsic_identity,
    intrinsic_payload,
);

#[test]
fn test_cross_fragment_mismatch_retains_complete_ordered_diagnostics() {
    let error = build_registry(&[&MISMATCH_RIGHT, &MISMATCH_LEFT]).expect_err("conflict");

    let capability_id = error.capability_id().expect("capability ID");
    assert_eq!(capability_id.as_str(), "example.contract");
    assert_eq!(
        error.capability_target(),
        Some(CapabilityTarget::Type(TypeId::of::<ConcreteTarget>()))
    );
    let detail = error.capability_details().expect("full capability context");
    assert_eq!(detail.kind(), CapabilityConflictKind::AdapterTypeMismatch);
    assert_eq!(detail.first_adapter_type(), TypeId::of::<u32>());
    assert_eq!(detail.second_adapter_type(), TypeId::of::<u64>());
    let (left, right) = error.conflicting_fragments().expect("both sources");
    assert_eq!(left, &source("a-source", 10));
    assert_eq!(right, &source("b-source", 20));
    assert!(std::error::Error::source(&error).is_some());
    assert!(error.intrinsic_conflict().is_none());
    let message = error.to_string();
    assert!(message.contains("example.contract"));
    assert!(message.contains("AdapterTypeMismatch"));
}

#[test]
fn test_cross_fragment_duplicate_and_fact_conflicts_share_classification() {
    for (fragments, id) in [
        ([&DUPLICATE_LEFT, &DUPLICATE_RIGHT], "example.duplicate"),
        ([&FACT_LEFT, &EXECUTABLE_RIGHT], "example.fact"),
    ] {
        let error = build_registry(&fragments).expect_err("duplicate capability ID");
        let detail = error.capability_details().expect("full capability context");
        let capability_id = error.capability_id().expect("capability ID");
        assert_eq!(capability_id.as_str(), id);
        assert_eq!(detail.kind(), CapabilityConflictKind::DuplicateId);
        assert_eq!(detail.first_adapter_type(), TypeId::of::<u32>());
        assert_eq!(detail.second_adapter_type(), TypeId::of::<u32>());
    }
}

#[test]
fn test_definition_target_is_reported_with_both_contracts() {
    let error = build_registry(&[&DEFINITION_RIGHT, &DEFINITION_LEFT]).expect_err("definition conflict");

    assert_eq!(
        error.capability_target(),
        Some(CapabilityTarget::TypeDefinition(definition().id()))
    );
    let detail = error.capability_details().expect("full capability context");
    assert_eq!(detail.kind(), CapabilityConflictKind::AdapterTypeMismatch);
    assert_eq!(detail.first_adapter_type(), TypeId::of::<u32>());
    assert_eq!(detail.second_adapter_type(), TypeId::of::<u64>());
}

#[test]
fn test_intrinsic_conflict_retains_target_source_and_error_chain() {
    let error = build_registry(&[&INTRINSIC_FRAGMENT]).expect_err("intrinsic conflict");

    let capability_id = error.capability_id().expect("capability ID");
    assert_eq!(capability_id.as_str(), "example.intrinsic");
    assert_eq!(
        error.capability_target(),
        Some(CapabilityTarget::Type(TypeId::of::<IntrinsicTarget>()))
    );
    let detail = error.capability_details().expect("full capability context");
    assert_eq!(detail.kind(), CapabilityConflictKind::DuplicateId);
    assert_eq!(detail.first_adapter_type(), TypeId::of::<u16>());
    assert_eq!(detail.second_adapter_type(), TypeId::of::<u16>());
    assert_eq!(error.intrinsic_conflict(), Some(detail));
    let intrinsic_source = FragmentIdentity::new("intrinsic-source", "diagnostics", 90, 1, "type", 90);
    assert_eq!(error.fragment_identity(), Some(&intrinsic_source));
    assert_eq!(
        std::error::Error::source(&error).and_then(|cause| cause.downcast_ref::<CapabilityConflict>()),
        Some(detail)
    );
}

#[test]
fn test_source_order_normalizes_complete_diagnostics() {
    let forward = build_registry(&[&MISMATCH_LEFT, &MISMATCH_RIGHT]).expect_err("conflict");
    let reverse = build_registry(&[&MISMATCH_RIGHT, &MISMATCH_LEFT]).expect_err("conflict");

    assert_eq!(forward, reverse);
}

#[test]
fn test_legacy_constructors_remain_usable_without_inventing_context() {
    let left = source("legacy-left", 100);
    let right = source("legacy-right", 110);
    let legacy = RegistryError::capability_conflict(left, right);
    assert!(legacy.capability_details().is_none());
    assert!(legacy.capability_target().is_none());
    assert!(legacy.capability_id().is_none());
    assert!(std::error::Error::source(&legacy).is_none());

    let conflict = TypeCapabilities::try_new(vec![
        CapabilityDescriptor::without_adapter(key::<u8>("example.legacy")),
        CapabilityDescriptor::without_adapter(key::<u8>("example.legacy")),
    ])
    .expect_err("fixture conflict");
    let intrinsic = RegistryError::intrinsic_capability_conflict(source("legacy", 120), conflict.clone());
    assert_eq!(intrinsic.capability_details(), Some(&conflict));
    assert_eq!(intrinsic.intrinsic_conflict(), Some(&conflict));
    assert!(intrinsic.capability_target().is_none());
}
