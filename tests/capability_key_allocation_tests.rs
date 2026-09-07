// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Allocation regression coverage for typed capability lookups.

use std::alloc::GlobalAlloc;
use std::alloc::Layout;
use std::alloc::System;
use std::cell::Cell;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use qubit_reflect::capability::CapabilityDescriptor;
use qubit_reflect::capability::CapabilityKey;
use qubit_reflect::capability::TypeCapabilities;
use qubit_reflect::identity::CapabilityId;

struct CountingAllocator;

thread_local! {
    // Const TLS initialization does not allocate or run a destructor.
    static ALLOCATIONS: Cell<Option<usize>> = const { Cell::new(None) };
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = ALLOCATIONS.try_with(|count| {
            if let Some(current) = count.get() {
                count.set(Some(current + 1));
            }
        });
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// Proves at compile time that `T` is copyable.
fn assert_copy<T: Copy>() {}

/// Verifies key construction and typed lookup perform no heap allocation.
#[test]
fn test_capability_key_lookup_is_allocation_free() {
    assert_copy::<CapabilityId>();
    assert_copy::<CapabilityKey<u32>>();

    let id = CapabilityId::new("example.allocation_free").expect("valid capability ID");
    let key = CapabilityKey::new(id);
    let capabilities =
        TypeCapabilities::try_new(vec![CapabilityDescriptor::with_adapter(key, 7_u32)]).expect("unique capability");

    ALLOCATIONS.with(|count| count.set(Some(0)));
    drop(std::hint::black_box(Box::new(7_u32)));
    let control = ALLOCATIONS.with(|count| count.replace(None).expect("control counting is active"));
    assert!(control > 0, "allocations on the measured thread must be counted");

    // Background work must not be charged to this thread's lookup contract.
    static START_BACKGROUND: AtomicBool = AtomicBool::new(false);
    static BACKGROUND_DONE: AtomicBool = AtomicBool::new(false);
    let background = std::thread::spawn(|| {
        while !START_BACKGROUND.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
        drop(std::hint::black_box(vec![0_u8; 4096]));
        BACKGROUND_DONE.store(true, Ordering::Release);
    });

    ALLOCATIONS.with(|count| count.set(Some(0)));
    START_BACKGROUND.store(true, Ordering::Release);
    for _ in 0..1_000 {
        let key = CapabilityKey::new(CapabilityId::new("example.allocation_free").expect("valid capability ID"));
        assert!(capabilities.contains(key));
        assert_eq!(capabilities.get(key), Some(&7_u32));
    }
    while !BACKGROUND_DONE.load(Ordering::Acquire) {
        std::thread::yield_now();
    }
    let allocations = ALLOCATIONS.with(|count| count.replace(None).expect("counting is active"));
    background.join().expect("background allocation must complete");

    assert_eq!(allocations, 0);
}
