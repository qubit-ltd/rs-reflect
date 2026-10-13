// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Integration coverage for associated items in reflected implementations.

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::AssociatedConstImplementationSource;
use qubit_reflect::descriptor::ImplDescriptor;
use qubit_reflect::expression::TypeExpression;
use qubit_reflect::registry::ReflectRegistry;

#[Reflect]
struct AssociatedItemTarget;

#[Reflect]
trait AssociatedContract {
    type Output: Reflect;

    const DEFAULTED: u32 = 17;
    const OVERRIDDEN: u32 = 19;
}

#[ReflectImpl(definition_provider_v2 = __qubit_reflect_trait_definition_AssociatedContract)]
impl AssociatedContract for AssociatedItemTarget {
    type Output = u16;

    const OVERRIDDEN: u32 = 23;
}

fn implementation_for<T: Reflect>(trait_name: &str) -> &'static ImplDescriptor {
    ReflectRegistry::initialize()
        .expect("associated-item fragments must initialize")
        .implementations(TypeDescriptor::of::<T>().type_id())
        .iter()
        .copied()
        .find(|implementation| {
            implementation
                .implemented_trait()
                .is_some_and(|descriptor| descriptor.rust_name() == trait_name)
        })
        .expect("the reflected trait implementation must be registered")
}

#[test]
fn test_reflect_impl_records_associated_type_and_constant_bindings() {
    let implementation = implementation_for::<AssociatedItemTarget>("AssociatedContract");
    let [output] = implementation.associated_types() else {
        panic!("the associated type binding must be present")
    };
    assert_eq!(output.declaration().rust_name(), "Output");
    assert!(matches!(
        output.value(),
        TypeExpression::Concrete(value)
            if value.path().last().is_some_and(|segment| segment.as_ref() == "u16")
    ));
    assert!(std::ptr::eq(
        output
            .concrete_type()
            .expect("the declared Reflect bound proves a descriptor"),
        TypeDescriptor::of::<u16>(),
    ));

    let [defaulted, overridden] = implementation.associated_consts() else {
        panic!("both associated constants must be described")
    };
    assert_eq!(defaulted.declaration().rust_name(), "DEFAULTED");
    assert_eq!(overridden.declaration().rust_name(), "OVERRIDDEN");
    assert_eq!(
        defaulted.implementation_source(),
        AssociatedConstImplementationSource::Defaulted
    );
    assert_eq!(
        overridden.implementation_source(),
        AssociatedConstImplementationSource::Overridden
    );
    assert_eq!(defaulted.read::<u32>().expect("default const reader"), Ok(17));
    assert_eq!(overridden.read::<u32>().expect("override const reader"), Ok(23));
}
