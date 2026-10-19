// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Isolated-process coverage for concurrent global context initialization.

use std::sync::{Arc, Barrier};
use std::thread;

use qubit_reflect::ReflectionContext;

#[test]
fn global_context_concurrently_freezes_once() {
    let workers = 16;
    let barrier = Arc::new(Barrier::new(workers));
    let handles: Vec<_> = (0..workers)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                let context = ReflectionContext::global()
                    .initialize()
                    .expect("linked registrations must initialize");
                (context as *const ReflectionContext as usize, context.id())
            })
        })
        .collect();

    let observations: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("context worker must finish"))
        .collect();
    assert!(observations.iter().all(|seen| *seen == observations[0]));
}
