// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

extern crate self as codegen_v4_trait_registration_tests;

use std::fmt::Display as DisplayAlias;
use std::marker::PhantomData;

use qubit_reflect::__private::codegen_v4::descriptor::ReflectionContext;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::reflect_external_trait;
use qubit_reflect::register_trait_object;

reflect_external_trait! {
    pub FmtDisplay = std::fmt::Display;
}

#[allow(dead_code)]
trait ExternalGenericContract<T, const LIMIT: usize> {
    fn external_value(&self) -> T;
}

reflect_external_trait! {
    pub ExternalGenericContractHandle = ExternalGenericContract;
}

mod duplicate_source_path_handle {
    qubit_reflect::reflect_external_trait! {
        pub AnotherDisplayHandle = std::fmt::Display;
    }
}

register_trait_object!(dyn std::fmt::Display);
#[derive(Clone)]
struct ExternalDisplay {
    value: u8,
}

#[Reflect]
struct GenericExternalTarget;
struct GenericExternalDefinitionTarget<T>(PhantomData<T>);

#[ReflectImpl(external_trait = ExternalGenericContractHandle)]
impl<T: 'static> ExternalGenericContract<T, 19> for GenericExternalDefinitionTarget<T> {
    fn external_value(&self) -> T {
        panic!("the generic source impl is metadata-only")
    }
}

#[ReflectImpl(external_trait = ExternalGenericContractHandle)]
impl ExternalGenericContract<u16, 17> for GenericExternalTarget {
    #[reflect(skip)]
    fn external_value(&self) -> u16 {
        17
    }
}

#[ReflectImpl(external_trait = ExternalGenericContractHandle)]
impl ExternalGenericContract<u16, 18> for GenericExternalTarget {
    #[reflect(skip)]
    fn external_value(&self) -> u16 {
        18
    }
}

impl qubit_reflect::__private::codegen_v4::descriptor::Reflect for ExternalDisplay {
    fn __reflect_registration() -> &'static qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<Self> {
        qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration::get_or_init_with(
            qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration::new,
        )
    }
}

qubit_reflect::__private::codegen_v4::registration::submit_type!(ExternalDisplay);

struct UnregisteredExternalDebug;

impl qubit_reflect::__private::codegen_v4::descriptor::Reflect for UnregisteredExternalDebug {
    fn __reflect_registration() -> &'static qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<Self> {
        qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration::get_or_init_with(
            qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration::new,
        )
    }
}

qubit_reflect::__private::codegen_v4::registration::submit_type!(UnregisteredExternalDebug);

mod first_unlinked_alias {
    pub use std::fmt::Display as LocalContract;
}

mod second_unlinked_alias {
    pub use std::fmt::Debug as LocalContract;
}

mod first_same_name_trait {
    pub trait SameName {
        fn value(&self) -> u8;
    }
}

mod second_same_name_trait {
    pub trait SameName {
        fn value(&self) -> u8;
    }
}

#[Reflect]
struct SharedUnlinkedAliasTarget;

#[Reflect]
struct FirstUnlinkedAliasTarget;

#[Reflect]
struct SecondUnlinkedAliasTarget;

#[Reflect]
struct UnregisteredExternalAliasTarget;

#[Reflect]
struct DistinctUnlinkedMethodTarget;

mod first_unlinked_impls {
    use qubit_reflect::ReflectImpl;

    use super::FirstUnlinkedAliasTarget;
    use super::SharedUnlinkedAliasTarget;
    use super::first_unlinked_alias::LocalContract;

    #[ReflectImpl]
    impl LocalContract for FirstUnlinkedAliasTarget {
        #[reflect(skip)]
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("first")
        }
    }

    #[ReflectImpl]
    impl LocalContract for SharedUnlinkedAliasTarget {
        #[reflect(skip)]
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("first")
        }
    }
}

mod second_unlinked_impls {
    use qubit_reflect::ReflectImpl;

    use super::SecondUnlinkedAliasTarget;
    use super::SharedUnlinkedAliasTarget;
    use super::second_unlinked_alias::LocalContract;

    #[ReflectImpl]
    impl LocalContract for SecondUnlinkedAliasTarget {
        #[reflect(skip)]
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("second")
        }
    }

    #[ReflectImpl]
    impl LocalContract for SharedUnlinkedAliasTarget {
        #[reflect(skip)]
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("second")
        }
    }
}

mod unregistered_external_alias_impl {
    use std::fmt::Debug as LocalDebug;

    use qubit_reflect::ReflectImpl;

    use super::UnregisteredExternalAliasTarget;

    #[ReflectImpl]
    impl LocalDebug for UnregisteredExternalAliasTarget {
        #[reflect(skip)]
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("external alias")
        }
    }
}

mod first_same_name_impl {
    use qubit_reflect::ReflectImpl;

    use super::DistinctUnlinkedMethodTarget;
    use super::first_same_name_trait::SameName;

    #[ReflectImpl]
    impl SameName for DistinctUnlinkedMethodTarget {
        fn value(&self) -> u8 {
            1
        }
    }
}

mod second_same_name_impl {
    use qubit_reflect::ReflectImpl;

    use super::DistinctUnlinkedMethodTarget;
    use super::second_same_name_trait::SameName;

    #[ReflectImpl]
    impl SameName for DistinctUnlinkedMethodTarget {
        fn value(&self) -> u8 {
            2
        }
    }
}

mod third_party {
    pub trait RemoteContract {
        fn remote_value(&self) -> u32;
    }
}

#[Reflect]
struct UnregisteredExternalTraitTarget;

#[Reflect]
trait SameSourcePathTrait {
    fn same_source_path(&self);
}

#[Reflect]
struct NakedLocalTraitTarget;

#[allow(dead_code)]
trait UnlinkedDefaultMethodTrait {
    fn unlinked_default_method(&self) -> u8 {
        42
    }
}

#[Reflect]
struct UnlinkedDefaultMethodTarget;

#[ReflectImpl]
impl third_party::RemoteContract for UnregisteredExternalTraitTarget {
    fn remote_value(&self) -> u32 {
        7
    }
}

#[ReflectImpl]
impl codegen_v4_trait_registration_tests::SameSourcePathTrait for NakedLocalTraitTarget {
    fn same_source_path(&self) {}
}

#[ReflectImpl]
impl UnlinkedDefaultMethodTrait for UnlinkedDefaultMethodTarget {}

#[ReflectImpl]
impl std::fmt::Debug for UnregisteredExternalDebug {
    #[reflect(skip)]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("external-debug")
    }
}

#[ReflectImpl(external_trait = FmtDisplay)]
impl DisplayAlias for ExternalDisplay {
    #[reflect(skip)]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("external")
    }
}

#[test]
fn test_registered_external_trait_identity_is_available_in_context() {
    assert_eq!(ExternalDisplay { value: 7 }.value, 7);
    let definition = ReflectionContext::global()
        .trait_definition_for::<FmtDisplay>()
        .expect("global context builds")
        .expect("external trait marker is linked");

    assert_eq!(definition.path(), "std::fmt::Display");
    assert_eq!(definition.name(), "Display");
    assert_eq!(
        definition.completeness(),
        qubit_reflect::__private::codegen_v4::traits::Completeness::ExternalIncomplete
    );
}

#[test]
fn test_generic_external_trait_handle_builds_in_context() {
    let context = ReflectionContext::builder()
        .include_linked()
        .build()
        .expect("generic external trait arguments do not require declaration metadata");

    let definition = context
        .trait_definition_for::<ExternalGenericContractHandle>()
        .expect("external generic trait definition lookup succeeds")
        .expect("external generic trait handle is linked");
    assert_eq!(
        definition.completeness(),
        qubit_reflect::__private::codegen_v4::traits::Completeness::ExternalIncomplete
    );

    let generic_impl = context
        .impl_definitions()
        .expect("generic impl source definitions are available")
        .iter()
        .find(|implementation| {
            implementation.generics().parameters().len() == 1
                && implementation
                    .methods()
                    .iter()
                    .any(|method| method.name() == "external_value")
        })
        .expect("the generic external impl source definition is registered");
    let implemented_trait = generic_impl
        .implemented_trait()
        .expect("the generic impl keeps its trait expression");
    let linked_definition = implemented_trait
        .find_definition()
        .expect("the explicit external handle remains linked from the source definition");
    assert_eq!(linked_definition.path(), definition.path());
    assert_eq!(linked_definition.name(), definition.name());

    let applications = context
        .trait_applications_by_path("ExternalGenericContract")
        .expect("generic external trait applications are queryable by source path");
    assert_eq!(applications.len(), 2);
    assert!(
        applications
            .iter()
            .all(|application| application.definition().is_some())
    );

    let mut const_arguments = applications
        .iter()
        .map(|application| {
            let arguments = application.arguments();
            assert_eq!(arguments.len(), 2);

            let type_argument = arguments
                .get_at(0)
                .expect("the first positional argument is retained")
                .type_argument()
                .expect("the first argument is a type");
            assert!(type_argument.parameter().is_none());
            assert_eq!(
                type_argument.concrete_type().expect("concrete type is retained").name(),
                "u16"
            );

            let const_argument = arguments
                .get_at(1)
                .expect("the second positional argument is retained")
                .const_argument()
                .expect("the second argument is a const");
            assert!(const_argument.parameter().is_none());
            match const_argument.expression() {
                qubit_reflect::descriptor::ConstExpression::UnsignedInteger(value) => value,
                other => panic!("expected an unsigned const argument, got {other:?}"),
            }
        })
        .collect::<Vec<_>>();
    const_arguments.sort_unstable();
    assert_eq!(const_arguments, [17, 18]);
}

#[test]
fn test_distinct_trait_handles_can_share_a_source_path() {
    let context = ReflectionContext::global();
    let lookup = context
        .lookup_trait_definition_by_name("std::fmt::Display")
        .expect("duplicate display paths remain lookup candidates");
    assert!(matches!(lookup, qubit_reflect::lookup::Lookup::Ambiguous(_)));
    let error = match context.trait_definition_by_name("std::fmt::Display") {
        Ok(_) => panic!("same-path trait handles should remain ambiguous"),
        Err(error) => error,
    };
    let qubit_reflect::error::LookupError::Ambiguous { candidates, .. } = error else {
        panic!("same-path trait handles should report ambiguous candidates");
    };
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].display_name, candidates[1].display_name);
    assert_ne!(candidates[0].identity.stable_id, candidates[1].identity.stable_id);
    assert!(
        ReflectionContext::global()
            .trait_definition_for::<duplicate_source_path_handle::AnotherDisplayHandle>()
            .expect("second typed handle is available")
            .is_some()
    );
}

#[test]
fn test_external_impl_source_path_links_to_external_trait_identity() {
    // The explicit handle links an impl-side alias to the external identity.
    let definition = ReflectionContext::global()
        .trait_definition_for::<FmtDisplay>()
        .expect("global context builds with the external impl")
        .expect("the external trait handle is registered");
    assert_eq!(definition.path(), "std::fmt::Display");
    assert_eq!(definition.name(), "Display");
    assert_eq!(
        definition.completeness(),
        qubit_reflect::__private::codegen_v4::traits::Completeness::ExternalIncomplete
    );
    let applications = ReflectionContext::global()
        .trait_applications_by_path("DisplayAlias")
        .expect("external impl alias remains queryable by its source path");
    assert_eq!(applications.len(), 1);
    assert_eq!(applications[0].path(), "DisplayAlias");
    assert_eq!(applications[0].completeness(), definition.completeness());
    assert_eq!(
        applications[0]
            .find_definition()
            .expect("typed external identity link is retained")
            .path(),
        "std::fmt::Display",
    );
}

#[test]
fn test_external_impl_without_handle_keeps_context_buildable() {
    // `Debug` has no registered identity handle in this fixture. Its impl
    // remains an incomplete source-path record and does not break freezing.
    let context = ReflectionContext::global();
    let applications = context
        .trait_applications_by_path("std::fmt::Debug")
        .expect("unhandled external impl does not invalidate context");
    assert_eq!(applications.len(), 1);
    assert_eq!(applications[0].path(), "std::fmt::Debug");
    assert!(applications[0].find_definition().is_none());
    assert_eq!(
        applications[0].completeness(),
        qubit_reflect::__private::codegen_v4::traits::Completeness::ExternalIncomplete,
    );

    let target = context
        .type_by_name(std::any::type_name::<UnregisteredExternalDebug>())
        .expect("external Debug target is registered");
    assert!(
        target
            .find_trait_impl(applications[0])
            .expect("the exact unlinked application resolves its own impl")
            .is_some()
    );
}

#[test]
fn test_unreflected_nonstandard_trait_keeps_context_buildable() {
    let context = ReflectionContext::global();
    let applications = context
        .trait_applications_by_path("third_party::RemoteContract")
        .expect("unreflected nonstandard trait does not require a generated provider");
    assert_eq!(applications.len(), 1);
    assert_eq!(applications[0].path(), "third_party::RemoteContract");
    assert_eq!(
        applications[0].completeness(),
        qubit_reflect::__private::codegen_v4::traits::Completeness::ExternalIncomplete,
    );
}

#[test]
fn test_unlinked_local_trait_path_is_not_promoted_by_matching_text() {
    let context = ReflectionContext::global();
    let applications = context
        .trait_applications_by_path("codegen_v4_trait_registration_tests::SameSourcePathTrait")
        .expect("the naked impl remains an unresolved source-path record");
    assert_eq!(applications.len(), 1);
    assert_eq!(
        applications[0].completeness(),
        qubit_reflect::__private::codegen_v4::traits::Completeness::ExternalIncomplete,
    );
    assert!(applications[0].find_definition().is_none());
}

#[test]
fn test_unlinked_local_trait_default_method_is_not_exposed() {
    let context = ReflectionContext::global();
    let applications = context
        .trait_applications_by_path("UnlinkedDefaultMethodTrait")
        .expect("the unlinked trait application remains queryable by its source path");
    assert_eq!(applications.len(), 1);
    assert_eq!(applications[0].path(), "UnlinkedDefaultMethodTrait");
    assert!(applications[0].find_definition().is_none());
    assert_eq!(
        applications[0].completeness(),
        qubit_reflect::__private::codegen_v4::traits::Completeness::ExternalIncomplete,
    );

    let target = context
        .type_by_name(std::any::type_name::<UnlinkedDefaultMethodTarget>())
        .expect("the target type is registered");
    let implementation = target
        .trait_impls()
        .expect("the target trait implementations are available")
        .iter()
        .find(|implementation| {
            implementation
                .implemented_trait()
                .is_some_and(|implemented_trait| implemented_trait.path() == applications[0].path())
        })
        .expect("the unlinked trait implementation remains registered");
    assert!(implementation.method("unlinked_default_method").is_err());
    assert!(
        target
            .lookup_method("unlinked_default_method")
            .is_ok_and(|lookup| lookup.is_missing())
    );
}

#[test]
fn test_same_source_alias_does_not_merge_unlinked_trait_identities() {
    let context = ReflectionContext::global();
    context
        .initialize()
        .expect("distinct unlinked traits can share an alias");

    let shared = context
        .type_by_name(std::any::type_name::<SharedUnlinkedAliasTarget>())
        .expect("shared target is registered");
    assert_eq!(
        shared.trait_impls().expect("shared impls are available").len(),
        2,
        "both distinct Rust trait impls must remain in the context",
    );

    let first = context
        .type_by_name(std::any::type_name::<FirstUnlinkedAliasTarget>())
        .expect("first target is registered");
    let first_application = first
        .trait_impls()
        .expect("first target impl is available")
        .iter()
        .next()
        .and_then(|implementation| implementation.implemented_trait())
        .expect("first trait application is registered");

    let second = context
        .type_by_name(std::any::type_name::<SecondUnlinkedAliasTarget>())
        .expect("second target is registered");
    assert!(
        second
            .find_trait_impl(first_application)
            .expect("unlinked source paths cannot establish exact identity")
            .is_none(),
        "same-spelled aliases must not match different external traits",
    );

    assert_eq!(
        context
            .trait_applications_by_path("LocalContract")
            .expect("source path lookup keeps all candidates")
            .len(),
        4,
    );
}

#[test]
fn test_unregistered_external_trait_import_alias_is_incomplete() {
    let context = ReflectionContext::global();
    context
        .initialize()
        .expect("an unmarked external trait alias does not require a provider");
    let applications = context
        .trait_applications_by_path("LocalDebug")
        .expect("the single-segment imported trait alias is retained");

    assert_eq!(applications.len(), 1);
    assert_eq!(applications[0].path(), "LocalDebug");
    assert!(applications[0].find_definition().is_none());
    assert_eq!(
        applications[0].completeness(),
        qubit_reflect::__private::codegen_v4::traits::Completeness::ExternalIncomplete,
    );
}

#[test]
fn test_distinct_unlinked_trait_aliases_keep_method_namespaces_separate() {
    let context = ReflectionContext::global();
    context
        .initialize()
        .expect("distinct unlinked trait applications may expose same-named methods");
    let target = context
        .type_by_name(std::any::type_name::<DistinctUnlinkedMethodTarget>())
        .expect("method target is registered");
    assert!(matches!(
        target.lookup_method("value").expect("method catalog is available"),
        qubit_reflect::lookup::Lookup::Ambiguous(candidates) if candidates.len() == 2
    ));
}

#[test]
fn test_registered_trait_object_is_queryable_by_exact_rust_type() {
    let object = ReflectionContext::global()
        .trait_object::<dyn std::fmt::Display>()
        .expect("global context builds")
        .expect("trait object is linked");

    assert_eq!(object.trait_path(), "dyn std :: fmt :: Display");
}
