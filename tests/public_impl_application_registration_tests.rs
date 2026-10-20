// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use qubit_reflect::__private::codegen_v4::expression::ExpressionData;
use qubit_reflect::__private::codegen_v4::expression::GenericDefinitionMetadata;
use qubit_reflect::__private::codegen_v4::impls::ImplApplicationSpec;
use qubit_reflect::__private::codegen_v4::impls::ImplDefinitionSpec;
use qubit_reflect::__private::codegen_v4::impls::Kind;
use qubit_reflect::__private::codegen_v4::impls::Qualifiers;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;
use qubit_reflect::descriptor::SupertraitCompleteness;
use qubit_reflect::descriptor::TraitCompleteness;
use qubit_reflect::error::ItemKind;
use qubit_reflect::registration::Registration;

#[Reflect]
struct MethodCatalogOnlyTarget;

#[allow(dead_code)]
trait BareExternalContract<T> {}

#[Reflect]
struct BareExternalTarget;

#[ReflectImpl]
impl BareExternalContract<u8> for BareExternalTarget {}

#[ReflectImpl]
impl BareExternalContract<String> for BareExternalTarget {}

struct ManualImplMarker;

static EMPTY_EXPRESSION: ExpressionData = ExpressionData::SelfType;
static EMPTY_GENERIC_DEFINITION: GenericDefinitionMetadata = GenericDefinitionMetadata::new(&[], &[]);
static IMPL_DEFINITION: ImplDefinitionSpec<ManualImplMarker> = ImplDefinitionSpec::new(
    Kind::Inherent,
    &EMPTY_EXPRESSION,
    None,
    &EMPTY_GENERIC_DEFINITION,
    Qualifiers::new(false),
    &[],
    &[],
    &[],
);
static IMPL_APPLICATION: ImplApplicationSpec<MethodCatalogOnlyTarget> =
    ImplApplicationSpec::for_target(&IMPL_DEFINITION, None, &[], &[], &[], &[], &[]);

fn impl_registration() -> Registration {
    Registration::metadata_only()
        .with_impl_definition(IMPL_DEFINITION.metadata())
        .with_impl_application(IMPL_APPLICATION.metadata())
}

#[test]
fn method_catalog_does_not_satisfy_impl_application_target_inclusion() {
    let result = ReflectionContext::builder().register(impl_registration()).build();
    let error = match result {
        Ok(_) => panic!("a method catalog does not include its target type"),
        Err(error) => error,
    };
    assert!(error.issues().iter().any(|issue| matches!(
        issue,
        qubit_reflect::error::ContextIssue::MissingDependency { dependency, .. }
            if dependency.kind == ItemKind::Type
    )));

    let context = ReflectionContext::builder()
        .include_type::<MethodCatalogOnlyTarget>()
        .register(impl_registration())
        .build()
        .expect("including the target type satisfies the impl application dependency");
    assert!(context.type_of::<MethodCatalogOnlyTarget>().is_registered().unwrap());
}

#[test]
fn bare_external_generic_trait_applications_retain_positional_arguments() {
    let context = ReflectionContext::builder()
        .include_linked()
        .build()
        .expect("bare generic external trait impls do not require a provider");
    let applications = context
        .trait_applications_by_path("BareExternalContract")
        .expect("bare external applications remain queryable by source path");

    assert_eq!(applications.len(), 2);
    let mut type_ids = applications
        .iter()
        .map(|application| {
            assert!(application.find_definition().is_none());
            assert_eq!(
                application.supertrait_completeness(),
                SupertraitCompleteness::Incomplete
            );
            assert_eq!(application.completeness(), TraitCompleteness::ExternalIncomplete);
            let arguments = application.arguments();
            assert_eq!(arguments.len(), 1);
            arguments
                .iter()
                .next()
                .expect("the positional type argument is retained")
                .type_argument()
                .expect("the argument is a type")
                .concrete_type()
                .expect("the concrete type identity is retained")
                .type_id()
        })
        .collect::<Vec<_>>();
    type_ids.sort_by_key(|type_id| format!("{type_id:?}"));
    assert_eq!(type_ids.len(), 2);
    assert!(type_ids.contains(&std::any::TypeId::of::<u8>()));
    assert!(type_ids.contains(&std::any::TypeId::of::<String>()));
}
