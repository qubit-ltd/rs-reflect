// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Crate-internal coverage for empty registry lookup views.

use std::any::TypeId;

use crate::TypeDescriptor;
use crate::descriptor::ImplDefinitionDescriptor;
use crate::descriptor::TraitId;
use crate::descriptor::TypeDefinitionId;
use crate::expression::TypeExpression;
use crate::identity::ExternalTraitId;
use crate::registry::RegistrySnapshotBuilder;

/// Verifies empty snapshots preserve the documented absence and ambiguity
/// states across every public registry view.
#[test]
fn test_empty_snapshot_exercises_public_lookup_views() {
    let registry = RegistrySnapshotBuilder::new().build().expect("empty snapshot is valid");
    let descriptor = TypeDescriptor::of::<u8>();

    assert!(registry.get(TypeId::of::<u8>()).is_none());
    assert!(registry.find_by_type_name("u8").is_empty());
    assert!(registry.find_by_query_name("u8").is_empty());
    assert!(registry.types().is_empty());
    assert!(registry.definitions().is_empty());
    assert!(registry.definition(TypeDefinitionId::of::<u8>()).is_none());
    assert!(registry.find_definitions_by_rust_path("u8").only().is_none());
    assert!(registry.find_definitions_by_query_name("u8").only().is_none());
    assert!(registry.definition_source(TypeDefinitionId::of::<u8>()).is_none());
    assert_eq!(registry.definitions_with_identity().count(), 0);
    assert_eq!(registry.types_with_identity().count(), 0);
    assert!(registry.type_source(TypeId::of::<u8>()).is_none());

    assert!(
        !registry
            .capabilities(descriptor)
            .expect("intrinsic u8 capabilities are valid")
            .descriptors()
            .is_empty()
    );
    assert!(
        registry
            .capability_by_id(descriptor, "missing")
            .expect("missing capability is valid")
            .is_none()
    );
    assert!(
        registry
            .capability_origin(descriptor, "missing")
            .expect("missing origin is valid")
            .is_none()
    );
    assert!(registry.capability_source(descriptor, "missing").is_none());
    assert!(
        registry
            .definition_capabilities(TypeDefinitionId::of::<u8>())
            .descriptors()
            .is_empty()
    );
    assert!(
        registry
            .definition_capability_by_id(TypeDefinitionId::of::<u8>(), "missing")
            .is_none()
    );
    assert_eq!(
        registry.types_with_capability(crate::capability::clone_key()).count(),
        0
    );
    assert_eq!(
        registry
            .definitions_with_capability(crate::capability::clone_key())
            .count(),
        0
    );

    assert!(registry.implementations(TypeId::of::<u8>()).is_empty());
    assert!(registry.impl_definitions().is_empty());
    assert!(
        registry
            .find_impl_definitions_by_target(&TypeExpression::Parameter("T".into()))
            .is_empty()
    );
    assert!(registry.effective_view(TypeId::of::<u8>()).implementations().is_empty());
    assert!(
        registry
            .impl_definition_trait(&ImplDefinitionDescriptor::new_unresolved_trait(
                crate::identity::FragmentIdentity::new("test", "registry", 1, 1, "impl", 1),
                TypeExpression::Parameter("T".into()),
                "Trait",
                None,
                Box::leak(Box::new(crate::expression::GenericDefinitionDescriptor::new([], []))),
            ))
            .is_none()
    );
    assert!(
        registry
            .trait_definition(&TraitId::External(
                ExternalTraitId::new("test.missing").expect("valid external trait ID"),
            ))
            .is_none()
    );
    assert!(registry.trait_definition_by_path("missing").is_none());
    assert!(registry.find_trait_definitions_by_path("missing").only().is_none());
}
