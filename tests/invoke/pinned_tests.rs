use super::fixtures::{Worker, method};
use qubit_reflect::invoke::PinnedInvocation;
use std::pin::Pin;

#[test]
fn test_pinned_mutable_receiver_invokes_pinned_method() {
    let mut worker = Box::pin(Worker { value: 23 });
    let invocation = PinnedInvocation::Mutable {
        receiver: Pin::as_mut(&mut worker),
        arguments: Box::new([]),
    };
    let output = method("pinned_value")
        .invoke_pinned(invocation)
        .expect("the generated pinned receiver adapter should run");

    assert_eq!(output.into_owned::<i32>().expect("pinned value is i32"), 23);
}

#[test]
fn test_pinned_receiver_failure_preserves_pin_and_argument_count() {
    let mut worker = Box::pin(Worker { value: 29 });
    let before = std::ptr::from_ref(Pin::as_ref(&worker).get_ref());
    let invocation = PinnedInvocation::Mutable {
        receiver: Pin::as_mut(&mut worker),
        arguments: Box::new([]),
    };
    let failure = method("sum")
        .invoke_pinned(invocation)
        .expect_err("a non-pinned method must reject a pinned receiver");
    let recovered = failure.into_invocation();

    let PinnedInvocation::Mutable {
        receiver,
        arguments,
    } = recovered
    else {
        panic!("the mutable pinned receiver must remain recoverable");
    };
    assert_eq!(std::ptr::from_ref(receiver.get_ref()), before);
    assert!(arguments.is_empty());
}
