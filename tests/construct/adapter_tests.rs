// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::NamedConstructionInput;
use qubit_reflect::Reflect;
use qubit_reflect::StructUpdateInput;
use qubit_reflect::TupleConstructionInput;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::construct::ConstructionError;
use qubit_reflect::construct::ConstructionShape;

#[Reflect]
#[derive(Debug, Eq, PartialEq)]
pub struct Profile {
    pub id: u32,
    pub label: String,
}

#[Reflect]
#[derive(Debug, Eq, PartialEq)]
pub struct Pair(pub u32, pub String);

#[Reflect]
#[derive(Debug, Eq, PartialEq)]
pub enum Event {
    Started,
    Failed { pub_code: u32, pub_message: String },
    Pair(u32, String),
}

#[Reflect]
#[derive(Debug)]
pub struct Restricted {
    pub id: u32,
    secret: String,
}

#[Reflect]
#[derive(Debug)]
pub struct OptionalValue {
    pub value: Option<u32>,
}

#[Reflect]
pub struct Unit;

#[test]
fn named_and_tuple_structs_construct_from_plain_values() {
    let profile: Profile = TypeDescriptor::of::<Profile>()
        .construct_struct(
            NamedConstructionInput::new()
                .field("id", 7_u32)
                .field("label", String::from("Ada")),
        )
        .expect("named construction accepts ordinary values");
    assert_eq!(
        profile,
        Profile {
            id: 7,
            label: String::from("Ada")
        }
    );

    let pair: Pair = TypeDescriptor::of::<Pair>()
        .construct_tuple(TupleConstructionInput::new().value(3_u32).value(String::from("three")))
        .expect("tuple construction accepts ordinary values");
    assert_eq!(pair, Pair(3, String::from("three")));
}

#[test]
fn local_constructor_direct_entry_points_return_the_declared_shapes() {
    let profile_constructor = TypeDescriptor::of::<Profile>()
        .struct_construction()
        .expect("profile construction metadata")
        .local_constructor();
    assert_eq!(profile_constructor.shape(), ConstructionShape::Named);
    let profile = profile_constructor
        .construct_named(
            NamedConstructionInput::new()
                .field("id", 4_u32)
                .field("label", String::from("direct")),
        )
        .unwrap_or_else(|_| panic!("direct named constructor accepts matching fields"))
        .into_any()
        .unwrap_or_else(|_| panic!("constructor produces an Any value"))
        .downcast::<Profile>()
        .unwrap_or_else(|_| panic!("constructor produces its exact reflected root"));
    assert_eq!(
        *profile,
        Profile {
            id: 4,
            label: String::from("direct")
        }
    );

    let pair_constructor = TypeDescriptor::of::<Pair>()
        .struct_construction()
        .expect("tuple construction metadata")
        .local_constructor();
    assert_eq!(pair_constructor.shape(), ConstructionShape::Tuple);
    let pair = pair_constructor
        .construct_tuple(TupleConstructionInput::new().value(5_u32).value(String::from("five")))
        .unwrap_or_else(|_| panic!("direct tuple constructor accepts matching positions"))
        .into_any()
        .unwrap_or_else(|_| panic!("tuple constructor produces an Any value"))
        .downcast::<Pair>()
        .unwrap_or_else(|_| panic!("tuple constructor produces its exact reflected root"));
    assert_eq!(*pair, Pair(5, String::from("five")));

    let unit_constructor = TypeDescriptor::of::<Unit>()
        .struct_construction()
        .expect("unit construction metadata")
        .local_constructor();
    assert_eq!(unit_constructor.shape(), ConstructionShape::Unit);
    assert!(
        unit_constructor
            .construct_unit()
            .expect("direct unit constructor accepts empty input")
            .downcast_ref::<Unit>()
            .is_some()
    );
}

#[test]
fn enum_variants_construct_from_plain_values() {
    let descriptor = TypeDescriptor::of::<Event>();
    let started: Event = descriptor
        .variant("Started")
        .unwrap()
        .construct_unit()
        .expect("unit variant construction");
    assert!(matches!(started, Event::Started));

    let failed: Event = descriptor
        .variant("Failed")
        .unwrap()
        .construct_struct(
            NamedConstructionInput::new()
                .field("pub_code", 500_u32)
                .field("pub_message", String::from("failure")),
        )
        .expect("named variant construction");
    assert!(matches!(failed, Event::Failed { pub_code: 500, .. }));

    let pair: Event = descriptor
        .variant("Pair")
        .unwrap()
        .construct_tuple(TupleConstructionInput::new().value(8_u32).value(String::from("eight")))
        .expect("tuple variant construction");
    assert!(matches!(pair, Event::Pair(8, _)));
}

#[test]
fn option_fields_are_required_without_an_explicit_default() {
    let failure = TypeDescriptor::of::<OptionalValue>()
        .construct_struct::<OptionalValue>(NamedConstructionInput::new())
        .expect_err("Option fields are not implicitly defaulted");
    assert!(
        matches!(failure.error(), ConstructionError::MissingField { field } if field.query_name() == Some("value"))
    );
    assert!(failure.into_any_parts().1.is_empty());

    let unit: Unit = TypeDescriptor::of::<Unit>()
        .construct_unit()
        .expect("unit struct construction takes no values");
    let _ = unit;
}

#[test]
fn private_fields_require_unsafe_construction_and_update() {
    let descriptor = TypeDescriptor::of::<Restricted>();
    let input = NamedConstructionInput::new()
        .field("id", 1_u32)
        .field("secret", String::from("hidden"));
    let failure = descriptor
        .construct_struct::<Restricted>(input)
        .expect_err("safe construction cannot initialize private fields");
    assert!(matches!(failure.error(), ConstructionError::VisibilityDenied { .. }));

    // SAFETY: the input initializes both fields with their declared types.
    let value: Restricted = unsafe {
        descriptor.construct_struct_unsafe(
            NamedConstructionInput::new()
                .field("id", 1_u32)
                .field("secret", String::from("hidden")),
        )
    }
    .expect("explicit visibility bypass");
    assert_eq!(value.secret, "hidden");

    // SAFETY: this test owns the value and replaces the private field with a
    // String.
    let updated: Restricted = unsafe {
        descriptor.update_struct_unsafe(StructUpdateInput::new(
            value,
            NamedConstructionInput::new().field("secret", String::from("changed")),
        ))
    }
    .expect("explicit update visibility bypass");
    assert_eq!(updated.secret, "changed");
}

#[test]
fn struct_construction_descriptor_exposes_cached_local_adapters() {
    let local = TypeDescriptor::of::<Profile>()
        .struct_construction()
        .expect("derived profile has construction adapters");
    let constructor = local.local_constructor();
    let updater = local.local_updater().expect("profile supports updates");
    assert_eq!(
        constructor.descriptor().type_id(),
        TypeDescriptor::of::<Profile>().type_id()
    );
    assert_eq!(constructor.fields().len(), 2);
    assert_eq!(constructor.shape().to_string(), "named");
    assert_eq!(
        updater.descriptor().type_id(),
        TypeDescriptor::of::<Profile>().type_id()
    );
    assert_eq!(updater.fields().len(), 2);
    assert!(format!("{constructor:?}").contains("StructConstructor"));
    assert!(format!("{updater:?}").contains("StructUpdater"));
}

#[test]
fn dynamic_updater_returns_the_plain_updated_value() {
    let descriptor = TypeDescriptor::of::<Profile>();
    let updater = descriptor
        .struct_construction()
        .unwrap()
        .local_updater()
        .expect("profile supports updates");
    let updated = updater
        .update_any(StructUpdateInput::new(
            Profile {
                id: 5,
                label: String::from("before"),
            },
            qubit_reflect::NamedConstructionInput::new().field("label", String::from("after")),
        ))
        .expect("valid dynamic update")
        .downcast::<Profile>()
        .expect("the dynamic result retains its exact root type");
    assert_eq!(updated.id, 5);
    assert_eq!(updated.label, "after");
}

#[test]
fn updater_typed_entry_points_preserve_unmodified_and_private_fields() {
    let updater = TypeDescriptor::of::<Profile>()
        .struct_construction()
        .expect("profile has construction metadata")
        .local_updater()
        .expect("profile supports updates");
    let safe = updater
        .update::<Profile>(StructUpdateInput::new(
            Profile {
                id: 10,
                label: String::from("retained"),
            },
            NamedConstructionInput::new().field("id", 11_u32),
        ))
        .expect("safe typed update accepts visible-field replacement");
    assert_eq!(safe.id, 11);
    assert_eq!(safe.label, "retained");

    let restricted_updater = TypeDescriptor::of::<Restricted>()
        .struct_construction()
        .expect("restricted struct has construction metadata")
        .local_updater()
        .expect("restricted struct supports updates");

    // SAFETY: the replacement preserves the private field's String type.
    let unsafe_updated = unsafe {
        restricted_updater.update_unsafe::<Restricted>(StructUpdateInput::new(
            Restricted {
                id: safe.id,
                secret: safe.label,
            },
            NamedConstructionInput::new().field("secret", String::from("updated")),
        ))
    }
    .expect("unsafe typed update explicitly permits private replacement");
    assert_eq!(unsafe_updated.id, 11);
    assert_eq!(unsafe_updated.secret, "updated");
}

#[test]
fn updater_rejects_wrong_typed_targets_and_unsafe_dynamic_update_preserves_private_values() {
    let descriptor = TypeDescriptor::of::<Restricted>();
    let updater = descriptor
        .struct_construction()
        .expect("restricted struct has construction metadata")
        .local_updater()
        .expect("restricted struct supports updates");

    let failure = updater
        .update::<String>(StructUpdateInput::new(
            Restricted {
                id: 1,
                secret: String::from("before"),
            },
            NamedConstructionInput::new().field("id", 2_u32),
        ))
        .expect_err("typed updater must reject a result type different from its root");
    assert!(matches!(failure.error(), ConstructionError::TargetTypeMismatch { .. }));
    let (_, recovered) = failure.into_any_parts();
    let [
        qubit_reflect::construct::RecoveredAnyValue::Base(base),
        qubit_reflect::construct::RecoveredAnyValue::Named { name, value },
    ] = recovered.as_ref()
    else {
        panic!("target type failure must retain base and override");
    };
    assert_eq!(base.downcast_ref::<Restricted>().map(|value| value.id), Some(1));
    assert_eq!(name.as_ref(), "id");
    assert_eq!(value.downcast_ref::<u32>(), Some(&2));

    // SAFETY: the override keeps Restricted's private field as a String.
    let updated = unsafe {
        updater.update_any_unsafe(StructUpdateInput::new(
            Restricted {
                id: 3,
                secret: String::from("before"),
            },
            NamedConstructionInput::new().field("secret", String::from("after")),
        ))
    }
    .expect("unsafe dynamic update validates exact field types")
    .downcast::<Restricted>()
    .expect("dynamic updater returns the reflected root");
    assert_eq!(updated.id, 3);
    assert_eq!(updated.secret, "after");
}

#[test]
fn struct_constructor_rejects_each_incompatible_input_shape() {
    let profile = TypeDescriptor::of::<Profile>()
        .struct_construction()
        .expect("profile has a constructor")
        .local_constructor();
    let tuple_failure = profile
        .construct_tuple(TupleConstructionInput::new().value(7_u32))
        .err()
        .expect("named struct cannot consume tuple input");
    assert!(matches!(tuple_failure.error(), ConstructionError::WrongShape { .. }));
    assert_eq!(tuple_failure.into_any_parts().1.len(), 1);
    let unit_failure = profile
        .construct_unit()
        .err()
        .expect("named struct cannot consume unit input");
    assert!(matches!(unit_failure.error(), ConstructionError::WrongShape { .. }));
    assert!(unit_failure.into_any_parts().1.is_empty());

    let unit = TypeDescriptor::of::<Unit>()
        .struct_construction()
        .expect("unit struct has a constructor")
        .local_constructor();
    let named_failure = unit
        .construct_named(NamedConstructionInput::new().field("extra", 1_u32))
        .err()
        .expect("unit struct cannot consume named input");
    assert!(matches!(named_failure.error(), ConstructionError::WrongShape { .. }));
    assert_eq!(named_failure.into_any_parts().1.len(), 1);
    let tuple_failure = unit
        .construct_tuple(TupleConstructionInput::new().value(1_u32))
        .err()
        .expect("unit struct cannot consume tuple input");
    assert!(matches!(tuple_failure.error(), ConstructionError::WrongShape { .. }));
    assert_eq!(tuple_failure.into_any_parts().1.len(), 1);
}

#[test]
fn variant_constructor_metadata_and_wrong_shape_recovery_are_stable() {
    let variant = TypeDescriptor::of::<Event>()
        .variant("Pair")
        .expect("tuple variant exists");
    let construction = variant.construction().expect("variant is constructible");
    let constructor = construction.local_constructor();

    assert_eq!(constructor.variant().rust_name(), "Pair");
    assert_eq!(constructor.shape(), ConstructionShape::Tuple);
    assert_eq!(constructor.fields().len(), 2);
    assert!(format!("{constructor:?}").contains("VariantConstructor"));
    let failure = variant
        .construct_struct::<Event>(NamedConstructionInput::new().field("value", 9_u32))
        .expect_err("a tuple variant rejects named construction input");
    let ConstructionError::WrongShape { expected, actual } = failure.error() else {
        panic!("wrong variant input shape must report a structured shape error");
    };
    assert_eq!(
        (*expected, *actual),
        (ConstructionShape::Tuple, ConstructionShape::Named)
    );
    let (_, recovered) = failure.into_any_parts();
    let [qubit_reflect::construct::RecoveredAnyValue::Named { name, value }] = recovered.as_ref() else {
        panic!("wrong-shape recovery must retain the named input");
    };
    assert_eq!(name.as_ref(), "value");
    assert_eq!(value.downcast_ref::<u32>(), Some(&9));

    assert!(
        variant
            .is_active(&Event::Pair(1, String::from("active")))
            .expect("exact enum root")
    );
    assert!(!variant.is_active(&Event::Started).expect("inactive sibling variant"));
    assert!(
        variant.is_active(&17_u32).is_err(),
        "a different root type must be rejected"
    );
}

#[test]
fn variant_constructor_unit_entry_point_recovers_wrong_shape_without_values() {
    let variant = TypeDescriptor::of::<Event>()
        .variant("Failed")
        .expect("named variant exists");
    let failure = variant
        .construct_unit::<Event>()
        .expect_err("named variant cannot be constructed as unit");
    let ConstructionError::WrongShape { expected, actual } = failure.error() else {
        panic!("unit entry point must report a shape mismatch");
    };
    assert_eq!(
        (*expected, *actual),
        (ConstructionShape::Named, ConstructionShape::Unit)
    );
    assert!(failure.into_any_parts().1.is_empty());
}

#[test]
fn variant_descriptor_dynamic_construction_and_wrong_root_recover_inputs() {
    let variant = TypeDescriptor::of::<Event>()
        .variant("Pair")
        .expect("tuple variant exists");
    let value = variant
        .construct_tuple_any(TupleConstructionInput::new().value(6_u32).value(String::from("six")))
        .expect("dynamic variant construction accepts matching inputs")
        .downcast::<Event>()
        .unwrap_or_else(|_| panic!("dynamic variant constructor returns its declaring enum"));
    assert!(matches!(*value, Event::Pair(6, ref text) if text == "six"));

    let failure = variant
        .construct_tuple::<String>(TupleConstructionInput::new().value(7_u32).value(String::from("seven")))
        .expect_err("typed variant construction rejects a non-enum target type");
    assert!(matches!(failure.error(), ConstructionError::TargetTypeMismatch { .. }));
    let (_, recovered) = failure.into_any_parts();
    let [
        qubit_reflect::construct::RecoveredAnyValue::Positional { index: 0, value: first },
        qubit_reflect::construct::RecoveredAnyValue::Positional {
            index: 1,
            value: second,
        },
    ] = recovered.as_ref()
    else {
        panic!("wrong-root recovery keeps positional inputs in order");
    };
    assert_eq!(first.downcast_ref::<u32>(), Some(&7));
    assert_eq!(second.downcast_ref::<String>().map(String::as_str), Some("seven"));
}
