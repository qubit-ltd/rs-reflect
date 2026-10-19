use super::fixtures::{Worker, method};
use qubit_reflect::Invocation;
use qubit_reflect::invoke::CaughtInvocationFailure;

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
