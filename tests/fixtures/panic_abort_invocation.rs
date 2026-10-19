use qubit_reflect::{
    CaughtInvocationFailure, Invocation, InvocationErrorKind, Reflect, ReflectImpl,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[Reflect]
struct AbortPayload {
    drops: Arc<AtomicUsize>,
}

impl Drop for AbortPayload {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

#[Reflect]
struct Worker;

#[ReflectImpl]
impl Worker {
    pub fn panic(&self, value: String) {
        panic!("must not run: {value}");
    }
}

#[cfg(not(panic = "abort"))]
fn main() {}

#[cfg(panic = "abort")]
fn main() {
    let worker = Worker;
    let drops = Arc::new(AtomicUsize::new(0));
    let payload = AbortPayload {
        drops: Arc::clone(&drops),
    };
    let method = qubit_reflect::ReflectionContext::global()
        .type_of::<Worker>()
        .method("panic")
        .expect("the panic method must be registered");
    let failure = method
        .invoke_catching(Invocation::on_ref(&worker).arg(payload))
        .expect_err("panic=abort cannot support catching dispatch");
    let CaughtInvocationFailure::Rejected(rejected) = failure else {
        panic!("abort mode must reject before entering the user method");
    };
    assert!(matches!(
        rejected.error().kind(),
        InvocationErrorKind::ValueModeUnavailable
    ));
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    let recovered = rejected.into_invocation();
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(recovered);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}
