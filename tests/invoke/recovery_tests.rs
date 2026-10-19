use super::fixtures::{Worker, method};
use qubit_reflect::{Invocation, InvocationErrorKind, Reflect};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[Reflect]
struct RecoveryPayload {
    drops: Arc<AtomicUsize>,
}

impl Drop for RecoveryPayload {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn test_preflight_failure_keeps_original_receiver_and_owned_argument() {
    let mut worker = Worker { value: 12 };
    let failure = method("add")
        .invoke(Invocation::on_mut(&mut worker).arg(String::from("keep me")))
        .expect_err("the owned String cannot satisfy the i32 parameter");

    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ArgumentTypeMismatch { index: 0, .. }
    ));
    assert_eq!(worker.value, 12, "preflight must not enter the method body");

    let retry = method("add")
        .invoke(failure.into_invocation())
        .expect_err("recovery must retain the caller's original typed input");
    assert!(matches!(
        retry.error().kind(),
        InvocationErrorKind::ArgumentTypeMismatch { index: 0, .. }
    ));
    assert_eq!(worker.value, 12);
}

#[test]
fn test_named_preflight_failure_retains_binding_for_retry() {
    let worker = Worker::default();
    let failure = method("sum")
        .invoke(
            Invocation::on_ref(&worker)
                .named_arg("left", 2_i32)
                .named_arg("right", String::from("original")),
        )
        .expect_err("a named argument with the wrong type must be rejected");

    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ArgumentTypeMismatch { index: 1, .. }
    ));
    let retry = method("sum")
        .invoke(failure.into_invocation())
        .expect_err("recovery must preserve the named binding and value type");
    assert!(matches!(
        retry.error().kind(),
        InvocationErrorKind::ArgumentTypeMismatch { index: 1, .. }
    ));
}

#[test]
fn test_recovery_owns_the_original_value_until_recovery_is_dropped() {
    let drops = Arc::new(AtomicUsize::new(0));
    let payload = RecoveryPayload {
        drops: Arc::clone(&drops),
    };
    let mut worker = Worker::default();
    let failure = method("add")
        .invoke(Invocation::on_mut(&mut worker).arg(payload))
        .expect_err("payload type must fail the i32 parameter preflight");

    assert_eq!(drops.load(Ordering::SeqCst), 0);
    let recovered = failure.into_invocation();
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(recovered);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}
