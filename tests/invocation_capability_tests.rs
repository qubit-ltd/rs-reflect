#![cfg(feature = "derive")]

use qubit_reflect::{CaughtInvocationFailure, Invocation, InvocationOutput, Reflect, ReflectImpl};

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
        .invoke_catching(rejected.into_invocation())
        .expect_err("recovered receiver must remain available");
    assert!(matches!(retry, CaughtInvocationFailure::Rejected(_)));
}
