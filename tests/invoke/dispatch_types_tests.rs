use super::fixtures::{Worker, method};
use qubit_reflect::Invocation;
use qubit_reflect::invoke::ThreadSafe;

#[test]
fn test_invocation_mode_selects_local_or_thread_safe_adapter() {
    let worker = Worker { value: 8 };

    let local = method("sum")
        .invoke(Invocation::on_ref(&worker).arg(1_i32).arg(2_i32))
        .expect("the local adapter should be available");
    let shared = method("sum")
        .invoke(
            Invocation::<ThreadSafe>::on_ref(&worker)
                .arg(1_i32)
                .arg(2_i32),
        )
        .expect("the thread-safe adapter should be available for a Sync target");

    assert_eq!(local.into_owned::<i32>().expect("i32 output"), 11);
    assert_eq!(shared.into_owned::<i32>().expect("i32 output"), 11);
}
