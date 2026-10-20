// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::Invocation;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::value::Local;

#[Reflect]
struct Customer {
    pub id: u64,
    pub suspended: bool,
    pub suspension_reason: Option<String>,
}

#[ReflectImpl]
impl Customer {
    pub fn suspend(&mut self, reason: String) -> bool {
        if self.suspended {
            return false;
        }
        self.suspended = true;
        self.suspension_reason = Some(reason);
        true
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut customer = Customer {
        id: 1001,
        suspended: false,
        suspension_reason: None,
    };

    let output = match TypeDescriptor::of::<Customer>()
        .method("suspend")?
        .invoke(Invocation::<Local>::on_mut(&mut customer).arg(String::from("chargeback dispute")))
    {
        Ok(output) => output,
        Err(failure) => {
            let (error, _recovery) = failure.into_parts();
            return Err(format!("suspension failed: {error:?}").into());
        }
    };
    let changed = output
        .into_owned::<bool>()
        .map_err(|_| std::io::Error::other("method returned a non-boolean output"))?;
    assert!(changed);
    assert!(customer.suspended);
    assert_eq!(customer.suspension_reason.as_deref(), Some("chargeback dispute"));
    Ok(())
}
