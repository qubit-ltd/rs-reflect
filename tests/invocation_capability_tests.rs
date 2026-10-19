#![cfg(feature = "derive")]

use qubit_reflect::invoke::CaughtInvocationFailure;
use qubit_reflect::{Invocation, InvocationOutput, Reflect, ReflectImpl};
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

#[Reflect]
struct Counter {
    value: u32,
}

#[ReflectImpl]
impl Counter {
    pub fn increment(&mut self, by: u32) {
        self.value += by;
    }

    pub async fn async_value(&self) -> u32 {
        self.value
    }
}

fn method(name: &str) -> qubit_reflect::MethodDescriptor<'static> {
    qubit_reflect::ReflectionContext::global()
        .type_of::<Counter>()
        .method(name)
        .expect("the reflected method must be registered")
}

#[test]
fn test_catching_capability_runs_generated_method_adapter() {
    let mut counter = Counter { value: 5 };
    let output = method("increment")
        .invoke_catching(Invocation::on_mut(&mut counter).arg(3_u32))
        .expect("synchronous catching adapter should be present");

    assert!(matches!(output, InvocationOutput::Unit));
    assert_eq!(counter.value, 8);
}

#[test]
fn test_catching_capability_rejects_async_before_dispatch_and_keeps_inputs() {
    let counter = Counter { value: 13 };
    let failure = method("async_value")
        .invoke_catching(Invocation::on_ref(&counter))
        .expect_err("synchronous catching cannot dispatch async methods");
    let CaughtInvocationFailure::Rejected(rejected) = failure else {
        panic!("async rejection occurs before user method entry");
    };
    let retry = method("async_value")
        .invoke(rejected.into_invocation())
        .expect("normal invocation should reuse the recovered async receiver");
    let InvocationOutput::Future(future) = retry else {
        panic!("async method must return a future");
    };
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    let Poll::Ready(output) = Future::poll(future.as_mut(), &mut context) else {
        panic!("fixture future should complete on first poll");
    };
    assert_eq!(output.into_owned::<u32>().expect("u32 output"), 13);
}
