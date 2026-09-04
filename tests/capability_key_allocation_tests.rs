// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

//! Allocation regression coverage for typed capability lookups.

use std::alloc::GlobalAlloc;
use std::alloc::Layout;
use std::alloc::System;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use qubit_reflect::capability::CapabilityDescriptor;
use qubit_reflect::capability::CapabilityKey;
use qubit_reflect::capability::TypeCapabilities;
use qubit_reflect::identity::CapabilityId;

struct CountingAllocator;

static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
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

    ALLOCATIONS.store(0, Ordering::Relaxed);
    COUNTING.store(true, Ordering::SeqCst);
    for _ in 0..1_000 {
        let key = CapabilityKey::new(CapabilityId::new("example.allocation_free").expect("valid capability ID"));
        assert!(capabilities.contains(key));
        assert_eq!(capabilities.get(key), Some(&7_u32));
    }
    COUNTING.store(false, Ordering::SeqCst);

    assert_eq!(ALLOCATIONS.load(Ordering::Relaxed), 0);
}
