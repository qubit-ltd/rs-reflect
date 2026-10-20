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
use qubit_reflect::reflect_external_trait;

#[Reflect]
struct ContextBoundaryGeneric<T> {
    value: T,
}

#[Reflect]
struct ContextBoundaryTraitTargetA;

#[Reflect]
struct ContextBoundaryTraitTargetB;

#[Reflect]
trait ContextBoundaryTrait {
    fn label(&self) -> &'static str;
}

#[ReflectImpl(reflected_trait = ContextBoundaryTrait)]
impl ContextBoundaryTrait for ContextBoundaryTraitTargetA {
    fn label(&self) -> &'static str {
        "a"
    }
}

#[ReflectImpl(reflected_trait = ContextBoundaryTrait)]
impl ContextBoundaryTrait for ContextBoundaryTraitTargetB {
    fn label(&self) -> &'static str {
        "b"
    }
}

#[allow(dead_code)]
trait ContextBoundaryExternalTrait {}

reflect_external_trait! {
    ContextBoundaryExternalTraitHandle = ContextBoundaryExternalTrait;
}

#[Reflect]
struct ContextBoundaryExternalTarget;

#[ReflectImpl(external_trait = ContextBoundaryExternalTraitHandle)]
impl ContextBoundaryExternalTrait for ContextBoundaryExternalTarget {}

#[test]
fn generic_definition_queries_return_only_registered_concrete_instances() {
    let context = ReflectionContext::builder()
        .include_type::<ContextBoundaryGeneric<u8>>()
        .build()
        .expect("the concrete generic type is a valid context registration");
    let concrete = context.type_of::<ContextBoundaryGeneric<u8>>();
    let definition = concrete.definition().expect("generic type has a source definition");

    let by_id = context
        .type_definition_by_id(definition.id())
        .expect("the definition identity is registered in this context");
    assert_eq!(by_id.path(), definition.path());

    let instances = by_id.instances().expect("registered concrete instances are queryable");
    let instance_ids = instances.iter().map(|instance| instance.type_id()).collect::<Vec<_>>();
    assert_eq!(instance_ids.len(), 1);
    assert!(instance_ids.contains(&std::any::TypeId::of::<ContextBoundaryGeneric<u8>>()));

    let short_name_matches = context
        .type_definitions_by_short_name("ContextBoundaryGeneric")
        .expect("short-name lookup returns source definitions");
    assert_eq!(short_name_matches.len(), 1);
    assert_eq!(short_name_matches.iter().next().unwrap().id(), definition.id());
}

#[test]
fn reflected_trait_queries_link_applications_by_definition_and_path() {
    let context = ReflectionContext::builder()
        .include_linked()
        .build()
        .expect("linked reflected trait declarations and applications are consistent");
    let definitions = context
        .trait_definitions()
        .expect("registered source trait declarations are enumerable");
    let definition = definitions
        .iter()
        .find(|definition| definition.name() == "ContextBoundaryTrait")
        .expect("the reflected declaration is registered");
    let by_path = context
        .trait_definition_by_name(definition.path())
        .expect("the declaration path identifies one reflected trait");
    assert_eq!(by_path.id(), definition.id());

    let applications = definition
        .applications()
        .expect("applications indexed by opaque trait identity are queryable");
    assert_eq!(applications.len(), 2);
    let by_path = context
        .trait_applications_by_path("ContextBoundaryTrait")
        .expect("applications retain their original source path");
    assert_eq!(
        by_path.len(),
        2,
        "impl source expressions use their retained local path spelling"
    );
    assert!(
        context
            .trait_applications()
            .expect("all applications are enumerable")
            .len()
            >= 2
    );
}

#[test]
fn external_trait_lookup_preserves_incomplete_metadata_without_inventing_a_definition() {
    let context = ReflectionContext::builder()
        .include_linked()
        .build()
        .expect("a typed external handle can register incomplete trait metadata");
    let application_path = "ContextBoundaryExternalTrait";
    let applications = context
        .trait_applications_by_path(application_path)
        .expect("external applications remain queryable by retained source path");
    assert_eq!(applications.len(), 1);
    assert!(applications[0].find_definition().is_some());
    assert_eq!(
        applications[0].completeness(),
        qubit_reflect::descriptor::TraitCompleteness::ExternalIncomplete,
    );

    let external_definition = context
        .external_trait_definition(application_path)
        .expect("external trait definition lookup succeeds")
        .expect("the typed external handle registered its incomplete identity");
    assert_eq!(external_definition.path(), application_path);
    assert_eq!(
        external_definition.completeness(),
        qubit_reflect::descriptor::TraitCompleteness::ExternalIncomplete,
    );
    assert!(
        context
            .external_trait_definition("not::registered::trait")
            .unwrap()
            .is_none()
    );
}
