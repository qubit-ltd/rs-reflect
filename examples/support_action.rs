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
use qubit_reflect::ReflectRegistry;
use qubit_reflect::ReflectedMut;
use qubit_reflect::ReflectedOwned;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::MethodLookup;
use qubit_reflect::invoke::InvocationArg;
use qubit_reflect::invoke::InvocationDispatchReason;
use qubit_reflect::reflect_impl;

#[derive(Reflect)]
#[reflect(crate = qubit_reflect)]
struct Customer {
    id: u64,
    suspended: bool,
    suspension_reason: Option<String>,
}

#[reflect_impl(crate = qubit_reflect)]
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
        let invocation = Invocation::borrowed_mut(
            ReflectedMut::new(&mut customer),
            [InvocationArg::Owned(ReflectedOwned::new(String::from(
                "chargeback dispute",
            )))],
        );
        let Err(unavailable) = suspend.invoke_catching_local(registry, invocation) else {
            panic!("panic capture was not requested for this method")
        };
        assert!(matches!(
            unavailable.reason(),
            InvocationDispatchReason::CatchingNotRequested
        ));
        let invocation = unavailable.into_invocation();
        let output = suspend
            .invoke_local(registry, invocation)
            .expect("local invocation entry is available")
            .expect("valid receiver and arguments");
        let InvocationOutput::Owned(changed) = output else {
            panic!("owned output")
        };
        assert!(changed.downcast::<bool>().unwrap_or_else(|_| panic!("bool")));
    }
    assert!(customer.suspended);
    assert_eq!(customer.suspension_reason.as_deref(), Some("chargeback dispute"));
}
// reflect-example-end
