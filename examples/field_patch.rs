// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// reflect-example-start
use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::access::FieldAccessError;

#[Reflect]
struct Customer {
    pub id: u64,
    pub email: String,
    pub credit_limit_cents: u64,
    secret: String,
    #[reflect(skip)]
    internal_note: String,
}

/// Runs the example and panics if a business assertion or reflection operation
/// fails.
fn main() {
    let descriptor = TypeDescriptor::of::<Customer>();
    let mut customer = Customer {
        id: 1001,
        email: String::from("ada@example.com"),
        credit_limit_cents: 50_000,
        secret: String::from("private"),
        internal_note: String::from("not reflected"),
    };

    let email = descriptor.field("email").expect("derived field");
    email
        .set(&mut customer, String::from("ada@corp.example"))
        .expect("exactly typed replacement");
    assert_eq!(customer.email, "ada@corp.example");

    let current: &String = email.get(&customer).expect("shared read");
    assert_eq!(current, "ada@corp.example");

    let secret = descriptor.field("secret").expect("private field is described");
    assert!(matches!(
        secret.get::<_, String>(&customer),
        Err(FieldAccessError::NotPublic { .. })
    ));
    // SAFETY: this example owns Customer and intentionally demonstrates the
    // visibility bypass.
    assert_eq!(
        unsafe { secret.get_unsafe::<_, String>(&customer) }.expect("exact field type"),
        "private"
    );
    // SAFETY: this example owns Customer and maintains the String field's type
    // invariant.
    unsafe { secret.set_unsafe(&mut customer, String::from("updated")) }.expect("exact replacement type");
    assert_eq!(customer.secret, "updated");
    assert!(descriptor.field("internal_note").is_none());
    assert_eq!(customer.internal_note, "not reflected");
}
// reflect-example-end
