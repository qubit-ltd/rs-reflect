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
use qubit_reflect::descriptor::AssociatedItemSource;
use qubit_reflect::descriptor::ImplDescriptor;

#[Reflect]
struct AssociatedItemTarget;

#[Reflect]
trait AssociatedContract {
    type Output: qubit_reflect::descriptor::Reflect;

    const DEFAULTED: u32 = 17;
    const OVERRIDDEN: u32 = 19;
}

#[ReflectImpl]
impl AssociatedContract for AssociatedItemTarget {
    type Output = u16;

    const OVERRIDDEN: u32 = 23;
}

fn implementation_for<T: qubit_reflect::descriptor::Reflect>(
    trait_name: &str,
) -> ImplDescriptor<'static> {
    TypeDescriptor::of::<T>()
        .trait_impls()
        .expect("associated-item fragments must initialize")
        .iter()
        .find(|implementation| {
            implementation
                .implemented_trait()
                .and_then(|descriptor| descriptor.definition())
                .is_some_and(|definition| definition.name() == trait_name)
        })
        .expect("the reflected trait implementation must be registered")
}

#[test]
fn test_reflect_impl_records_associated_type_and_constant_bindings() {
    let implementation = implementation_for::<AssociatedItemTarget>("AssociatedContract");
    let output = implementation
        .associated_type("Output")
        .expect("the associated type binding must be present");
    assert_eq!(output.definition().name(), "Output");
    assert!(format!("{}", output.expression().display()).contains("u16"));
    assert_eq!(
        output
            .find_concrete_type()
            .expect("the declared Reflect bound proves a descriptor")
            .type_id(),
        TypeDescriptor::of::<u16>().type_id(),
    );

    let defaulted = implementation
        .associated_const("DEFAULTED")
        .expect("the defaulted constant must be described");
    let overridden = implementation
        .associated_const("OVERRIDDEN")
        .expect("the overridden constant must be described");
    assert_eq!(defaulted.definition().name(), "DEFAULTED");
    assert_eq!(overridden.definition().name(), "OVERRIDDEN");
    assert_eq!(defaulted.source(), AssociatedItemSource::Defaulted);
    assert_eq!(overridden.source(), AssociatedItemSource::Overridden);
}
