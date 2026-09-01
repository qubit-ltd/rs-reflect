// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow explicit-imports
//! Isolated integration coverage for root aggregation when registry startup
//! fails.

use std::any::TypeId;

use qubit_reflect as reflect;
use qubit_reflect::__private::registration::FragmentKind;
use qubit_reflect::__private::registration::FragmentPayload;
use qubit_reflect::__private::registration::RegistrationFragment;
use qubit_reflect::__private::registration::RuntimeIdentity;
use qubit_reflect::__private::registration::StaticFragmentIdentity;
use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::FieldDescriptor;
use qubit_reflect::descriptor::OpaqueTypeDescriptor;
use qubit_reflect::descriptor::StructKind;
use qubit_reflect::descriptor::TypeRef;
use qubit_reflect::error::RegistryErrorKind;
use qubit_reflect::identity::Visibility;

struct RegistryIndependentShape {
    value: u8,
}

/// Resolves the root used by the field fixture.
fn registry_independent_shape_descriptor() -> &'static TypeDescriptor {
    &REGISTRY_INDEPENDENT_SHAPE
}

static VALUE_TYPE_DESCRIPTOR: OpaqueTypeDescriptor = reflect::__private::descriptor::opaque_member::<u8>();
static VALUE_TYPE: TypeRef = TypeRef::Opaque(&VALUE_TYPE_DESCRIPTOR);
static FIELDS: [FieldDescriptor; 1] = [reflect::__private::descriptor::field(
    registry_independent_shape_descriptor,
    0,
    Some("value"),
    Some("value"),
    &VALUE_TYPE,
    Visibility::Private,
)];
static REGISTRY_INDEPENDENT_SHAPE: TypeDescriptor = reflect::__private::descriptor::struct_type::<
    RegistryIndependentShape,
>("RegistryIndependentShape", StructKind::Named, &FIELDS);

impl Reflect for RegistryIndependentShape {
    /// Returns a structurally valid root that does not initialize the global
    /// registry.
    fn type_descriptor() -> &'static TypeDescriptor {
        &REGISTRY_INDEPENDENT_SHAPE
    }
}

const CONFLICTING_IDENTITY: StaticFragmentIdentity =
    StaticFragmentIdentity::new("type-descriptor-registry-failure-fixture", "conflict", 1, 1, "type", 1);

/// Returns the first conflicting fragment target.
fn first_target_identity() -> RuntimeIdentity {
    RuntimeIdentity::Type(TypeId::of::<RegistryIndependentShape>())
}

/// Returns the second conflicting fragment target.
fn second_target_identity() -> RuntimeIdentity {
    RuntimeIdentity::Type(TypeId::of::<u8>())
}

/// Builds the first conflicting fragment payload.
fn first_payload() -> FragmentPayload {
    FragmentPayload::Type(TypeDescriptor::of::<RegistryIndependentShape>())
}

/// Builds the second conflicting fragment payload.
fn second_payload() -> FragmentPayload {
    FragmentPayload::Type(TypeDescriptor::of::<u8>())
}

reflect::__private::inventory::submit! {
    RegistrationFragment::new(
        FragmentKind::Type,
        CONFLICTING_IDENTITY,
        first_target_identity,
        first_payload,
    )
}

reflect::__private::inventory::submit! {
    RegistrationFragment::new(
        FragmentKind::Type,
        CONFLICTING_IDENTITY,
        second_target_identity,
        second_payload,
    )
}

/// Verifies aggregation methods propagate one cached registry error while
/// direct shape navigation remains usable.
#[test]
fn test_type_descriptor_aggregation_propagates_cached_registry_error() {
    let value = RegistryIndependentShape { value: 7 };
    let descriptor = TypeDescriptor::of::<RegistryIndependentShape>();
    let first = descriptor
        .impls()
        .expect_err("duplicate registration fragments must fail initialization");
    let second = descriptor
        .methods()
        .expect_err("method aggregation must propagate the cached error");
    let third = descriptor
        .methods_named("anything")
        .expect_err("method lookup must propagate the cached error");

    assert_eq!(value.value, 7);
    assert_eq!(first.kind(), RegistryErrorKind::DuplicateFragment);
    assert_eq!(second.kind(), first.kind());
    assert_eq!(third.kind(), first.kind());
    assert_eq!(second.conflicting_fragments(), first.conflicting_fragments());
    assert_eq!(third.conflicting_fragments(), first.conflicting_fragments());
    assert_eq!(descriptor.fields().len(), 1);
    assert_eq!(
        descriptor
            .field("value")
            .expect("shape lookup must remain independent")
            .index(),
        0
    );
}
