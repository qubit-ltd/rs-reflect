// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Integration tests for construction adapters emitted by `#[derive(Reflect)]`.

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use qubit_reflect::Reflect;
use qubit_reflect::construct::ConstructionError;
use qubit_reflect::construct::NamedConstructionInput;
use qubit_reflect::construct::StructUpdateInput;
use qubit_reflect::construct::TupleConstructionInput;
use qubit_reflect::value::ReflectedOwned;

#[derive(Debug, Eq, PartialEq, Reflect)]
struct Profile {
    id: u32,
    label: String,
}

#[derive(Debug, Eq, PartialEq, Reflect)]
struct Defaults {
    id: u32,
    #[reflect(default)]
    label: String,
}

#[derive(Debug, Eq, PartialEq, Reflect)]
enum Event {
    Started,
    Failed {
        code: u32,
        message: String,
    },
    Defaulted {
        id: u32,
        #[reflect(default)]
        label: String,
    },
    Pair(u32, String),
}

#[derive(Reflect)]
struct Unavailable {
    id: u32,
    #[reflect(no_construct)]
    secret: String,
}

struct DropProbe(Arc<AtomicUsize>);

impl Drop for DropProbe {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn test_derive_struct_constructor_and_updater_are_descriptor_queryable() {
    let descriptor = Profile::type_descriptor();
    let construction = descriptor
        .struct_construction()
        .expect("derived structs expose construction metadata");
    assert!(std::ptr::eq(
        construction.local_constructor(),
        construction.local_constructor(),
    ));
    let value = descriptor
        .construct_struct(NamedConstructionInput::new([
            ("id", ReflectedOwned::new(7_u32)),
            ("label", ReflectedOwned::new(String::from("Ada"))),
        ]))
        .expect("validated inputs construct the derived struct")
        .downcast::<Profile>()
        .unwrap_or_else(|_| panic!("the adapter returns the declared root type"));
    assert_eq!(
        value,
        Profile {
            id: 7,
            label: String::from("Ada"),
        }
    );

    let updater = construction.local_updater().expect("derived structs expose updates");
    assert!(std::ptr::eq(
        updater,
        construction.local_updater().expect("derived structs cache updates"),
    ));
    let updated = updater
        .update(StructUpdateInput::new(
            ReflectedOwned::new(value),
            NamedConstructionInput::new([("label", ReflectedOwned::new(String::from("Grace")))]),
        ))
        .expect("validated overrides update the derived struct")
        .downcast::<Profile>()
        .unwrap_or_else(|_| panic!("the updater returns the declared root type"));
    assert_eq!(
        updated,
        Profile {
            id: 7,
            label: String::from("Grace"),
        }
    );
}

#[test]
fn test_derive_construction_honors_defaults_and_enum_shapes() {
    let defaults = Defaults::type_descriptor()
        .construct_struct(NamedConstructionInput::new([("id", ReflectedOwned::new(9_u32))]))
        .expect("an explicit default policy supplies an omitted field")
        .downcast::<Defaults>()
        .unwrap_or_else(|_| panic!("the adapter returns the declared root type"));
    assert_eq!(
        defaults,
        Defaults {
            id: 9,
            label: String::new(),
        }
    );

    let event_descriptor = Event::type_descriptor();
    assert!(event_descriptor.variants()[0].construction().is_some());
    let started = event_descriptor.variants()[0]
        .construct_unit()
        .expect("unit construction succeeds")
        .downcast::<Event>()
        .unwrap_or_else(|_| panic!("the variant adapter returns the enum root"));
    assert_eq!(started, Event::Started);

    let failed = event_descriptor.variants()[1]
        .construct_struct(NamedConstructionInput::new([
            ("code", ReflectedOwned::new(500_u32)),
            ("message", ReflectedOwned::new(String::from("failure"))),
        ]))
        .expect("named variant construction succeeds")
        .downcast::<Event>()
        .unwrap_or_else(|_| panic!("the variant adapter returns the enum root"));
    assert_eq!(
        failed,
        Event::Failed {
            code: 500,
            message: String::from("failure"),
        }
    );

    let defaulted = event_descriptor.variants()[2]
        .construct_struct(NamedConstructionInput::new([("id", ReflectedOwned::new(4_u32))]))
        .expect("variant field defaults are explicit and usable")
        .downcast::<Event>()
        .unwrap_or_else(|_| panic!("the variant adapter returns the enum root"));
    assert_eq!(
        defaulted,
        Event::Defaulted {
            id: 4,
            label: String::new(),
        }
    );

    let pair = event_descriptor.variants()[3]
        .construct_tuple(TupleConstructionInput::new([
            ReflectedOwned::new(3_u32),
            ReflectedOwned::new(String::from("three")),
        ]))
        .expect("tuple variant construction succeeds")
        .downcast::<Event>()
        .unwrap_or_else(|_| panic!("the variant adapter returns the enum root"));
    assert_eq!(pair, Event::Pair(3, String::from("three")));
}

#[test]
fn test_derive_no_construct_without_provider_recovers_every_input() {
    let result = Unavailable::type_descriptor().construct_struct(NamedConstructionInput::new([
        ("id", ReflectedOwned::new(11_u32)),
        ("secret", ReflectedOwned::new(String::from("unchanged"))),
    ]));
    let recovery = match result {
        Ok(_) => panic!("a no-construct field without a provider disables construction"),
        Err(recovery) => recovery,
    };
    assert!(matches!(recovery.error(), ConstructionError::Unavailable { .. }));
    let values = recovery.into_values();
    assert_eq!(values.len(), 2);
}

#[test]
fn test_derive_constructor_validation_failure_preserves_owned_drop_probe() {
    let drops = Arc::new(AtomicUsize::new(0));
    let result = Profile::type_descriptor().construct_struct(NamedConstructionInput::new([
        ("id", ReflectedOwned::new(DropProbe(Arc::clone(&drops)))),
        ("label", ReflectedOwned::new(String::from("unchanged"))),
    ]));
    let recovery = match result {
        Ok(_) => panic!("wrong exact field type must fail before construction"),
        Err(recovery) => recovery,
    };
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(recovery);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}
