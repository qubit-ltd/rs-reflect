// qubit-style: allow explicit-imports
//! Standalone coverage for invocation capability descriptor state.

use qubit_reflect::descriptor::CatchingAvailability;
use qubit_reflect::descriptor::InvocationAdapter;
use qubit_reflect::invoke::CatchingInvocationResult;
use qubit_reflect::invoke::Invocation;
use qubit_reflect::invoke::InvocationFailure;
use qubit_reflect::invoke::InvocationOutput;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

fn return_seven<'call>(
    _invocation: Invocation<'call, Local>,
) -> Result<InvocationOutput<'call, Local>, InvocationFailure<'call, Local>> {
    Ok(InvocationOutput::Owned(DynamicOwned::<Local>::new(7_u8)))
}

fn return_eight<'call>(
    _invocation: Invocation<'call, ThreadSafe>,
) -> Result<InvocationOutput<'call, ThreadSafe>, InvocationFailure<'call, ThreadSafe>> {
    Ok(InvocationOutput::Owned(DynamicOwned::<ThreadSafe>::new(8_u8)))
}

fn catch_seven<'call>(invocation: Invocation<'call, Local>) -> CatchingInvocationResult<'call, Local> {
    return_seven(invocation).map(Ok)
}

fn catch_eight<'call>(invocation: Invocation<'call, ThreadSafe>) -> CatchingInvocationResult<'call, ThreadSafe> {
    return_eight(invocation).map(Ok)
}

#[test]
fn test_invocation_adapter_reports_explicit_catching_availability_by_mode() {
    let local = InvocationAdapter::local_with_catching(return_seven, catch_seven);
    assert_eq!(local.catching_availability(), CatchingAvailability::Available);
    assert!(
        local
            .invoke_catching_local(Invocation::associated([]))
            .expect("the local catching entry point must be present")
            .expect("validation must succeed")
            .is_ok()
    );

    let thread_safe = InvocationAdapter::thread_safe_with_catching(return_eight, catch_eight);
    assert_eq!(thread_safe.catching_availability(), CatchingAvailability::Available);
    assert!(
        thread_safe
            .invoke_catching_thread_safe(Invocation::associated([]))
            .expect("the thread-safe catching entry point must be present")
            .expect("validation must succeed")
            .is_ok()
    );
}

#[test]
fn test_invocation_adapter_distinguishes_unrequested_and_abort_unavailable_catching() {
    let unrequested = InvocationAdapter::local(return_seven);
    assert_eq!(unrequested.catching_availability(), CatchingAvailability::NotRequested);

    let unavailable = InvocationAdapter::local_with_unavailable_catching(return_seven);
    assert_eq!(
        unavailable.catching_availability(),
        CatchingAvailability::UnavailablePanicAbort
    );
    assert!(unavailable.invoke_catching_local(Invocation::associated([])).is_none());
}
