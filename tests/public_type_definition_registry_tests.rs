// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

//! Public integration tests for first-class generic type declarations.

#![cfg(feature = "derive")]

use qubit_reflect::__private::codegen_v2::inventory;
use qubit_reflect::__private::codegen_v2::registration::CapabilityRegistration;
use qubit_reflect::__private::codegen_v2::registration::CapabilityTarget;
use qubit_reflect::__private::codegen_v2::registration::FragmentKind;
use qubit_reflect::__private::codegen_v2::registration::FragmentPayload;
use qubit_reflect::__private::codegen_v2::registration::RegistrationFragment;
use qubit_reflect::__private::codegen_v2::registration::RuntimeIdentity;
use qubit_reflect::__private::codegen_v2::registration::StaticFragmentIdentity;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectedOwned;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::capability::CapabilityDescriptor;
use qubit_reflect::capability::CapabilityKey;
use qubit_reflect::capability::clone_key;
use qubit_reflect::capability::default_key;
use qubit_reflect::descriptor::StructKind;
use qubit_reflect::descriptor::TypeDefinitionData;
use qubit_reflect::descriptor::TypeDefinitionDescriptor;
use qubit_reflect::descriptor::TypeDefinitionId;
use qubit_reflect::descriptor::VariantKind;
use qubit_reflect::expression::GenericDefinitionDescriptor;
use qubit_reflect::identity::CapabilityId;
use qubit_reflect::identity::VisibilityKind;
use qubit_reflect::registry::ReflectRegistry;

#[derive(Clone, Reflect)]
#[reflect(crate = qubit_reflect, capabilities(Clone))]
struct GenericRecord<T: Clone> {
    value: T,
}

#[derive(Reflect)]
#[reflect(crate = qubit_reflect)]
enum GenericEnum<T> {
    Unit,
    Tuple(T),
    Struct { value: T },
}

/// Returns the typed declaration capability used by this fixture.
fn definition_key() -> CapabilityKey<u32> {
    CapabilityKey::new(CapabilityId::new("example.generic_definition").expect("valid capability ID"))
}

/// Declares one capability fragment targeting the generic definition itself.
mod definition_capability_registration {
    use super::__qubit_reflect_type_definition_GenericRecord;
    use super::CapabilityDescriptor;
    use super::CapabilityRegistration;
    use super::CapabilityTarget;
    use super::FragmentKind;
    use super::FragmentPayload;
    use super::RegistrationFragment;
    use super::RuntimeIdentity;
    use super::StaticFragmentIdentity;
    use super::definition_key;
    use super::inventory;

    /// Returns the exact definition capability target.
    fn runtime_identity() -> RuntimeIdentity {
        RuntimeIdentity::Capabilities(CapabilityTarget::TypeDefinition(
            __qubit_reflect_type_definition_GenericRecord().id(),
        ))
    }

    /// Builds the typed definition capability payload.
    fn payload() -> FragmentPayload {
        FragmentPayload::Capability(CapabilityRegistration::for_definition(
            __qubit_reflect_type_definition_GenericRecord(),
            vec![CapabilityDescriptor::with_adapter(definition_key(), 7_u32)],
        ))
    }

    inventory::submit! {
        RegistrationFragment::new(
            FragmentKind::Capability,
            StaticFragmentIdentity::new(
                env!("CARGO_PKG_NAME"),
                module_path!(),
                line!(),
                column!(),
                "definition-capability",
                1,
            ),
            runtime_identity,
            payload,
        )
    }
}

/// Verifies a generic declaration is registered without registering a
/// concrete monomorph as a root.
#[test]
fn test_registry_registers_generic_type_definition() {
    let registry = ReflectRegistry::initialize().expect("generic definitions must register");
    let concrete = TypeDescriptor::of::<GenericRecord<u8>>();
    let definition = concrete.type_definition().expect("generic source definition");

    assert!(std::ptr::eq(
        registry.definition(definition.id()).expect("registered definition"),
        definition,
    ));
    assert_eq!(definition.query_name(), "GenericRecord");
    assert!(definition.rust_path().ends_with("::GenericRecord"));
    assert_eq!(definition.generics().parameters().len(), 1);
    assert_eq!(definition.fields().expect("struct fields").len(), 1);
    assert!(registry.definition_source(definition.id()).is_some());
    assert_eq!(
        registry.definitions_with_identity().count(),
        registry.definitions().len()
    );
    assert_eq!(
        registry.definition_capability(definition.id(), definition_key()),
        Some(&7_u32),
    );
    assert!(
        registry
            .definition_capability_by_id(definition.id(), "example.generic_definition")
            .is_some()
    );
    assert!(
        registry
            .definitions_with_capability(definition_key())
            .any(|candidate| candidate.id() == definition.id())
    );
    assert!(
        registry
            .find_definitions_by_query_name("GenericRecord")
            .iter()
            .any(|candidate| candidate.id() == definition.id())
    );
    assert!(registry.get(concrete.type_id()).is_none());
    assert!(registry.capability(concrete, clone_key()).is_some());
}

/// Verifies concrete monomorphs share one declaration identity while retaining
/// distinct runtime type identities.
#[test]
fn test_concrete_monomorphs_share_definition_identity() {
    let left = TypeDescriptor::of::<GenericRecord<u8>>();
    let right = TypeDescriptor::of::<GenericRecord<String>>();

    assert_eq!(left.definition_id(), right.definition_id());
    assert_ne!(left.type_id(), right.type_id());
    assert_ne!(
        left.generic_arguments()
            .and_then(|arguments| arguments.type_argument(0))
            .expect("left concrete argument")
            .type_id(),
        right
            .generic_arguments()
            .and_then(|arguments| arguments.type_argument(0))
            .expect("right concrete argument")
            .type_id(),
    );
}

/// Verifies enum declaration accessors expose only source-level structure.
#[test]
fn test_generic_enum_definition_exposes_source_structure() {
    let _values = [
        GenericEnum::Unit,
        GenericEnum::Tuple(1_u8),
        GenericEnum::Struct { value: 2_u8 },
    ];
    let descriptor = TypeDescriptor::of::<GenericEnum<u8>>();
    let definition = descriptor.type_definition().expect("generic enum definition");
    let variants = definition.variants().expect("enum variants");

    assert!(matches!(definition.data(), TypeDefinitionData::Enum { .. }));
    assert_eq!(
        definition.id().marker_type_id(),
        descriptor
            .definition_id()
            .expect("generic definition ID")
            .marker_type_id()
    );
    assert_eq!(variants.len(), 3);
    assert_eq!(variants[0].index(), 0);
    assert_eq!(variants[0].rust_name(), "Unit");
    assert_eq!(variants[0].query_name(), "Unit");
    assert_eq!(variants[0].kind(), VariantKind::Unit);
    assert!(variants[0].fields().is_empty());

    let tuple_field = &variants[1].fields()[0];
    assert_eq!(tuple_field.index(), 0);
    assert_eq!(tuple_field.rust_name(), None);
    assert_eq!(tuple_field.query_name(), None);
    let _declared_type = tuple_field.ty();
    assert_eq!(tuple_field.visibility().kind(), VisibilityKind::Private);

    let registry = ReflectRegistry::initialize().expect("generic enum must register");
    let candidates = registry.find_definitions_by_rust_path(definition.rust_path());
    assert_eq!(candidates.len(), 1);
    assert!(!candidates.is_empty());
    assert_eq!(candidates.iter().count(), 1);
    assert!(std::ptr::eq(
        candidates.only().expect("sole definition candidate"),
        definition,
    ));
    let missing = TypeDefinitionId::of::<MissingDefinition>();
    assert!(registry.definition(missing).is_none());
    assert!(registry.definition_capabilities(missing).descriptors().is_empty());
    assert!(registry.definition_capability(missing, definition_key()).is_none());
    assert!(
        registry
            .definition_capability_by_id(missing, "example.generic_definition")
            .is_none()
    );
}

struct MissingDefinition;

/// Verifies all declaration constructors preserve their kind-specific shape.
#[test]
fn test_type_definition_constructors_preserve_declared_shape() {
    let generics = Box::leak(Box::new(GenericDefinitionDescriptor::new([], [])));
    let opaque = TypeDefinitionDescriptor::opaque(
        TypeDefinitionId::of::<OpaqueMarker>(),
        "example::Opaque",
        "Opaque",
        generics,
    );
    let structure = TypeDefinitionDescriptor::struct_type(
        TypeDefinitionId::of::<StructMarker>(),
        "example::Structure",
        "Structure",
        generics,
        StructKind::Unit,
        &[],
    );
    let enumeration = TypeDefinitionDescriptor::enum_type(
        TypeDefinitionId::of::<EnumMarker>(),
        "example::Enumeration",
        "Enumeration",
        generics,
        &[],
    );

    assert!(matches!(opaque.data(), TypeDefinitionData::Opaque));
    assert!(opaque.fields().is_none());
    assert!(opaque.variants().is_none());
    assert!(structure.fields().expect("struct fields").is_empty());
    assert!(structure.variants().is_none());
    assert!(enumeration.variants().expect("enum variants").is_empty());
    assert!(enumeration.fields().is_none());
}

struct OpaqueMarker;
struct StructMarker;
struct EnumMarker;

#[derive(Clone, Default, Reflect)]
#[reflect(crate = qubit_reflect, capabilities(Clone, Default, concrete_provider))]
struct CapabilityRecord<T: Clone + Default> {
    value: T,
}

#[derive(Clone, Reflect)]
#[reflect(crate = qubit_reflect, capabilities(Clone, concrete_provider))]
enum CapabilityEnum<T: Clone> {
    Value(T),
}

/// Returns the exact concrete identity through a custom typed adapter.
fn concrete_provider<T: 'static>() -> CapabilityDescriptor {
    CapabilityDescriptor::with_adapter(concrete_key(), std::any::TypeId::of::<T> as fn() -> std::any::TypeId)
}

/// Identifies the custom provider contract shared by all concrete instances.
fn concrete_key() -> CapabilityKey<fn() -> std::any::TypeId> {
    CapabilityKey::new(CapabilityId::new("example.concrete_identity").expect("valid capability ID"))
}

/// Executes adapters instead of merely checking that capability IDs exist.
fn assert_concrete_capabilities<T: Reflect + Clone>(value: T, registry: &ReflectRegistry) {
    let descriptor = TypeDescriptor::of::<T>();
    let owned = ReflectedOwned::new(value);
    let cloned = registry
        .capability(descriptor, clone_key())
        .expect("clone adapter")
        .clone_owned(&owned)
        .expect("clone must accept its exact monomorph");
    assert!(cloned.downcast_ref::<T>().is_some());
    assert_eq!(
        registry
            .capability(descriptor, concrete_key())
            .expect("custom provider")(),
        std::any::TypeId::of::<T>()
    );
    if let Some(default) = registry.capability(descriptor, default_key()) {
        assert!(
            default.create().downcast_ref::<T>().is_some(),
            "default must create its exact monomorph"
        );
    }
}

/// Different generic arguments must never share concrete capability adapters.
#[test]
fn test_generic_capabilities_preserve_each_monomorph() {
    let registry = ReflectRegistry::initialize().expect("valid registry");
    assert_concrete_capabilities(CapabilityRecord { value: 7_u32 }, registry);
    assert_concrete_capabilities(
        CapabilityRecord {
            value: String::from("second"),
        },
        registry,
    );
    assert_concrete_capabilities(CapabilityEnum::Value(String::from("first")), registry);
    assert_concrete_capabilities(CapabilityEnum::Value(9_u32), registry);
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                assert_concrete_capabilities(CapabilityRecord { value: 3_u64 }, registry);
                assert_concrete_capabilities(CapabilityRecord { value: false }, registry);
            });
        }
    });
}

#[derive(Reflect)]
#[reflect(crate = qubit_reflect, definition_provider_v2 = domain_definition)]
struct DomainDefinition<T> {
    value: T,
}

/// The facade chooses the provider name through the versioned contract.
#[test]
fn test_explicit_definition_provider_needs_no_monomorph() {
    let registry = ReflectRegistry::initialize().expect("valid definitions");
    let definition = domain_definition();
    assert!(std::ptr::eq(
        registry.definition(definition.id()).expect("registered definition"),
        definition
    ));
    assert_eq!(definition.query_name(), "DomainDefinition");
    assert!(std::ptr::eq(
        TypeDescriptor::of::<DomainDefinition<u8>>()
            .type_definition()
            .expect("concrete link"),
        definition
    ));
}
