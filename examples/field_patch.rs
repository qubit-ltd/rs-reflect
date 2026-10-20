// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::error::Error;

use qubit_reflect::Invocation;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::value::Local;

#[Reflect]
struct Customer {
    pub id: u64,
    pub email: String,
    pub credit_limit_cents: u64,
    #[reflect(skip)]
    internal_note: String,
}

#[ReflectImpl]
impl Customer {
    pub fn raise_credit_limit(&mut self, credit_limit_cents: u64) -> bool {
        if credit_limit_cents <= self.credit_limit_cents {
            return false;
        }
        self.credit_limit_cents = credit_limit_cents;
        true
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let descriptor = TypeDescriptor::of::<Customer>();
    let mut customer = Customer {
        id: 1001,
        email: String::from("ada@example.com"),
        credit_limit_cents: 50_000,
        internal_note: String::from("not reflected"),
    };

    let email = descriptor.field("email")?;
    if let Err(failure) = email.set(&mut customer, String::from("ada@corp.example")) {
        let (error, _rejected_email) = failure.into_parts();
        return Err(format!("email update failed: {error:?}").into());
    }
    let current: &String = email.get(&customer)?;
    assert_eq!(current, "ada@corp.example");
    assert!(descriptor.field("internal_note").is_err());
    assert_eq!(customer.internal_note, "not reflected");

    let output = match descriptor
        .method("raise_credit_limit")?
        .invoke(Invocation::<Local>::on_mut(&mut customer).arg(75_000_u64))
    {
        Ok(output) => output,
        Err(failure) => {
            let (error, _recovery) = failure.into_parts();
            return Err(format!("method invocation failed: {error:?}").into());
        }
    };
    let changed = output
        .into_owned::<bool>()
        .map_err(|_| std::io::Error::other("method returned a non-boolean output"))?;
    assert!(changed);
    assert_eq!(customer.credit_limit_cents, 75_000);
    Ok(())
}
