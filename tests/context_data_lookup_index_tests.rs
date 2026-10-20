// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;

#[Reflect]
struct DataIndexGeneric<T> {
    value: T,
}

#[Reflect]
struct DataIndexTarget {
    value: u8,
}

#[Reflect]
trait DataIndexContract {
    fn value(&self) -> u8;
}

#[ReflectImpl(reflected_trait = DataIndexContract)]
impl DataIndexContract for DataIndexTarget {
    fn value(&self) -> u8 {
        self.value
    }
}

#[ReflectImpl]
impl DataIndexTarget {
    fn doubled(&self) -> u8 {
        self.value * 2
    }
}

qubit_reflect::register_trait_object!(dyn std::fmt::Display);

#[test]
fn generic_definition_and_type_indexes_resolve_registered_instances() {
    let context = ReflectionContext::builder()
        .include_type::<DataIndexGeneric<u16>>()
        .build()
        .expect("the generic application is a valid registration");
    let concrete = context.type_of::<DataIndexGeneric<u16>>();
    let definition = concrete.definition().expect("the generic source definition exists");

    assert_eq!(
        context
            .find_type_by_id(std::any::TypeId::of::<DataIndexGeneric<u16>>())
            .expect("type ID lookup succeeds")
            .expect("the concrete type is registered")
            .type_id(),
        std::any::TypeId::of::<DataIndexGeneric<u16>>()
    );
    assert_eq!(
        context
            .type_definition_by_id(definition.id())
            .expect("opaque definition ID lookup succeeds")
            .path(),
        definition.path()
    );
    assert_eq!(
        context
            .type_definition_by_name(definition.path())
            .expect("full source path lookup succeeds")
            .id(),
        definition.id()
    );
    let instances = context
        .type_definition_by_id(definition.id())
        .expect("definition remains context-bound")
        .instances()
        .expect("registered concrete instances are indexed by definition");
    assert_eq!(instances.len(), 1);
    assert_eq!(instances.iter().next().unwrap().type_id(), concrete.type_id());
    assert!(
        context
            .type_definitions_by_short_name("DataIndexGeneric")
            .expect("short-name index lookup succeeds")
            .iter()
            .any(|candidate| candidate.id() == definition.id())
    );
}

#[test]
fn trait_indexes_and_impl_indexes_link_the_same_concrete_application() {
    let context = ReflectionContext::builder()
        .include_linked()
        .build()
        .expect("the linked trait and implementation registrations are consistent");
    let trait_definition = context
        .trait_definition_by_name(concat!(module_path!(), "::DataIndexContract"))
        .expect("the reflected trait definition is indexed by full path");
    assert!(
        context
            .trait_definitions_by_short_name("DataIndexContract")
            .expect("short trait names are indexed")
            .iter()
            .any(|candidate| candidate.id() == trait_definition.id())
    );

    let applications = trait_definition
        .applications()
        .expect("the trait definition links its concrete applications");
    assert_eq!(applications.len(), 1);
    assert_eq!(
        applications.iter().next().unwrap().definition().unwrap().id(),
        trait_definition.id()
    );
    assert_eq!(
        context
            .trait_applications_by_path("DataIndexContract")
            .expect("the retained source path is indexed")
            .len(),
        1
    );

    let target = context.type_of::<DataIndexTarget>();
    assert_eq!(
        target.impls().expect("impl applications are indexed by target").len(),
        2
    );
    assert_eq!(
        target
            .inherent_impls()
            .expect("inherent applications are selectable")
            .len(),
        1
    );
    assert_eq!(
        target.trait_impls().expect("trait applications are selectable").len(),
        1
    );
    assert_eq!(
        target
            .method("doubled")
            .expect("inherent method catalog is available")
            .name(),
        "doubled"
    );
}

#[test]
fn implementation_definition_index_returns_concrete_instances_and_trait_objects() {
    let context = ReflectionContext::builder()
        .include_linked()
        .build()
        .expect("the linked registrations build");
    let implementation = context
        .impl_definitions()
        .expect("source impl definitions can be enumerated")
        .iter()
        .find(|definition| {
            definition
                .implemented_trait()
                .is_some_and(|trait_| trait_.path().ends_with("DataIndexContract"))
        })
        .expect("the source trait impl definition is registered");
    let instances = implementation
        .instances()
        .expect("concrete impl applications are indexed by source impl identity");
    assert_eq!(instances.len(), 1);
    assert_eq!(
        instances.iter().next().unwrap().target().type_id(),
        std::any::TypeId::of::<DataIndexTarget>()
    );

    let object = context
        .trait_object::<dyn std::fmt::Display>()
        .expect("trait object lookup succeeds")
        .expect("the compiler-checked trait object is registered");
    assert_eq!(object.trait_path(), "dyn std :: fmt :: Display");
}
