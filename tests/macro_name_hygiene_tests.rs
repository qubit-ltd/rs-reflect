// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

mod shadowed_names {
    #![allow(dead_code)]

    use qubit_reflect::Reflect;

    pub struct Option;
    pub struct Some;
    pub struct None;
    pub struct Result;
    pub struct Vec;

    #[Reflect]
    #[derive(Debug, PartialEq)]
    pub struct Named {
        pub first: u32,
        pub second: u32,
    }

    #[Reflect]
    #[derive(Debug, PartialEq)]
    pub struct Tuple(pub u32, pub u32);

    #[Reflect]
    #[derive(Debug, PartialEq)]
    pub struct Unit;

    #[Reflect]
    #[derive(Debug, PartialEq)]
    pub enum Choice {
        Named { first: u32, second: u32 },
        Tuple(u32, u32),
        Unit,
    }
}

use qubit_reflect::TypeDescriptor;
use qubit_reflect::construct::NamedFields;
use qubit_reflect::construct::PositionalFields;
use qubit_reflect::construct::UnitFields;
use qubit_reflect::construct::UpdateInput;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

#[test]
fn test_constructs_and_updates_shadowed_structs() {
    let descriptor = TypeDescriptor::of::<shadowed_names::Named>();
    let named: shadowed_names::Named = descriptor
        .construct(NamedFields::<Local>::new().field("first", 1_u32).field("second", 2_u32))
        .expect("named construction");
    assert_eq!(named, shadowed_names::Named { first: 1, second: 2 });

    let updated: shadowed_names::Named = descriptor
        .update(UpdateInput::<Local>::new(named).field("first", 9_u32))
        .expect("named update");
    assert_eq!(updated, shadowed_names::Named { first: 9, second: 2 });

    let tuple: shadowed_names::Tuple = TypeDescriptor::of::<shadowed_names::Tuple>()
        .construct(PositionalFields::<Local>::new().value(3_u32).value(4_u32))
        .expect("tuple construction");
    assert_eq!(tuple, shadowed_names::Tuple(3, 4));
    let tuple: shadowed_names::Tuple = TypeDescriptor::of::<shadowed_names::Tuple>()
        .update(UpdateInput::<Local>::new(tuple).field("0", 8_u32))
        .expect("tuple update");
    assert_eq!(tuple, shadowed_names::Tuple(8, 4));

    let _: shadowed_names::Unit = TypeDescriptor::of::<shadowed_names::Unit>()
        .construct(UnitFields)
        .expect("unit construction");

    let thread_safe: shadowed_names::Named = descriptor
        .construct(
            NamedFields::<ThreadSafe>::new()
                .field("first", 10_u32)
                .field("second", 11_u32),
        )
        .expect("thread-safe construction");
    assert_eq!(thread_safe, shadowed_names::Named { first: 10, second: 11 });
}

#[test]
fn test_constructs_and_updates_shadowed_enum_variants() {
    let descriptor = TypeDescriptor::of::<shadowed_names::Choice>();
    let named: shadowed_names::Choice = descriptor
        .variant("Named")
        .expect("named variant")
        .construct(NamedFields::<Local>::new().field("first", 1_u32).field("second", 2_u32))
        .expect("named variant construction");
    assert!(matches!(named, shadowed_names::Choice::Named { first: 1, second: 2 }));
    let tuple: shadowed_names::Choice = descriptor
        .variant("Tuple")
        .expect("tuple variant")
        .construct(PositionalFields::<Local>::new().value(3_u32).value(4_u32))
        .expect("tuple variant construction");
    assert!(matches!(tuple, shadowed_names::Choice::Tuple(3, 4)));
    let unit: shadowed_names::Choice = descriptor
        .variant("Unit")
        .expect("unit variant")
        .construct(UnitFields)
        .expect("unit variant construction");
    assert!(matches!(unit, shadowed_names::Choice::Unit));
}
