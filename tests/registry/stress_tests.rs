// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Deterministic concurrency coverage for frozen reflection contexts.

#![cfg(feature = "derive")]

use std::any::TypeId;
use std::sync::Arc;
use std::thread;

use qubit_reflect::{Reflect, ReflectionContext};

#[Reflect]
struct ConcurrentRoot {
    id: u32,
}

#[test]
fn concurrent_global_initialization_publishes_one_immutable_context() {
    let workers = 16;
    let barrier = Arc::new(std::sync::Barrier::new(workers));
    let handles: Vec<_> = (0..workers)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                let context = ReflectionContext::global()
                    .initialize()
                    .expect("the global context initializes successfully");
                let descriptor = context.type_of::<ConcurrentRoot>();
                assert_eq!(descriptor.type_id(), TypeId::of::<ConcurrentRoot>());
                assert_eq!(descriptor.field("id").expect("field exists").name(), "id");
                context as *const ReflectionContext as usize
            })
        })
        .collect();

    let addresses: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("context worker must finish"))
        .collect();
    assert!(addresses.iter().all(|address| *address == addresses[0]));
}
