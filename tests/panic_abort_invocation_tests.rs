#![cfg(feature = "derive")]
#![cfg(panic = "abort")]

use qubit_reflect::invoke::{CaughtInvocationFailure, InvocationErrorKind};
use qubit_reflect::{Invocation, Reflect, ReflectImpl};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[Reflect]
struct AbortPayload {
    value: String,
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

#[test]
fn test_abort_configuration_rejects_catching_before_method_entry_and_recovers_input() {
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
    let failure = method
        .invoke_catching(Invocation::on_mut(&mut worker).arg(payload))
        .expect_err("panic=abort cannot support catching dispatch");
    let CaughtInvocationFailure::Rejected(rejected) = failure else {
        panic!("abort mode must reject before entering the user method");
    };
    assert!(matches!(
        rejected.error().kind(),
        InvocationErrorKind::ValueModeUnavailable
    ));

    assert_eq!(drops.load(Ordering::SeqCst), 0);
    let output = method
        .invoke(rejected.into_invocation())
        .expect("normal invocation should reuse the rejected receiver and payload");
    assert!(matches!(output, qubit_reflect::InvocationOutput::Unit));
    assert_eq!(worker.value, "recovered value".len());
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}
