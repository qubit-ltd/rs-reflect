// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::Reflect;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::descriptor::TypeRegistration;

static REGISTRATION_CALLS: AtomicUsize = AtomicUsize::new(0);

struct Fixture;

impl Reflect for Fixture {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        REGISTRATION_CALLS.fetch_add(1, Ordering::SeqCst);
        static REGISTRATION: TypeRegistration<Fixture> = TypeRegistration::new();
        &REGISTRATION
    }
}

#[test]
fn descriptor_is_bound_to_the_requested_context() {
    let context = ReflectionContext::empty();
    let descriptor = TypeDescriptor::new(Fixture::__reflect_registration().metadata(), &context);

    assert!(std::ptr::eq(descriptor.context(), &context));
    assert_eq!(descriptor.type_id(), std::any::TypeId::of::<Fixture>());
}

#[test]
fn of_uses_the_global_context_without_building_aggregate_state() {
    REGISTRATION_CALLS.store(0, Ordering::SeqCst);
    let before = ReflectionContext::global() as *const ReflectionContext;
    let descriptor = TypeDescriptor::of::<Fixture>();

    assert_eq!(descriptor.type_id(), std::any::TypeId::of::<Fixture>());
    assert!(std::ptr::eq(descriptor.context(), ReflectionContext::global()));
    assert_eq!(before, ReflectionContext::global() as *const ReflectionContext);
    assert_eq!(REGISTRATION_CALLS.load(Ordering::SeqCst), 1);
}
