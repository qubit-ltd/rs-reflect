//! Integration coverage for callable invocation adapter descriptors.

use qubit_reflect::descriptor::InvocationAdapter;
use qubit_reflect::invoke::{Invocation, InvocationFailure, InvocationOutput};
use qubit_reflect::value::{DynamicOwned, Local, ThreadSafe};

/// Returns one owned output through the local invocation boundary.
fn return_seven<'call>(
    _invocation: Invocation<'call, Local>,
) -> Result<InvocationOutput<'call, Local>, InvocationFailure<'call, Local>> {
    Ok(InvocationOutput::Owned(DynamicOwned::<Local>::new(7_u8)))
}

#[test]
fn test_invocation_adapter_local_descriptor_invokes_its_typed_entry_point() {
    let adapter = InvocationAdapter::local(return_seven);
    let output = adapter
        .invoke_local(Invocation::associated([]))
        .expect("the local entry point must be present")
        .expect("the local adapter must be callable");
    let InvocationOutput::Owned(value) = output else {
        panic!("adapter must retain its owned output");
    };
    let Ok(value) = DynamicOwned::<Local>::downcast::<u8>(value) else {
        panic!("output type must be u8");
    };
    assert_eq!(value, 7);
}

/// Returns one owned thread-safe output through the generated boundary.
fn return_eight<'call>(
    _invocation: Invocation<'call, ThreadSafe>,
) -> Result<InvocationOutput<'call, ThreadSafe>, InvocationFailure<'call, ThreadSafe>> {
    Ok(InvocationOutput::Owned(DynamicOwned::<ThreadSafe>::new(8_u8)))
}

#[test]
fn test_invocation_adapter_thread_safe_descriptor_invokes_its_typed_entry_point() {
    let adapter = InvocationAdapter::thread_safe(return_eight);
    let output = adapter
        .invoke_thread_safe(Invocation::associated([]))
        .expect("the thread-safe entry point must be present")
        .expect("the thread-safe adapter must be callable");
    let InvocationOutput::Owned(value) = output else {
        panic!("adapter must retain its owned output");
    };
    let Ok(value) = DynamicOwned::<ThreadSafe>::downcast::<u8>(value) else {
        panic!("output type must be u8");
    };
    assert_eq!(value, 8);
}
