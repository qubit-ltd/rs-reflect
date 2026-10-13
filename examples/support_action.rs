// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// reflect-example-start
use qubit_reflect::Invocation;
use qubit_reflect::InvocationOutput;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectRegistry;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::MethodLookup;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;

#[Reflect]
struct Customer {
    pub id: u64,
    pub suspended: bool,
    pub suspension_reason: Option<String>,
}

#[ReflectImpl]
impl Customer {
    /// Suspends this customer with `reason`, returning false if already
    /// suspended.
    fn suspend(&mut self, reason: String) -> bool {
        if self.suspended {
            return false;
        }
        self.suspended = true;
        self.suspension_reason = Some(reason);
        true
    }
}

/// Runs the example and panics if a business assertion or reflection operation
/// fails.
fn main() {
    let registry = ReflectRegistry::initialize().expect("valid reflection declarations");

    let MethodLookup::Unique(suspend) = TypeDescriptor::of::<Customer>().methods_named_in(registry, "suspend") else {
        panic!("exactly one method named suspend")
    };

    let mut customer = Customer {
        id: 1001,
        suspended: false,
        suspension_reason: None,
    };
    {
        let invocation = Invocation::receiver_mut(&mut customer).arg(String::from("chargeback dispute"));
        let output = suspend
            .invoke(registry, invocation)
            .expect("valid receiver and arguments");
        let InvocationOutput::Owned(changed) = output else {
            panic!("owned output")
        };
        assert!(DynamicOwned::<Local>::downcast::<bool>(changed).unwrap_or_else(|_| panic!("bool")));
    }
    assert!(customer.suspended);
    assert_eq!(customer.suspension_reason.as_deref(), Some("chargeback dispute"));
}
// reflect-example-end
