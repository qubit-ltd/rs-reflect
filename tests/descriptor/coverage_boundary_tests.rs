// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Boundary coverage for trait, method, and impl descriptor queries.

#![allow(dead_code)] // Metadata-only trait fixtures intentionally have no executable use.

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::ImplKind;
use qubit_reflect::descriptor::TraitCompleteness;
use qubit_reflect::descriptor::TraitVisibility;
use qubit_reflect::error::LookupError;
use qubit_reflect::error::LookupQuery;
use qubit_reflect::error::LookupSubject;
use qubit_reflect::lookup::Lookup;

#[Reflect]
struct DescriptorTarget;

#[Reflect]
struct ExternalTraitTarget;

#[Reflect]
trait DescriptorBase {
    type Output: qubit_reflect::descriptor::Reflect;

    const LIMIT: u32 = 37;

    fn required(&self, value: u8);

    fn defaulted(&self) -> u16 {
        37
    }
}

#[Reflect]
trait DescriptorChild: DescriptorBase + std::fmt::Display {
    fn child(&self);
}

#[Reflect]
trait RepeatedGenericContract<T> {
    fn value(&self) -> T;
}

#[Reflect]
trait UnimplementedContract {
    fn absent(&self);
}

#[ReflectImpl(reflected_trait = DescriptorBase)]
impl DescriptorBase for DescriptorTarget {
    type Output = u16;

    fn required(&self, _value: u8) {}
}

#[ReflectImpl(reflected_trait = DescriptorChild)]
impl DescriptorChild for DescriptorTarget {
    fn child(&self) {}
}

#[ReflectImpl(reflected_trait = RepeatedGenericContract::<u8>)]
impl RepeatedGenericContract<u8> for DescriptorTarget {
    fn value(&self) -> u8 {
        8
    }
}

#[ReflectImpl(reflected_trait = RepeatedGenericContract::<u16>)]
impl RepeatedGenericContract<u16> for DescriptorTarget {
    fn value(&self) -> u16 {
        16
    }
}

#[ReflectImpl]
impl DescriptorTarget {
    fn inherent_only(&self) {}
}

impl std::fmt::Display for DescriptorTarget {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("descriptor target")
    }
}

#[ReflectImpl]
impl std::fmt::Debug for ExternalTraitTarget {
    #[reflect(skip)]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ExternalTraitTarget")
    }
}

fn trait_implementation<T: qubit_reflect::descriptor::Reflect>(
    name: &str,
) -> qubit_reflect::descriptor::ImplDescriptor<'static> {
    TypeDescriptor::of::<T>()
        .trait_impls()
        .expect("trait implementation fragments initialize")
        .iter()
        .find(|implementation| {
            implementation
                .implemented_trait()
                .is_some_and(|trait_| trait_.name() == name)
        })
        .expect("the requested implementation is registered")
}

#[test]
fn trait_definition_exposes_declaration_defaults_and_precise_missing_errors() {
    let context = ReflectionContext::global();
    context.initialize().expect("descriptor fixtures register");
    let definition = context
        .trait_definition_by_name(concat!(module_path!(), "::DescriptorBase"))
        .expect("the trait definition is indexed by its source path");

    assert_eq!(definition.completeness(), TraitCompleteness::Complete);
    assert_eq!(definition.visibility(), TraitVisibility::Private);
    assert_eq!(definition.supertraits().len(), 0);
    assert_eq!(definition.supertraits().iter().len(), 0);
    assert_eq!(
        definition.method("required").unwrap().visibility(),
        TraitVisibility::Public
    );
    assert!(definition.method("defaulted").unwrap().has_default_body());
    assert!(matches!(
        definition.method("absent"),
        Err(LookupError::Missing {
            subject: LookupSubject::Method,
            query: LookupQuery::Name(name),
        }) if name.as_ref() == "absent"
    ));

    let output = definition.associated_type("Output").expect("associated type exists");
    assert_eq!(output.name(), "Output");
    assert!(output.default().is_none());
    assert!(!output.bounds().is_empty());

    let limit = definition
        .associated_const("LIMIT")
        .expect("associated constant exists");
    assert_eq!(
        limit.default(),
        Some(qubit_reflect::descriptor::ConstExpression::UnsignedInteger(37))
    );
    assert!(limit.ty().display().to_string().contains("u32"));
    assert!(matches!(
        definition.associated_type("MissingType"),
        Err(LookupError::Missing {
            subject: LookupSubject::AssociatedType,
            query: LookupQuery::Name(name),
        }) if name.as_ref() == "MissingType"
    ));
    assert!(matches!(
        definition.associated_const("MissingConst"),
        Err(LookupError::Missing {
            subject: LookupSubject::AssociatedConst,
            query: LookupQuery::Name(name),
        }) if name.as_ref() == "MissingConst"
    ));
}

#[test]
fn trait_application_reports_supertrait_closure_completeness_and_unresolved_external_trait() {
    let child = trait_implementation::<DescriptorTarget>("DescriptorChild")
        .implemented_trait()
        .expect("the child trait application is present");

    assert_eq!(child.completeness(), TraitCompleteness::Complete);
    assert!(child.find_definition().is_some());
    assert_eq!(child.direct_supertraits().len(), 2);
    assert_eq!(child.all_supertraits().len(), 2);
    assert_eq!(format!("{:?}", child.supertrait_completeness()), "Incomplete");

    let external = TypeDescriptor::of::<ExternalTraitTarget>()
        .trait_impls()
        .expect("external impl metadata is available")
        .iter()
        .find_map(|implementation| {
            implementation
                .implemented_trait()
                .filter(|trait_| trait_.name() == "Debug")
        });
    let external = external.expect("the bare external impl is retained");
    assert_eq!(external.completeness(), TraitCompleteness::ExternalIncomplete);
    assert!(external.find_definition().is_none());
    assert!(external.path().contains("Debug"));
}

#[test]
fn impl_descriptors_distinguish_source_declarations_and_concrete_applications() {
    let context = ReflectionContext::global();
    context.initialize().expect("descriptor fixtures register");
    let source = context
        .impl_definitions()
        .expect("source impl declarations are available")
        .iter()
        .find(|definition| {
            definition
                .implemented_trait()
                .is_some_and(|trait_| trait_.path().contains("DescriptorBase"))
        })
        .expect("the DescriptorBase source impl is registered");

    assert_eq!(source.kind(), ImplKind::Trait);
    assert_eq!(source.methods().len(), 1);
    assert_eq!(source.method("required").unwrap().name(), "required");
    assert!(matches!(
        source.method("missing"),
        Err(LookupError::Missing {
            subject: LookupSubject::Method,
            query: LookupQuery::Name(name),
        }) if name.as_ref() == "missing"
    ));
    assert!(source.associated_type("MissingType").is_err());
    assert!(source.associated_const("MissingConst").is_err());
    assert_eq!(
        source
            .instances()
            .expect("source impl applications are queryable")
            .len(),
        1
    );

    let concrete = trait_implementation::<DescriptorTarget>("DescriptorBase");
    assert_eq!(concrete.kind(), ImplKind::Trait);
    assert_eq!(
        concrete.target().type_id(),
        TypeDescriptor::of::<DescriptorTarget>().type_id()
    );
    assert_eq!(concrete.method("defaulted").unwrap().name(), "defaulted");
    assert!(matches!(
        concrete.method("missing"),
        Err(LookupError::Missing {
            subject: LookupSubject::Method,
            query: LookupQuery::Name(name),
        }) if name.as_ref() == "missing"
    ));
    assert!(concrete.associated_type("MissingType").is_err());
    assert!(concrete.associated_const("MissingConst").is_err());

    let inherent = TypeDescriptor::of::<DescriptorTarget>()
        .inherent_impls()
        .expect("inherent applications are queryable")
        .iter()
        .next()
        .expect("the fixture has an inherent impl");
    assert_eq!(inherent.kind(), ImplKind::Inherent);
    assert!(inherent.implemented_trait().is_none());
    assert_eq!(inherent.method("inherent_only").unwrap().name(), "inherent_only");
    assert!(inherent.same_application(inherent));
    assert!(!concrete.same_application(inherent));
}

#[test]
fn impl_collection_reports_missing_and_ambiguous_trait_definition_queries() {
    let descriptor = TypeDescriptor::of::<DescriptorTarget>();
    let implementations = descriptor.trait_impls().expect("trait impls are indexed");
    let context = ReflectionContext::global();
    context.initialize().expect("trait definitions are registered");
    let generic_definition = context
        .trait_definition_by_name(concat!(module_path!(), "::RepeatedGenericContract"))
        .expect("the generic trait declaration exists");
    let missing_definition = context
        .trait_definition_by_name(concat!(module_path!(), "::UnimplementedContract"))
        .expect("the unimplemented trait declaration exists");

    assert!(matches!(
        implementations.lookup_trait_definition(missing_definition).unwrap(),
        Lookup::Missing
    ));
    let matches = implementations
        .lookup_trait_definition(generic_definition)
        .expect("lookup by source definition succeeds");
    let candidates = matches.candidates().to_vec();
    assert_eq!(candidates.len(), 2);
    assert!(matches!(matches, Lookup::Ambiguous(_)));

    for implementation in candidates {
        let application = implementation
            .implemented_trait()
            .expect("generic trait application exists");
        assert!(
            implementations
                .find_trait_application(application)
                .expect("exact application lookup succeeds")
                .is_some_and(|found| found.same_application(implementation))
        );
    }
}

#[test]
fn impl_collection_rejects_descriptors_from_a_different_context() {
    let global = ReflectionContext::global();
    global.initialize().expect("global metadata initializes");
    let foreign_definition = global
        .trait_definition_by_name(concat!(module_path!(), "::DescriptorBase"))
        .expect("global trait definition exists");
    let foreign_application = trait_implementation::<DescriptorTarget>("DescriptorBase")
        .implemented_trait()
        .expect("global trait application exists");
    let isolated = ReflectionContext::builder()
        .build()
        .expect("empty isolated context builds");
    let empty_impls = isolated
        .type_of::<DescriptorTarget>()
        .trait_impls()
        .expect("empty context returns an empty trait impl collection");

    assert!(empty_impls.is_empty());
    assert!(empty_impls.lookup_trait_definition(foreign_definition).is_err());
    assert!(empty_impls.find_trait_application(foreign_application).is_err());
}
