use super::fixtures::{Worker, method};
use qubit_reflect::Invocation;
use qubit_reflect::invoke::InvocationErrorKind;

#[test]
fn test_invocation_rejects_missing_receiver() {
    let failure = method("add")
        .invoke(Invocation::associated().arg(9_i32))
        .expect_err("an instance method requires a receiver");

    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ReceiverMissing
    ));
}

#[test]
fn test_invocation_rejects_wrong_argument_type_before_method_body() {
    let mut worker = Worker::default();
    let failure = method("add")
        .invoke(Invocation::on_mut(&mut worker).arg(String::from("preserved")))
        .expect_err("argument TypeId must be checked before dispatch");

    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ArgumentTypeMismatch { index: 0, .. }
    ));
    let retry = method("add")
        .invoke(failure.into_invocation())
        .expect_err("the rejected call should retain its original argument");
    assert!(matches!(
        retry.error().kind(),
        InvocationErrorKind::ArgumentTypeMismatch { index: 0, .. }
    ));
    drop(retry);
    assert_eq!(worker.value, 0);
}

#[test]
fn test_unsafe_retry_uses_same_receiver_and_argument_after_visibility_rejection() {
    let mut worker = Worker::default();
    let private = method("private_method")
        .invoke(Invocation::on_mut(&mut worker).arg(5_i32))
        .expect_err("private methods require the explicit unsafe API");
    assert!(matches!(
        private.error().kind(),
        InvocationErrorKind::VisibilityDenied
    ));
    let private_output =
        unsafe { method("private_method").invoke_unchecked(private.into_invocation()) }
            .expect("unchecked retry should use the recovered private call inputs");
    assert!(matches!(
        private_output,
        qubit_reflect::InvocationOutput::Unit
    ));
    assert_eq!(worker.value, 5);

    let unsafe_method = method("unsafe_method")
        .invoke(Invocation::on_mut(&mut worker).arg(7_i32))
        .expect_err("unsafe methods require the explicit unsafe API");
    assert!(matches!(
        unsafe_method.error().kind(),
        InvocationErrorKind::UnsafeMethod
    ));
    let unsafe_output =
        unsafe { method("unsafe_method").invoke_unchecked(unsafe_method.into_invocation()) }
            .expect("unchecked retry should use the recovered unsafe call inputs");
    assert!(matches!(
        unsafe_output,
        qubit_reflect::InvocationOutput::Unit
    ));
    assert_eq!(worker.value, 12);
}

#[test]
fn test_named_bindings_dispatch_in_declaration_order() {
    let worker = Worker { value: 1 };
    let output = method("sum")
        .invoke(
            Invocation::on_ref(&worker)
                .named_arg("right", 4_i32)
                .named_arg("left", 2_i32),
        )
        .expect("known named bindings should be reordered by parameter metadata");

    assert_eq!(output.into_owned::<i32>().expect("sum returns i32"), 7);
}
