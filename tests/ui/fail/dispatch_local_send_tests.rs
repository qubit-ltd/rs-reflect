// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::rc::Rc;

use qubit_reflect::descriptor::InvocationAdapter;
use qubit_reflect::invoke::Invocation;
use qubit_reflect::invoke::InvocationArg;
use qubit_reflect::registry::RegistrySnapshotBuilder;
use qubit_reflect::ReflectedOwned;

fn opaque_entry() {}

fn main() {
    let registry = RegistrySnapshotBuilder::new().build().unwrap();
    let input = Invocation::associated([InvocationArg::Owned(ReflectedOwned::new(Rc::new(7_u8)))]);
    let Err(unavailable) = InvocationAdapter::new(opaque_entry).invoke_local(&registry, input) else {
        panic!("opaque entry");
    };
    std::thread::spawn(move || drop(unavailable));
}
