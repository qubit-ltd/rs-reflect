// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Compile fixture for invocation behavior in `panic = "abort"` builds.

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

#[cfg(panic = "abort")]
use qubit_reflect::Invocation;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
#[cfg(panic = "abort")]
use qubit_reflect::invoke::CaughtInvocationFailure;
#[cfg(panic = "abort")]
use qubit_reflect::invoke::InvocationErrorKind;
#[cfg(panic = "abort")]
use qubit_reflect::value::Local;

#[Reflect]
struct AbortPayload {
    value: String,
    #[reflect(skip)]
    drops: Arc<AtomicUsize>,
}

impl Drop for AbortPayload {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

#[Reflect]
struct Worker {
    value: usize,
}

#[ReflectImpl]
impl Worker {
    pub fn set(&mut self, value: AbortPayload) {
        self.value = value.value.len();
    }
}

#[cfg(not(panic = "abort"))]
fn main() {}

#[cfg(panic = "abort")]
fn main() {
    let mut worker = Worker { value: 0 };
    let drops = Arc::new(AtomicUsize::new(0));
    let payload = AbortPayload {
        value: String::from("recovered value"),
        drops: Arc::clone(&drops),
    };
    let method = qubit_reflect::ReflectionContext::global()
        .type_of::<Worker>()
        .method("set")
        .expect("the panic method must be registered");
    let failure = match method.invoke_catching(Invocation::<Local>::on_mut(&mut worker).arg(payload)) {
        Err(failure) => failure,
        Ok(_) => panic!("panic=abort cannot support catching dispatch"),
    };
    let rejected = match failure {
        CaughtInvocationFailure::Rejected(rejected) => rejected,
        CaughtInvocationFailure::Panicked(_) => {
            panic!("abort mode must reject before entering the user method");
        }
    };
    assert!(matches!(
        rejected.error().kind(),
        InvocationErrorKind::ValueModeUnavailable
    ));
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    let recovered = rejected.into_invocation();
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    let output = match method.invoke(recovered) {
        Ok(output) => output,
        Err(_) => panic!("normal invocation should reuse the recovered input"),
    };
    assert!(matches!(output, qubit_reflect::InvocationOutput::Unit));
    assert_eq!(worker.value, "recovered value".len());
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}
