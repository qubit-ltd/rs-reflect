// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::__private::codegen_v4::descriptor::Reflect;
use qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration;
use qubit_reflect::construct::ConstructField;
use qubit_reflect::construct::ConstructionAvailability;
use qubit_reflect::construct::ConstructionError;
use qubit_reflect::construct::ConstructionFieldPolicy;
use qubit_reflect::construct::NamedFields;
use qubit_reflect::construct::UnitFields;
use qubit_reflect::construct::UpdateFieldPolicy;
use qubit_reflect::construct::UpdateInput;
use qubit_reflect::descriptor::TypeDescriptor;
use qubit_reflect::descriptor::Visibility;
use qubit_reflect::value::Local;

#[derive(Debug)]
struct ContractRecord {
    value: u32,
}

#[derive(Debug)]
enum ContractEnum {
    Empty,
}

impl Reflect for ContractRecord {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        static REGISTRATION: TypeRegistration<ContractRecord> = TypeRegistration::new();
        &REGISTRATION
    }
}

impl Reflect for ContractEnum {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        static REGISTRATION: TypeRegistration<ContractEnum> = TypeRegistration::new();
        &REGISTRATION
    }
}

qubit_reflect::__private::codegen_v4::registration::submit_type!(ContractRecord);
qubit_reflect::__private::codegen_v4::registration::submit_type!(ContractEnum);

#[test]
fn construction_facade_is_usable_from_downstream_code() {
    let _empty = ContractEnum::Empty;
    let descriptor = TypeDescriptor::of::<ContractRecord>();
    let construction = descriptor.construction();
    assert!(matches!(
        construction.availability(),
        ConstructionAvailability::Unavailable { .. }
    ));
    assert_eq!(construction.fields().len(), 0);

    let failure = descriptor
        .construct::<ContractRecord, _>(NamedFields::<Local>::new().field("value", 7_u32))
        .expect_err("the fixture intentionally has no generated provider");
    assert!(matches!(
        failure.error(),
        ConstructionError::WrongTargetKind | ConstructionError::Unavailable(_)
    ));
    assert!(!format!("{failure:?}").is_empty());
    assert!(matches!(
        failure.into_input(),
        qubit_reflect::construct::RecoveredConstructionInput::Named(_)
    ));

    let unchecked_construction = unsafe { descriptor.construct_unchecked::<ContractRecord, _>(UnitFields::new()) };
    assert!(unchecked_construction.is_err());

    let update = descriptor.update::<ContractRecord, _>(UpdateInput::<Local>::new(ContractRecord { value: 1 }));
    assert!(
        matches!(update, Err(failure) if matches!(failure.error(), qubit_reflect::construct::UpdateError::WrongTargetKind | qubit_reflect::construct::UpdateError::UpdateUnavailable(_)))
    );

    let _unit_input = UnitFields::new();
    let field = ConstructField::typed::<u32>(
        "value",
        Visibility::Public,
        ConstructionFieldPolicy::Required,
        UpdateFieldPolicy::Replaceable,
    );
    assert_eq!(field.name(), "value");

    let _dynamic_construct = descriptor.construct_dynamic::<_>(NamedFields::<Local>::new().field("value", 5_u32));
    let _dynamic_update = descriptor.update_dynamic(UpdateInput::<Local>::from_dynamic(
        qubit_reflect::value::DynamicOwned::new(ContractRecord { value: 2 }),
    ));

    if let Some(variant) = TypeDescriptor::of::<ContractEnum>().variants().get("Empty") {
        let _typed_variant = variant.construct::<ContractEnum, _>(UnitFields::new());
        let _dynamic_variant = variant.construct_dynamic::<_>(UnitFields::new());
    }

    assert_eq!(ContractRecord { value: 11 }.value, 11);
}
