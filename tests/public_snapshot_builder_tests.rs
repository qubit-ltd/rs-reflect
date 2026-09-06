// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

//! Public integration tests for explicit reflection registry snapshots.

use std::any::TypeId;
use std::sync::LazyLock;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use qubit_reflect::__private::codegen_v2::descriptor::opaque_root_with_capabilities;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::capability::CapabilityConflictKind;
use qubit_reflect::capability::CapabilityDescriptor;
use qubit_reflect::capability::CapabilityKey;
use qubit_reflect::capability::TypeCapabilities;
use qubit_reflect::capability::TypeCapabilitiesResult;
use qubit_reflect::descriptor::ImplDefinitionDescriptor;
use qubit_reflect::descriptor::ImplDescriptor;
use qubit_reflect::descriptor::ImplKind;
use qubit_reflect::descriptor::TraitCompleteness;
use qubit_reflect::descriptor::TraitDefinitionDescriptor;
use qubit_reflect::descriptor::TraitId;
use qubit_reflect::descriptor::TypeDefinitionData;
use qubit_reflect::descriptor::TypeDefinitionDescriptor;
use qubit_reflect::descriptor::TypeDefinitionId;
use qubit_reflect::error::RegistryErrorKind;
use qubit_reflect::expression::GenericDefinitionDescriptor;
use qubit_reflect::expression::TypeExpression;
use qubit_reflect::identity::CapabilityId;
use qubit_reflect::identity::FragmentIdentity;
use qubit_reflect::register_reflected_type;
use qubit_reflect::registry::CapabilityTarget;
use qubit_reflect::registry::ReflectRegistry;
use qubit_reflect::registry::RegistrySnapshotBuilder;

register_reflected_type!(u32);
register_reflected_type!(u32);

struct DefinitionMarker;
struct TraitMarker;
struct ProviderTarget;

static EMPTY_GENERICS: LazyLock<GenericDefinitionDescriptor> =
    LazyLock::new(|| GenericDefinitionDescriptor::new([], []));
static DEFINITION: LazyLock<TypeDefinitionDescriptor> = LazyLock::new(|| {
    TypeDefinitionDescriptor::enum_type(
        TypeDefinitionId::of::<DefinitionMarker>(),
        "snapshot::GenericChoice",
        "GenericChoice",
        &EMPTY_GENERICS,
        &[],
    )
});
static TRAIT_DEFINITION: LazyLock<TraitDefinitionDescriptor> = LazyLock::new(|| {
    TraitDefinitionDescriptor::new(
        TraitId::Reflected(TypeId::of::<TraitMarker>()),
        "SnapshotTrait",
        "snapshot::SnapshotTrait",
        "SnapshotTrait",
        TraitCompleteness::Complete,
        &EMPTY_GENERICS,
    )
});
static TRAIT_IMPL_DEFINITION: LazyLock<ImplDefinitionDescriptor> = LazyLock::new(|| {
    ImplDefinitionDescriptor::new_unresolved_trait(
        source(300, "impl-definition", 300),
        TypeExpression::Parameter("T".into()),
        "snapshot::SnapshotTrait",
        Some(TRAIT_DEFINITION.trait_id().clone()),
        &EMPTY_GENERICS,
    )
});
static INHERENT_IMPL_DEFINITION: LazyLock<ImplDefinitionDescriptor> = LazyLock::new(|| {
    ImplDefinitionDescriptor::new(
        source(400, "impl", 400),
        TypeExpression::Parameter("Self".into()),
        ImplKind::Inherent,
        None,
        &EMPTY_GENERICS,
    )
    .expect("valid inherent impl definition")
});
static INHERENT_IMPL: LazyLock<ImplDescriptor> = LazyLock::new(|| {
    ImplDescriptor::builder(&INHERENT_IMPL_DEFINITION, u64_descriptor)
        .build()
        .expect("valid concrete inherent impl")
});

static PROVIDER_CALLS: AtomicUsize = AtomicUsize::new(0);
static PROVIDER_CAPABILITIES: LazyLock<TypeCapabilities> = LazyLock::new(TypeCapabilities::default);
static PROVIDER_DESCRIPTOR: TypeDescriptor =
    opaque_root_with_capabilities::<ProviderTarget>("ProviderTarget", intrinsic_capabilities);

fn intrinsic_capabilities() -> TypeCapabilitiesResult {
    PROVIDER_CALLS.fetch_add(1, Ordering::SeqCst);
    Ok(&PROVIDER_CAPABILITIES)
}

fn u64_descriptor() -> &'static TypeDescriptor {
    TypeDescriptor::of::<u64>()
}

fn source(line: u32, kind: &str, fingerprint: u64) -> FragmentIdentity {
    FragmentIdentity::new("snapshot-test", "tests", line, 1, kind, fingerprint)
}

fn key<A: 'static>(id: &'static str) -> CapabilityKey<A> {
    CapabilityKey::new(CapabilityId::new(id).expect("valid fixture capability ID"))
}

#[test]
fn test_empty_snapshots_do_not_import_global_inventory() {
    let first = RegistrySnapshotBuilder::new().build().expect("empty snapshot");
    let second = RegistrySnapshotBuilder::default()
        .build()
        .expect("default empty snapshot");

    assert!(first.types().is_empty());
    assert!(first.definitions().is_empty());
    assert!(first.impl_definitions().is_empty());
    assert!(second.types().is_empty());
}

#[test]
fn test_capability_only_snapshots_do_not_register_or_share_members() {
    let descriptor = TypeDescriptor::of::<u32>();
    let capability_key = key("example.snapshot");
    let mut first = RegistrySnapshotBuilder::new();
    first.add_type_capabilities(
        descriptor,
        vec![CapabilityDescriptor::with_adapter(capability_key, 7_u32)],
        source(1, "capability", 1),
    );
    let first = first.build().expect("first snapshot");
    let mut second = RegistrySnapshotBuilder::new();
    second.add_type_capabilities(
        descriptor,
        vec![CapabilityDescriptor::with_adapter(capability_key, 9_u32)],
        source(2, "capability", 2),
    );
    let second = second.build().expect("second snapshot");

    assert_eq!(first.capability(descriptor, capability_key).unwrap(), Some(&7));
    assert_eq!(second.capability(descriptor, capability_key).unwrap(), Some(&9));
    assert!(first.types().is_empty());
    assert!(second.get(descriptor.type_id()).is_none());
}

#[test]
fn test_add_only_collects_payloads_until_build() {
    let calls_before = PROVIDER_CALLS.load(Ordering::SeqCst);
    let mut builder = RegistrySnapshotBuilder::new();
    builder.add_type_capabilities(&PROVIDER_DESCRIPTOR, Vec::new(), source(10, "capability", 10));
    assert_eq!(PROVIDER_CALLS.load(Ordering::SeqCst), calls_before);

    let registry = builder.build().expect("valid capability-only snapshot");
    assert_eq!(PROVIDER_CALLS.load(Ordering::SeqCst), calls_before + 1);
    assert!(registry.types().is_empty());
}

#[test]
fn test_type_membership_and_duplicate_type_validation_are_explicit() {
    let descriptor = TypeDescriptor::of::<u16>();
    let mut builder = RegistrySnapshotBuilder::new();
    builder.add_type(descriptor, source(20, "type", 20));
    let registry = builder.build().expect("one type snapshot");
    assert_eq!(registry.types().len(), 1);
    assert!(std::ptr::eq(registry.types()[0], descriptor));

    let mut duplicate = RegistrySnapshotBuilder::new();
    duplicate
        .add_type(descriptor, source(21, "type", 21))
        .add_type(descriptor, source(22, "type", 22));
    assert_eq!(
        duplicate.build().expect_err("duplicate TypeId must fail").kind(),
        RegistryErrorKind::IdentityConflict,
    );
}

#[test]
fn test_duplicate_and_changed_source_identities_use_registry_validation() {
    let exact = source(30, "type", 30);
    let mut duplicate = RegistrySnapshotBuilder::new();
    duplicate
        .add_type(TypeDescriptor::of::<u8>(), exact.clone())
        .add_type(TypeDescriptor::of::<u16>(), exact);
    assert_eq!(
        duplicate.build().expect_err("duplicate source must fail").kind(),
        RegistryErrorKind::DuplicateFragment,
    );

    let mut changed = RegistrySnapshotBuilder::new();
    changed
        .add_type(TypeDescriptor::of::<u8>(), source(31, "type", 1))
        .add_type(TypeDescriptor::of::<u16>(), source(31, "type", 2));
    assert_eq!(
        changed.build().expect_err("changed source must fail").kind(),
        RegistryErrorKind::IdentityConflict,
    );
}

#[test]
fn test_capability_conflicts_preserve_duplicate_and_adapter_mismatch_details() {
    let target = TypeDescriptor::of::<u32>();
    let duplicate_key = key("example.snapshot.duplicate");
    let mut duplicate = RegistrySnapshotBuilder::new();
    duplicate
        .add_type_capabilities(
            target,
            vec![CapabilityDescriptor::with_adapter(duplicate_key, 1_u32)],
            source(40, "capability", 40),
        )
        .add_type_capabilities(
            target,
            vec![CapabilityDescriptor::with_adapter(duplicate_key, 2_u32)],
            source(41, "capability", 41),
        );
    let duplicate = duplicate.build().expect_err("duplicate capability must fail");
    assert_eq!(duplicate.kind(), RegistryErrorKind::CapabilityConflict);
    assert_eq!(
        duplicate.capability_details().map(|detail| detail.kind()),
        Some(CapabilityConflictKind::DuplicateId),
    );

    let mismatch_id = "example.snapshot.mismatch";
    let mut mismatch = RegistrySnapshotBuilder::new();
    mismatch
        .add_type_capabilities(
            target,
            vec![CapabilityDescriptor::with_adapter(key(mismatch_id), 1_u32)],
            source(42, "capability", 42),
        )
        .add_type_capabilities(
            target,
            vec![CapabilityDescriptor::with_adapter(key(mismatch_id), 2_u64)],
            source(43, "capability", 43),
        );
    let mismatch = mismatch.build().expect_err("adapter mismatch must fail");
    assert_eq!(
        mismatch.capability_details().map(|detail| detail.kind()),
        Some(CapabilityConflictKind::AdapterTypeMismatch),
    );
    assert_eq!(
        mismatch.capability_target(),
        Some(CapabilityTarget::Type(target.type_id())),
    );
}

#[test]
fn test_definition_membership_is_separate_from_definition_capabilities() {
    let mut definition_only = RegistrySnapshotBuilder::new();
    definition_only.add_definition(&DEFINITION, source(50, "type-definition", 50));
    let definition_only = definition_only.build().expect("definition-only snapshot");
    assert_eq!(definition_only.definitions().len(), 1);
    assert!(std::ptr::eq(definition_only.definitions()[0], &*DEFINITION));
    assert!(matches!(DEFINITION.data(), TypeDefinitionData::Enum { .. }));

    let capability_key = key("example.snapshot.definition");
    let mut capability_only = RegistrySnapshotBuilder::new();
    capability_only.add_definition_capabilities(
        &DEFINITION,
        vec![CapabilityDescriptor::with_adapter(capability_key, 17_u32)],
        source(51, "definition-capability", 51),
    );
    let capability_only = capability_only.build().expect("definition capability snapshot");
    assert!(capability_only.definitions().is_empty());
    assert_eq!(
        capability_only.definition_capability(DEFINITION.id(), capability_key),
        Some(&17),
    );
}

#[test]
fn test_trait_and_impl_definition_link_only_with_an_explicit_trait_member() {
    let mut complete = RegistrySnapshotBuilder::new();
    complete
        .add_trait(&TRAIT_DEFINITION, source(60, "trait", 60))
        .add_impl_definition(
            &TRAIT_IMPL_DEFINITION,
            TRAIT_IMPL_DEFINITION.fragment_identity().clone(),
        );
    let complete = complete.build().expect("trait link must resolve");
    assert!(std::ptr::eq(
        complete
            .impl_definition_trait(&TRAIT_IMPL_DEFINITION)
            .expect("linked trait"),
        &*TRAIT_DEFINITION,
    ));

    let mut missing = RegistrySnapshotBuilder::new();
    missing.add_impl_definition(
        &TRAIT_IMPL_DEFINITION,
        TRAIT_IMPL_DEFINITION.fragment_identity().clone(),
    );
    assert_eq!(
        missing.build().expect_err("missing trait must fail").kind(),
        RegistryErrorKind::ImplTraitResolution,
    );
}

#[test]
fn test_impl_and_unregistered_monomorph_queries_do_not_add_type_members() {
    let mut builder = RegistrySnapshotBuilder::new();
    builder.add_impl(&INHERENT_IMPL, INHERENT_IMPL_DEFINITION.fragment_identity().clone());
    let registry = builder.build().expect("impl-only snapshot");

    assert!(registry.types().is_empty());
    assert_eq!(registry.implementations(TypeId::of::<u64>()).len(), 1);
    assert_eq!(registry.effective_view(TypeId::of::<u64>()).implementations().len(), 1);
    assert!(registry.get(TypeId::of::<u64>()).is_none());
}

#[test]
fn test_failed_global_initialization_does_not_poison_explicit_builder() {
    assert!(ReflectRegistry::initialize().is_err());

    let descriptor = TypeDescriptor::of::<u128>();
    let mut builder = RegistrySnapshotBuilder::new();
    builder.add_type(descriptor, source(70, "type", 70));
    let registry = builder.build().expect("explicit snapshot remains independent");
    assert_eq!(registry.types().len(), 1);
    assert!(std::ptr::eq(registry.types()[0], descriptor));
}
