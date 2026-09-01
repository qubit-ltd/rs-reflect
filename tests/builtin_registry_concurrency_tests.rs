// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow explicit-imports
//! Isolated-process coverage for concurrent first registry initialization.

use std::sync::Arc;
use std::sync::Barrier;

use qubit_reflect::registry::ReflectRegistry;

/// Verifies concurrent first initialization publishes one immutable snapshot.
#[test]
fn test_builtin_registry_concurrently_freezes_once() {
    const THREAD_COUNT: usize = 32;

    let barrier = Arc::new(Barrier::new(THREAD_COUNT));
    let handles: Vec<_> = (0..THREAD_COUNT)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                let registry = ReflectRegistry::initialize()
                    .expect("concurrent initialization must succeed");
                let identities: Vec<_> = registry
                    .types()
                    .iter()
                    .map(|descriptor| descriptor.type_id())
                    .collect();
                (registry as *const ReflectRegistry as usize, identities)
            })
        })
        .collect();
    let snapshots: Vec<_> = handles
        .into_iter()
        .map(|handle| {
            handle
                .join()
                .expect("registry initialization thread must finish")
        })
        .collect();

    assert!(snapshots.windows(2).all(|pair| pair[0] == pair[1]));
    assert_eq!(snapshots[0].1.len(), 20);
}
