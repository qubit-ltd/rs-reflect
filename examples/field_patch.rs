// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// reflect-example-start
use std::error::Error;
use std::io;

use qubit_reflect::Invocation;
use qubit_reflect::InvocationOutput;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectRegistry;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::access::FieldAccessError;
use qubit_reflect::descriptor::MethodLookup;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;

#[Reflect]
struct Customer {
    pub id: u64,
    pub email: String,
    pub credit_limit_cents: u64,
    secret: String,
    #[reflect(skip)]
    internal_note: String,
}

#[ReflectImpl]
impl Customer {
    /// Raises the credit limit, returning false when the requested limit is not
    /// greater than the current one.
    pub fn raise_credit_limit(&mut self, credit_limit_cents: u64) -> bool {
        if credit_limit_cents <= self.credit_limit_cents {
            return false;
        }
        self.credit_limit_cents = credit_limit_cents;
        true
    }
}

/// Runs the example, returning reflection setup and operation failures to the
/// caller.
fn main() -> Result<(), Box<dyn Error>> {
    let registry = ReflectRegistry::initialize()?;
    let descriptor = TypeDescriptor::of::<Customer>();
    let mut customer = Customer {
        id: 1001,
        email: String::from("ada@example.com"),
        credit_limit_cents: 50_000,
        secret: String::from("private"),
        internal_note: String::from("not reflected"),
    };

    let email_field = descriptor
        .field("email")
        .ok_or(io::Error::other("missing reflected email field"))?;
    email_field.set(&mut customer, String::from("ada@corp.example"))?;
    assert_eq!(customer.email, "ada@corp.example");

    let current: &String = email_field.get(&customer)?;
    assert_eq!(current, "ada@corp.example");

    let secret_field = descriptor
        .field("secret")
        .ok_or(io::Error::other("missing reflected secret field"))?;
    assert!(matches!(
        secret_field.get::<_, String>(&customer),
        Err(FieldAccessError::NotPublic { .. })
    ));
    // SAFETY: this example owns Customer and intentionally demonstrates the
    // visibility bypass.
    assert_eq!(unsafe { secret_field.get_unsafe::<_, String>(&customer) }?, "private");
    // SAFETY: this example owns Customer and maintains the String field's type
    // invariant.
    unsafe { secret_field.set_unsafe(&mut customer, String::from("updated")) }?;
    assert_eq!(customer.secret, "updated");
    assert!(descriptor.field("internal_note").is_none());
    assert_eq!(customer.internal_note, "not reflected");

    let MethodLookup::Unique(raise_credit_limit) = descriptor.methods_named_in(registry, "raise_credit_limit") else {
        return Err(io::Error::other("expected exactly one method named raise_credit_limit").into());
    };
    {
        let invocation = Invocation::receiver_mut(&mut customer).arg(75_000_u64);
        let output = raise_credit_limit
            .invoke(registry, invocation)
            .map_err(|failure| io::Error::other(failure.to_string()))?;
        let InvocationOutput::Owned(changed) = output else {
            return Err(io::Error::other("raise_credit_limit did not return an owned value").into());
        };
        let changed = DynamicOwned::<Local>::downcast::<bool>(changed)
            .map_err(|_| io::Error::other("raise_credit_limit did not return bool"))?;
        assert!(changed);
    }
    assert_eq!(customer.credit_limit_cents, 75_000);
    Ok(())
}
// reflect-example-end
