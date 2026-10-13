// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]
#![allow(dead_code)]

use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::TraitId;
use qubit_reflect::expression::TypeExpression;
use qubit_reflect::registry::ReflectRegistry;

#[Reflect]
trait Parent<T> {
    fn parent_value(&self, value: T) -> T;
}

#[Reflect(supertrait(Parent<u8>), dyn_compatible)]
trait Child: Parent<u8> {
    fn child_value(&self) -> u8;
}

#[test]
fn generic_dyn_trait_keeps_identity_and_applied_direct_supertrait() {
    let object_descriptor = TypeDescriptor::of::<dyn Child>();
    let repeated_object_descriptor = TypeDescriptor::of::<dyn Child>();
    let applied = object_descriptor
        .as_trait_object()
        .expect("the declared dyn-compatible trait exposes its applied descriptor")
        .trait_descriptor();
    let registry = ReflectRegistry::initialize().expect("trait registration must succeed");
    let definition = registry
        .trait_definition_by_path(concat!(module_path!(), "::Child"))
        .expect("the child trait definition is registered");

    assert!(std::ptr::eq(object_descriptor, repeated_object_descriptor));
    assert!(std::ptr::eq(applied.definition(), definition));
    assert!(matches!(definition.trait_id(), TraitId::Reflected(_)));
    assert_eq!(applied.direct_supertraits().len(), 1);

    let parent = applied.direct_supertraits()[0].descriptor();
    assert_eq!(parent.rust_name(), "Parent");
    assert_eq!(parent.arguments().len(), 1);
    assert!(matches!(
        parent.methods()[0].parameters()[0].signature_type(),
        TypeExpression::Concrete(value)
            if value.path().last().is_some_and(|segment| segment.as_ref() == std::any::type_name::<u8>())
    ));
}
