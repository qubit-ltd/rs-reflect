use super::fixtures::{Worker, method};
use qubit_reflect::{Invocation, InvocationOutput};
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

#[test]
fn test_associated_and_instance_receivers_follow_method_metadata() {
    let worker = Worker { value: 10 };
    let instance = method("sum")
        .invoke(Invocation::on_ref(&worker).arg(2_i32).arg(3_i32))
        .expect("an instance receiver should match the method declaration");
    assert_eq!(instance.into_owned::<i32>().expect("sum output"), 15);

    let failure = method("sum")
        .invoke(Invocation::associated().arg(2_i32).arg(3_i32))
        .expect_err("associated input must not satisfy a shared receiver");
    assert!(matches!(
        failure.error().kind(),
        qubit_reflect::invoke::InvocationErrorKind::ReceiverMissing
    ));
}

#[test]
fn test_named_and_positional_bindings_can_be_mixed() {
    let worker = Worker { value: 10 };
    let output = method("sum")
        .invoke(
            Invocation::on_ref(&worker)
                .arg(2_i32)
                .named_arg("right", 3_i32),
        )
        .expect("mixed positional and named arguments should bind to declarations");

    assert_eq!(output.into_owned::<i32>().expect("sum output"), 15);
}

#[test]
fn test_async_method_returns_a_future_for_the_caller_to_poll() {
    let worker = Worker { value: 41 };
    let output = method("async_value")
        .invoke(Invocation::on_ref(&worker))
        .expect("async method dispatch should construct a reflected future");
    let InvocationOutput::Future(future) = output else {
        panic!("async method dispatch must return InvocationOutput::Future");
    };
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());

    let Poll::Ready(InvocationOutput::Owned(value)) = Future::poll(future.as_mut(), &mut context)
    else {
        panic!("the fixture future should complete on its first poll");
    };
    assert_eq!(
        InvocationOutput::Owned(value)
            .into_owned::<i32>()
            .expect("future output type"),
        41
    );
}
