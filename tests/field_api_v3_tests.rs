// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use std::any::Any;

use qubit_reflect::TypeDescriptor;
use qubit_reflect::access::FieldAccessError;
use qubit_reflect::value::DynamicMut;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::DynamicRef;
use qubit_reflect::value::Local;

mod restricted_visibility {
    #[qubit_reflect::Reflect]
    pub struct Fields {
        pub(super) parent_visible: u8,
        pub(crate) crate_visible: u8,
        private: u8,
    }

    pub fn fields() -> Fields {
        Fields {
            parent_visible: 1,
            crate_visible: 2,
            private: 3,
        }
    }
}

#[qubit_reflect::Reflect]
pub enum FieldVariant {
    Value { field: u32 },
    Empty,
}

#[qubit_reflect::Reflect]
pub struct BoxedAnyRecord {
    pub value: Box<dyn Any>,
}

#[qubit_reflect::Reflect]
pub struct PublicRecord {
    pub value: u32,
    secret: String,
    pub(crate) crate_value: u8,
    #[reflect(skip)]
    pub omitted: bool,
}

#[test]
fn public_field_uses_typed_references_and_owned_values() {
    let descriptor = TypeDescriptor::of::<PublicRecord>();
    let field = descriptor.field("value").expect("public field is described");
    let mut record = PublicRecord {
        value: 7,
        secret: String::from("hidden"),
        crate_value: 1,
        omitted: true,
    };

    let value: &u32 = field.get(&record).expect("exact shared field type");
    assert_eq!(*value, 7);
    *field.get_mut::<_, u32>(&mut record).expect("exact mutable field type") = 8;
    field.set(&mut record, 9_u32).expect("exact replacement type");
    assert_eq!(record.value, 9);
    assert!(descriptor.field("omitted").is_none());
    assert!(descriptor.field_at(3).is_none());
}

#[test]
fn dynamic_field_access_preserves_borrows_values_and_recovery() {
    let descriptor = TypeDescriptor::of::<PublicRecord>();
    let field = descriptor.field("value").expect("field exists");
    let mut record = PublicRecord {
        value: 21,
        secret: String::from("private"),
        crate_value: 1,
        omitted: false,
    };

    let borrowed = field
        .get_dynamic(DynamicRef::<Local>::new(&record))
        .expect("dynamic shared field access succeeds");
    assert_eq!(borrowed.downcast_ref::<u32>(), Some(&21));

    {
        let mut borrowed = field
            .get_mut_dynamic(DynamicMut::<Local>::new(&mut record))
            .expect("dynamic mutable field access succeeds");
        *borrowed.downcast_mut::<u32>().expect("exact field type") = 22;
    }

    field
        .set_dynamic(
            DynamicMut::<Local>::new(&mut record),
            DynamicOwned::<Local>::new(23_u32),
        )
        .expect("dynamic replacement succeeds");
    assert_eq!(record.value, 23);

    let failure = field
        .set_dynamic(
            DynamicMut::<Local>::new(&mut record),
            DynamicOwned::<Local>::new(String::from("recover me")),
        )
        .expect_err("wrong dynamic type is rejected");
    assert!(matches!(failure.error(), FieldAccessError::ValueTypeMismatch { .. }));
    let recovered: DynamicOwned<Local> = failure.into_recovery().unwrap().into_value();
    assert_eq!(
        recovered
            .downcast::<String>()
            .ok()
            .expect("the recovered dynamic type remains String"),
        "recover me"
    );

    let mut wrong_target = 9_u32;
    let failure = field
        .set_dynamic(
            DynamicMut::<Local>::new(&mut wrong_target),
            DynamicOwned::<Local>::new(24_u32),
        )
        .expect_err("wrong dynamic target is rejected");
    assert!(matches!(failure.error(), FieldAccessError::TargetTypeMismatch { .. }));
    assert_eq!(
        failure
            .into_recovery()
            .unwrap()
            .into_value()
            .downcast::<u32>()
            .ok()
            .expect("the recovered dynamic type remains u32"),
        24
    );
}

#[test]
fn dynamic_field_access_obeys_visibility_and_boxed_any_identity() {
    let record_descriptor = TypeDescriptor::of::<PublicRecord>();
    let secret = record_descriptor.field("secret").expect("private field exists");
    let mut record = PublicRecord {
        value: 1,
        secret: String::from("private"),
        crate_value: 1,
        omitted: false,
    };
    assert!(matches!(
        secret.get_dynamic(DynamicRef::<Local>::new(&record)),
        Err(FieldAccessError::NotPublic { .. })
    ));
    // SAFETY: the test owns the target and explicitly exercises visibility bypass.
    assert_eq!(
        unsafe { secret.get_dynamic_unsafe(DynamicRef::<Local>::new(&record)) }
            .unwrap()
            .downcast_ref::<String>()
            .unwrap(),
        "private"
    );
    assert!(matches!(
        secret.set_dynamic(
            DynamicMut::<Local>::new(&mut record),
            DynamicOwned::<Local>::new(String::from("denied")),
        ),
        Err(ref failure) if matches!(failure.error(), FieldAccessError::NotPublic { .. })
    ));
    // SAFETY: the test owns the target and explicitly exercises visibility bypass.
    unsafe {
        secret
            .set_dynamic_unsafe(
                DynamicMut::<Local>::new(&mut record),
                DynamicOwned::<Local>::new(String::from("allowed")),
            )
            .unwrap();
    }
    assert_eq!(record.secret, "allowed");

    let value = record_descriptor.field("value").unwrap();
    let boxed: Box<dyn Any> = Box::new(25_u32);
    value
        .set_dynamic(DynamicMut::<Local>::new(&mut record), DynamicOwned::<Local>::new(boxed))
        .expect("boxed dynamic input is checked by its inner type");
    assert_eq!(record.value, 25);

    let boxed_field = TypeDescriptor::of::<BoxedAnyRecord>().field("value").unwrap();
    let mut boxed_record = BoxedAnyRecord { value: Box::new(0_u32) };
    let replacement: Box<dyn Any> = Box::new(String::from("exact box"));
    boxed_field
        .set_dynamic(
            DynamicMut::<Local>::new(&mut boxed_record),
            DynamicOwned::<Local>::new(replacement),
        )
        .expect("a Box<dyn Any> field keeps the exact box value");
    assert_eq!(
        boxed_record.value.downcast::<String>().map(|value| *value).unwrap(),
        "exact box"
    );
}

#[test]
fn private_field_requires_unsafe_visibility_bypass_and_keeps_type_checks() {
    let descriptor = TypeDescriptor::of::<PublicRecord>();
    let field = descriptor.field("secret").expect("private field is described");
    let mut record = PublicRecord {
        value: 7,
        secret: String::from("hidden"),
        crate_value: 1,
        omitted: false,
    };

    assert!(matches!(
        field.get::<_, String>(&record),
        Err(FieldAccessError::NotPublic { .. })
    ));
    // SAFETY: this test owns the record and intentionally exercises the API's
    // visibility bypass.
    assert_eq!(unsafe { field.get_unsafe::<_, String>(&record) }.unwrap(), "hidden");
    // SAFETY: this test owns the record and intentionally exercises the API's
    // visibility bypass.
    *unsafe { field.get_mut_unsafe::<_, String>(&mut record) }.unwrap() = String::from("changed");
    // SAFETY: this test owns the record and intentionally exercises the API's
    // visibility bypass.
    unsafe { field.set_unsafe(&mut record, String::from("replaced")) }.unwrap();
    assert_eq!(record.secret, "replaced");
    // SAFETY: the test intentionally bypasses visibility to check that type
    // validation remains active.
    let mismatch =
        unsafe { field.get_unsafe::<_, u32>(&record) }.expect_err("unsafe access still checks the requested type");
    assert!(matches!(mismatch, FieldAccessError::FieldTypeMismatch { .. }));
    // SAFETY: the test intentionally bypasses visibility to check that set type
    // validation remains active.
    let failure =
        unsafe { field.set_unsafe(&mut record, 12_u32) }.expect_err("unsafe set still checks the replacement type");
    assert!(matches!(failure.error(), FieldAccessError::ValueTypeMismatch { .. }));
    assert_eq!(failure.into_recovery().unwrap().into_value(), 12_u32);
    assert_eq!(record.secret, "replaced");
    let field = descriptor.field("crate_value").expect("restricted field is described");
    assert!(matches!(
        field.get::<_, u8>(&record),
        Err(FieldAccessError::NotPublic { .. })
    ));
    // SAFETY: this test owns the record and intentionally bypasses source
    // visibility.
    assert_eq!(*unsafe { field.get_unsafe::<_, u8>(&record) }.unwrap(), 1);
}

#[test]
fn restricted_visibilities_are_not_available_through_safe_access() {
    let descriptor = TypeDescriptor::of::<restricted_visibility::Fields>();
    let record = restricted_visibility::fields();
    for (name, expected) in [("parent_visible", 1_u8), ("crate_visible", 2_u8), ("private", 3_u8)] {
        let field = descriptor.field(name).expect("restricted field is described");
        assert!(matches!(
            field.get::<_, u8>(&record),
            Err(FieldAccessError::NotPublic { .. })
        ));
        // SAFETY: this integration test owns the value and intentionally bypasses
        // reflection visibility.
        assert_eq!(*unsafe { field.get_unsafe::<_, u8>(&record) }.unwrap(), expected);
    }
}

#[test]
fn inactive_variant_set_rejects_without_consuming_the_value() {
    let descriptor = TypeDescriptor::of::<FieldVariant>();
    let field = descriptor
        .variant("Value")
        .and_then(|variant| variant.field("field"))
        .expect("variant field is described");
    let mut value = FieldVariant::Empty;
    let failure = field
        .set(&mut value, 42_u32)
        .expect_err("inactive variant cannot be updated");
    assert!(matches!(failure.error(), FieldAccessError::InactiveVariant { .. }));
    assert_eq!(failure.into_recovery().unwrap().into_value(), 42_u32);
    assert!(matches!(value, FieldVariant::Empty));
}

#[test]
fn set_failure_returns_the_original_typed_value() {
    let field = TypeDescriptor::of::<PublicRecord>()
        .field("value")
        .expect("field exists");
    let mut record = PublicRecord {
        value: 7,
        secret: String::new(),
        crate_value: 1,
        omitted: false,
    };
    let failure = field
        .set(&mut record, String::from("wrong type"))
        .expect_err("replacement has the wrong type");
    assert!(matches!(failure.error(), FieldAccessError::ValueTypeMismatch { .. }));
    let recovery: String = failure.into_recovery().expect("preflight retains input").into_value();
    assert_eq!(recovery, "wrong type");
    assert_eq!(record.value, 7);
    let mut wrong_target = 17_u32;
    let failure = field
        .set(&mut wrong_target, 11_u32)
        .expect_err("the target type is checked before mutation");
    assert!(matches!(failure.error(), FieldAccessError::TargetTypeMismatch { .. }));
    assert_eq!(failure.into_recovery().unwrap().into_value(), 11_u32);
    assert_eq!(wrong_target, 17);
}

#[test]
fn boxed_any_set_uses_the_inner_type_and_recovers_the_original_box() {
    let field = TypeDescriptor::of::<PublicRecord>()
        .field("value")
        .expect("field exists");
    let mut record = PublicRecord {
        value: 7,
        secret: String::new(),
        crate_value: 1,
        omitted: false,
    };

    let value: Box<dyn Any> = Box::new(12_u32);
    field
        .set(&mut record, value)
        .expect("the inner boxed type matches the field");
    assert_eq!(record.value, 12);

    let value: Box<dyn Any> = Box::new(String::from("keep me"));
    let failure = field
        .set(&mut record, value)
        .expect_err("the inner boxed type differs from the field");
    assert!(matches!(failure.error(), FieldAccessError::ValueTypeMismatch { .. }));
    let recovered: Box<dyn Any> = failure.into_recovery().unwrap().into_value();
    assert_eq!(recovered.downcast::<String>().map(|value| *value).unwrap(), "keep me");
    assert_eq!(record.value, 12);
}

#[test]
fn a_boxed_any_field_keeps_the_exact_box_type() {
    let field = TypeDescriptor::of::<BoxedAnyRecord>()
        .field("value")
        .expect("field exists");
    let mut record = BoxedAnyRecord { value: Box::new(1_u32) };
    let replacement: Box<dyn Any> = Box::new(String::from("boxed field"));
    field
        .set(&mut record, replacement)
        .expect("the field itself has exact Box<dyn Any> type");
    let value: &Box<dyn Any> = field.get(&record).expect("the boxed field keeps its exact type");
    assert_eq!(value.downcast_ref::<String>().map(String::as_str), Some("boxed field"));
    assert_eq!(
        record.value.downcast_ref::<String>().map(String::as_str),
        Some("boxed field")
    );
}
