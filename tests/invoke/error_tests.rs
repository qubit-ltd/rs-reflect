use super::fixtures::{Worker, method};
use qubit_reflect::{CaughtInvocationFailure, Invocation};

#[test]
fn test_catching_invocation_reports_user_panic_as_a_separate_failure() {
    let worker = Worker::default();
    let Err(CaughtInvocationFailure::Panicked(panic)) =
        method("panic").invoke_catching(Invocation::on_ref(&worker))
    else {
        panic!("a panic after method entry must be reported as Panicked");
    };

    assert!(panic.to_string().contains("worker panic payload"));
}

#[test]
fn test_async_invocation_is_not_run_by_synchronous_catching_entry() {
    let worker = Worker { value: 17 };
    let failure = method("async_value")
        .invoke_catching(Invocation::on_ref(&worker))
        .expect_err("async methods cannot use synchronous panic catching");

    assert!(matches!(failure, CaughtInvocationFailure::Rejected(_)));
}
