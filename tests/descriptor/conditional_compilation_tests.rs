// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Public behavior for conditionally configured reflected trait and impl
//! members.

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::descriptor::TraitDefinitionDescriptor;
use qubit_reflect::ReflectionContext;
use qubit_reflect::TypeDescriptor;

#[Reflect]
struct ConditionalTarget;

#[cfg(any())]
#[Reflect]
trait EntirelyDisabledTrait {
    fn absent(&self) -> MissingTraitType;
}

#[Reflect]
#[allow(dead_code)]
trait ConditionalMetadata {
    #[cfg(any())]
    type DisabledAssociated = MissingTraitType;

    #[cfg(target_pointer_width = "64")]
    type ActiveAssociated;

    #[cfg(any())]
    const DISABLED_CONST: MissingTraitType;

    #[cfg(target_pointer_width = "64")]
    const ACTIVE_CONST: u32;

    #[cfg(any())]
    fn disabled(&self) -> MissingTraitType;

    #[cfg(target_pointer_width = "64")]
    fn enabled(&self) -> u32;

    #[cfg(target_pointer_width = "64")]
    fn platform_64(&self);

    #[cfg(not(target_pointer_width = "64"))]
    fn platform_other(&self);

    fn cfg_attr_policy(&self);

    #[cfg(any())]
    fn disabled_invalid_helper(&self);

    fn configured_helper(&self);

    fn retained_but_unavailable(&self);
}

#[cfg(any())]
#[ReflectImpl]
impl ConditionalTarget {
    fn absent(&self) -> MissingImplType {
        unreachable!()
    }
}

#[ReflectImpl]
impl ConditionalTarget {
    #[cfg(any())]
    fn disabled(&self) -> MissingImplType {
        unreachable!()
    }

    #[cfg(target_pointer_width = "64")]
    fn enabled(&self) -> u32 {
        1
    }

    fn configured_method(&self) -> u32 {
        2
    }
}

fn method_names(definition: &TraitDefinitionDescriptor) -> Vec<&str> {
    definition.methods().iter().map(|method| method.rust_name()).collect()
}

#[test]
fn test_cfg_controls_trait_metadata_members() {
    let context = ReflectionContext::global();
    context.initialize().expect("active trait fragments must initialize");
    let definition = context
        .trait_definition_by_name(concat!(module_path!(), "::ConditionalMetadata"))
        .expect("the reflected trait definition must be registered");

    #[cfg(target_pointer_width = "64")]
    {
        assert_eq!(
            method_names(definition),
            [
                "enabled",
                "platform_64",
                "cfg_attr_policy",
                "configured_helper",
                "retained_but_unavailable",
            ]
        );
        assert_eq!(definition.methods().get("platform_64").unwrap().name(), "platform_64");
        assert_eq!(definition.methods().get("configured_helper").unwrap().name(), "configured_helper");
        assert_eq!(definition.associated_types().len(), 1);
        assert_eq!(definition.associated_consts().len(), 1);
    }
    #[cfg(not(target_pointer_width = "64"))]
    {
        assert_eq!(
            method_names(definition),
            [
                "platform_other",
                "cfg_attr_policy",
                "configured_helper",
                "retained_but_unavailable",
            ]
        );
        assert_eq!(definition.methods().get("platform_other").unwrap().name(), "platform_other");
        assert_eq!(definition.methods().get("configured_helper").unwrap().name(), "configured_helper");
    }
    assert!(
        context
            .trait_definition_by_name(concat!(module_path!(), "::EntirelyDisabledTrait"))
            .is_err(),
        "a fully cfg-disabled trait must not be registered",
    );
}

#[test]
fn test_inactive_impl_member_has_no_descriptor_or_adapter() {
    let context = ReflectionContext::global();
    context.initialize().expect("active impl fragments must initialize");
    let implementations = TypeDescriptor::of::<ConditionalTarget>()
        .inherent_impls()
        .expect("active inherent impls must be available");
    let implementation = implementations
        .iter()
        .find(|implementation| {
            implementation
                .methods()
                .iter()
                .any(|method| method.rust_name() == "enabled")
        })
        .expect("the active method's implementation must be registered");

    assert_eq!(
        implementation
            .methods()
            .iter()
            .map(|method| method.rust_name())
            .collect::<Vec<_>>(),
        ["enabled", "configured_method"]
    );
}
