// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;

use qubit_reflect::Reflect;
use qubit_reflect::ReflectionContext;
use qubit_reflect::TypeDescriptor;

#[Reflect]
struct GenericRecord<T, const N: usize> {
    value: T,
    bytes: [u8; N],
}

#[Reflect]
enum GenericChoice<T, const N: usize> {
    Value(T),
    Bytes([u8; N]),
}

#[Reflect]
struct PlainRecord {
    value: u8,
}

#[Reflect]
struct ConstRecord<const SIGNED: i32, const FLAG: bool, const LETTER: char>;

struct Opaque;

impl qubit_reflect::__private::codegen_v4::descriptor::Reflect for Opaque {
    fn __reflect_registration() -> &'static qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<Self> {
        static REGISTRATION: qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<Opaque> =
            qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration::new();
        &REGISTRATION
    }
}

#[test]
fn type_descriptor_reports_registration_definition_and_generic_arguments() {
    let _ = GenericChoice::<u32, 3>::Value(1);
    let _ = GenericChoice::<u32, 3>::Bytes([0; 3]);
    let reflected = ReflectionContext::builder()
        .include_type::<GenericRecord<String, 7>>()
        .include_type::<GenericChoice<u32, 3>>()
        .include_type::<PlainRecord>()
        .include_type::<ConstRecord<-5, true, 'q'>>()
        .build()
        .expect("reflection context builds");

    let record = TypeDescriptor::of_in::<GenericRecord<String, 7>>(&reflected);
    assert!(record.is_registered().expect("context is valid"));
    let definition = record.find_definition().expect("generic record has a definition");
    assert_eq!(definition.name(), "GenericRecord");
    assert_eq!(
        record.definition().expect("definition exists").path(),
        definition.path()
    );
    let arguments = record.generic_arguments();
    assert_eq!(arguments.iter().len(), 2);
    assert_eq!(
        arguments
            .argument("T")
            .expect("type argument exists")
            .type_argument()
            .expect("argument is a type")
            .concrete_type()
            .expect("type argument has an exact type")
            .type_id(),
        TypeId::of::<String>()
    );
    assert_eq!(
        arguments
            .argument("N")
            .expect("const argument exists")
            .const_argument()
            .expect("argument is const")
            .expression(),
        qubit_reflect::descriptor::ConstExpression::UnsignedInteger(7)
    );

    let choice = TypeDescriptor::of_in::<GenericChoice<u32, 3>>(&reflected);
    assert!(choice.find_definition().is_some());
    assert_eq!(choice.generic_arguments().iter().len(), 2);
    assert_eq!(
        choice
            .generic_arguments()
            .argument("N")
            .expect("const argument exists")
            .const_argument()
            .expect("argument is const")
            .expression(),
        qubit_reflect::descriptor::ConstExpression::UnsignedInteger(3)
    );

    let plain = TypeDescriptor::of_in::<PlainRecord>(&reflected);
    assert!(plain.is_registered().expect("context is valid"));
    assert!(plain.generic_arguments().is_empty());
    assert!(plain.find_definition().is_some());

    let constants = TypeDescriptor::of_in::<ConstRecord<-5, true, 'q'>>(&reflected).generic_arguments();
    assert_eq!(constants.iter().len(), 3);
    assert_eq!(
        constants
            .argument("SIGNED")
            .expect("signed const argument exists")
            .const_argument()
            .expect("argument is const")
            .expression(),
        qubit_reflect::descriptor::ConstExpression::SignedInteger(-5)
    );
    assert_eq!(
        constants
            .argument("FLAG")
            .expect("bool const argument exists")
            .const_argument()
            .expect("argument is const")
            .expression(),
        qubit_reflect::descriptor::ConstExpression::Boolean(true)
    );
    assert_eq!(
        constants
            .argument("LETTER")
            .expect("char const argument exists")
            .const_argument()
            .expect("argument is const")
            .expression(),
        qubit_reflect::descriptor::ConstExpression::Character('q')
    );

    let unregistered = TypeDescriptor::of_in::<GenericRecord<String, 9>>(&reflected);
    assert!(!unregistered.is_registered().expect("context is valid"));
    assert!(unregistered.find_definition().is_some());

    let opaque = TypeDescriptor::of_in::<Opaque>(&reflected);
    assert!(!opaque.is_registered().expect("context is valid"));
    assert!(opaque.find_definition().is_none());
    assert!(opaque.definition().is_err());
    assert!(opaque.generic_arguments().is_empty());
}
