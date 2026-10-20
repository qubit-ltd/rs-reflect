// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use std::any::TypeId;

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::bind_method;
use qubit_reflect::context::MethodBinding;
use qubit_reflect::context::RegisteredTraitDescriptor;
use qubit_reflect::error::ItemIdentity;
use qubit_reflect::error::ItemKind;
use qubit_reflect::error::LookupCandidate;
use qubit_reflect::error::LookupQuery;
use qubit_reflect::error::LookupSubject;
use qubit_reflect::error::StableItemId;
use qubit_reflect::lookup::Lookup;

#[Reflect]
struct PublicFacadeFixture {
    value: u32,
}

#[ReflectImpl]
impl PublicFacadeFixture {
    pub fn run(&self) -> u32 {
        self.value
    }
}

#[Reflect]
struct PublicBindingFixture;

impl PublicBindingFixture {
    fn run(&self) -> u32 {
        7
    }
}

#[test]
fn public_facade_supports_named_type_and_member_queries() {
    let context = ReflectionContext::global();
    let type_name = std::any::type_name::<PublicFacadeFixture>();

    let descriptor = context
        .type_by_name(type_name)
        .expect("the unique fully qualified type name should resolve");
    assert_eq!(descriptor.type_id(), TypeId::of::<PublicFacadeFixture>());
    assert!(std::ptr::eq(descriptor.context(), context));
    assert_eq!(descriptor.field("value").expect("field exists").name(), "value");
    assert_eq!(descriptor.method("run").expect("method exists").name(), "run");

    let lookup = context
        .lookup_type_by_name(type_name)
        .expect("the lookup should complete");
    assert_eq!(lookup.len(), 1);
    let lookup_descriptor = lookup.only().expect("the type name is unique");
    assert_eq!(lookup_descriptor.type_id(), TypeId::of::<PublicFacadeFixture>());
    assert!(std::ptr::eq(lookup_descriptor.context(), context));

    let method_lookup = descriptor
        .lookup_method("run")
        .expect("the method lookup should complete");
    assert_eq!(method_lookup.only().expect("the method is unique").name(), "run");
}

#[test]
fn public_type_descriptor_is_the_context_bound_type_handle() {
    let context = ReflectionContext::global();
    let descriptor = TypeDescriptor::of_in::<PublicFacadeFixture>(context);

    assert_eq!(descriptor.type_id(), TypeId::of::<PublicFacadeFixture>());
    assert!(std::ptr::eq(descriptor.context(), context));
}

#[test]
fn public_context_facade_names_typed_bindings_and_trait_descriptors() {
    let binding: MethodBinding<PublicBindingFixture> =
        bind_method!(PublicBindingFixture::run as fn(&PublicBindingFixture) -> u32);
    let context = ReflectionContext::builder()
        .bind(binding)
        .build()
        .expect("typed binding builds an explicit context");
    assert_eq!(
        context
            .type_of::<PublicBindingFixture>()
            .method("run")
            .expect("the bound method is public")
            .name(),
        "run"
    );

    let trait_lookup: Result<Option<RegisteredTraitDescriptor<'_>>, _> =
        context.external_trait_definition("missing::Trait");
    assert!(trait_lookup.expect("trait descriptor lookup completes").is_none());
}

#[test]
fn public_lookup_errors_expose_constructible_candidate_identities() {
    let candidate = LookupCandidate {
        identity: ItemIdentity {
            kind: ItemKind::Type,
            stable_id: StableItemId("public-fixture".into()),
            display_name: "PublicFacadeFixture".into(),
        },
        display_name: "PublicFacadeFixture".into(),
        origin: None,
    };
    let lookup = Lookup::Ambiguous(vec![candidate].into_boxed_slice());
    let error = lookup
        .into_unique(LookupSubject::Type, LookupQuery::Name("PublicFacadeFixture".into()))
        .expect_err("an ambiguous lookup reports its public candidate identities");
    assert!(matches!(error, qubit_reflect::error::LookupError::Ambiguous { .. }));
}
