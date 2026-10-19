use super::fixtures::{Worker, method};
use qubit_reflect::invoke::{CaughtInvocationFailure, InvocationErrorKind};
use qubit_reflect::{Invocation, InvocationOutput, Reflect};
use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

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
    let retry = method("add")
        .invoke(failure.into_invocation())
        .expect_err("recovery must retain the caller's original typed input");
    assert!(matches!(
        retry.error().kind(),
        InvocationErrorKind::ArgumentTypeMismatch { index: 0, .. }
    ));
    drop(retry);
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

#[test]
fn test_rejected_async_invocation_retries_with_recovered_receiver_and_named_arguments() {
    let worker = Worker { value: 17 };
    let failure = method("async_sum")
        .invoke_catching(
            Invocation::on_ref(&worker)
                .named_arg("right", 5_i32)
                .named_arg("left", 3_i32),
        )
        .expect_err("synchronous catching must reject async methods before execution");
    let CaughtInvocationFailure::Rejected(rejected) = failure else {
        panic!("preflight rejection must remain distinct from a user panic");
    };

    let output = method("async_sum")
        .invoke(rejected.into_invocation())
        .expect("ordinary dispatch should accept the same recovered call");
    let InvocationOutput::Future(future) = output else {
        panic!("async methods should return a future");
    };
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    let Poll::Ready(output) = Future::poll(future.as_mut(), &mut context) else {
        panic!("fixture future should complete on the first poll");
    };

    assert_eq!(output.into_owned::<i32>().expect("i32 output"), 25);
}
