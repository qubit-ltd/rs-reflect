// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;
use std::cell::Cell;
use std::rc::Rc;

use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::access::FieldAccessError;
use qubit_reflect::access::FieldSetError;

struct NotReflected;

#[Reflect]
pub struct Account {
    pub balance: u64,
    secret: String,
    #[reflect(skip)]
    // This member remains a Rust field but must not enter reflection metadata.
    #[allow(dead_code)]
    private_note: NotReflected,
}

struct OtherTarget;

struct DropProbe(Rc<Cell<usize>>);

impl Drop for DropProbe {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

#[test]
fn field_descriptor_keeps_declaring_type_and_source_identity() {
    let descriptor = TypeDescriptor::of::<Account>();
    let field = descriptor.fields().get("secret").expect("private field is described");

    assert_eq!(field.name(), "secret");
    assert_eq!(field.index(), 1);
    assert_eq!(field.declaring_type().type_id(), TypeId::of::<Account>());
    assert!(field.declaring_variant().is_none());
}

#[test]
fn public_field_uses_typed_borrows_and_owned_values() {
    let descriptor = TypeDescriptor::of::<Account>();
    let field = descriptor.field("balance").expect("public field is described");
    let mut account = Account {
        balance: 7,
        secret: String::from("token"),
        private_note: NotReflected,
    };

    let balance: &u64 = field.get(&account).expect("public shared access");
    assert_eq!(*balance, 7);
    *field.get_mut::<_, u64>(&mut account).expect("public mutable access") = 8;
    field.set(&mut account, 9_u64).expect("public typed replacement");
    assert_eq!(account.balance, 9);
}

#[test]
fn private_field_requires_unsafe_visibility_bypass() {
    let descriptor = TypeDescriptor::of::<Account>();
    let field = descriptor.field("secret").expect("private field is described");
    let mut account = Account {
        balance: 7,
        secret: String::from("token"),
        private_note: NotReflected,
    };

    assert!(matches!(
        field.get::<_, String>(&account),
        Err(FieldAccessError::NotPublic)
    ));
    // SAFETY: the test owns the account and deliberately bypasses reflection
    // visibility.
    assert_eq!(unsafe { field.get_unsafe::<_, String>(&account) }.unwrap(), "token");
    // SAFETY: the test owns the account and deliberately bypasses reflection
    // visibility.
    *unsafe { field.get_mut_unsafe::<_, String>(&mut account) }.unwrap() = String::from("changed");
    // SAFETY: the replacement keeps the field's declared String type.
    unsafe { field.set_unsafe(&mut account, String::from("replaced")) }.unwrap();
    assert_eq!(account.secret, "replaced");

    // SAFETY: the test owns the account; this checks that bypassing visibility does
    // not bypass field-type validation.
    let error = unsafe { field.get_unsafe::<_, u32>(&account) }
        .expect_err("the requested type must match the field's declared type");
    assert!(matches!(error, FieldAccessError::FieldTypeMismatch { .. }));
}

#[test]
fn failures_recover_typed_replacements_before_mutation() {
    let field = TypeDescriptor::of::<Account>().field("balance").expect("field exists");
    let mut account = Account {
        balance: 7,
        secret: String::new(),
        private_note: NotReflected,
    };

    let failure = field
        .set(&mut account, String::from("wrong type"))
        .expect_err("replacement type must match the field");
    assert!(matches!(failure.error(), FieldSetError::ValueTypeMismatch(_)));
    let recovered: String = failure.into_parts().1;
    assert_eq!(recovered, "wrong type");
    assert_eq!(account.balance, 7);

    let mut wrong_target = 17_u32;
    let failure = field
        .set(&mut wrong_target, 11_u64)
        .expect_err("target type must match the declaring type");
    assert!(matches!(failure.error(), FieldSetError::TargetTypeMismatch(_)));
    assert_eq!(failure.into_parts().1, 11_u64);
    assert_eq!(wrong_target, 17);

    let other = OtherTarget;
    let error = field
        .get::<_, u64>(&other)
        .expect_err("target type must match the declaring type");
    let FieldAccessError::TargetTypeMismatch(mismatch) = error
    else {
        panic!("the error must retain target mismatch context");
    };
    assert_eq!(mismatch.expected(), TypeId::of::<Account>());
    assert_eq!(mismatch.actual(), TypeId::of::<OtherTarget>());

    let field = TypeDescriptor::of::<Account>().field("balance").expect("field exists");
    let mut account = Account {
        balance: 7,
        secret: String::new(),
        private_note: NotReflected,
    };
    let failure = field
        .set(&mut account, 12_u32)
        .expect_err("the replacement type must match exactly");
    let FieldSetError::ValueTypeMismatch(mismatch) = failure.error()
    else {
        panic!("the error must classify a replacement type mismatch");
    };
    assert_eq!(mismatch.expected(), TypeId::of::<u64>());
    assert_eq!(mismatch.actual(), TypeId::of::<u32>());
    assert_eq!(account.balance, 7);
}

#[test]
fn failed_set_retains_the_exact_value_without_dropping_or_duplicating_it() {
    let field = TypeDescriptor::of::<Account>().field("balance").expect("field exists");
    let mut account = Account {
        balance: 7,
        secret: String::new(),
        private_note: NotReflected,
    };
    let drops = Rc::new(Cell::new(0));
    let failure = field
        .set(&mut account, DropProbe(Rc::clone(&drops)))
        .expect_err("the replacement type does not match the field");

    assert!(matches!(failure.error(), FieldSetError::ValueTypeMismatch(_)));
    assert_eq!(drops.get(), 0, "validation must leave ownership with recovery");
    let recovered = failure.into_parts().1;
    assert_eq!(drops.get(), 0, "taking recovery must not drop the value");
    drop(recovered);
    assert_eq!(drops.get(), 1, "the caller must control the single final drop");
    assert_eq!(account.balance, 7);
}

#[test]
fn typed_failure_exposes_the_exact_replacement_value() {
    let field = TypeDescriptor::of::<Account>().field("balance").expect("field exists");
    let mut account = Account {
        balance: 7,
        secret: String::new(),
        private_note: NotReflected,
    };
    let failure = field
        .set(&mut account, 12_u32)
        .expect_err("the exact field type is u64");

    assert!(matches!(failure.error(), FieldSetError::ValueTypeMismatch(_)));
    assert_eq!(*failure.replacement(), 12_u32);
    let (_, replacement) = failure.into_parts();
    assert_eq!(replacement, 12_u32);
    assert_eq!(account.balance, 7);
}
