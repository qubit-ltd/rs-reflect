// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;
use std::hash::Hash;
use std::hash::Hasher;

use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::InvocationAdapter;
use qubit_reflect::descriptor::TraitId;
use qubit_reflect::error::RegistryErrorKind;
use qubit_reflect::expression::ConcreteTypeExpression;
use qubit_reflect::expression::TypeExpression;
use qubit_reflect::identity::FragmentIdentity;
use qubit_reflect::registry::MethodBinding;
use qubit_reflect::registry::RegistrySnapshotBuilder;

fn opaque_entry() {}

static ADAPTER: InvocationAdapter = InvocationAdapter::new(opaque_entry);

#[test]
fn equal_method_binding_selectors_have_equal_hashes() {
    let first = MethodBinding::inherent(TypeDescriptor::of::<u8>(), "read", [], &ADAPTER);
    let duplicate = MethodBinding::inherent(TypeDescriptor::of::<u8>(), "read", [], &ADAPTER);

    assert_eq!(first, duplicate);
    let mut first_hash = std::collections::hash_map::DefaultHasher::new();
    first.hash(&mut first_hash);
    let mut duplicate_hash = std::collections::hash_map::DefaultHasher::new();
    duplicate.hash(&mut duplicate_hash);
    assert_eq!(first_hash.finish(), duplicate_hash.finish());
}

#[test]
fn binding_identity_keeps_concrete_target_and_trait_application() {
    let u8_binding = MethodBinding::inherent(TypeDescriptor::of::<u8>(), "read", [], &ADAPTER);
    let u16_binding = MethodBinding::inherent(TypeDescriptor::of::<u16>(), "read", [], &ADAPTER);
    assert_ne!(u8_binding.identity(), u16_binding.identity());
    assert_eq!(u8_binding.identity().target_type_id(), TypeId::of::<u8>());

    let trait_id = TraitId::External(
        qubit_reflect::identity::ExternalTraitId::new("example.read").expect("valid external trait identifier"),
    );
    let trait_u8 = TypeExpression::Concrete(
        ConcreteTypeExpression::new(
            ["example", "Read"],
            [qubit_reflect::expression::GenericArgument::Type(
                TypeExpression::Concrete(ConcreteTypeExpression::new(["u8"], []).expect("valid type path")),
            )],
        )
        .expect("valid trait application"),
    );
    let trait_u16 = TypeExpression::Concrete(
        ConcreteTypeExpression::new(
            ["example", "Read"],
            [qubit_reflect::expression::GenericArgument::Type(
                TypeExpression::Concrete(ConcreteTypeExpression::new(["u16"], []).expect("valid type path")),
            )],
        )
        .expect("valid trait application"),
    );
    let u8_trait_binding = MethodBinding::trait_method(
        TypeDescriptor::of::<u8>(),
        trait_id.clone(),
        trait_u8,
        "read",
        [],
        &ADAPTER,
    );
    let u16_trait_binding =
        MethodBinding::trait_method(TypeDescriptor::of::<u8>(), trait_id, trait_u16, "read", [], &ADAPTER);
    assert_ne!(u8_trait_binding.identity(), u16_trait_binding.identity());
}

#[test]
fn binding_without_registered_definition_fails_with_structured_error() {
    let mut builder = RegistrySnapshotBuilder::new();
    builder.add_method_binding(
        MethodBinding::inherent(TypeDescriptor::of::<u8>(), "missing", [], &ADAPTER),
        FragmentIdentity::new("test", module_path!(), line!(), 1, "method-binding", 1),
    );

    let error = builder
        .build()
        .expect_err("unmatched method binding must fail transactionally");
    assert_eq!(error.kind(), RegistryErrorKind::MethodBindingNotFound);
    assert_eq!(error.method_binding_name(), Some("missing"));
    assert_eq!(error.method_binding_matches(), Some(0));
}
