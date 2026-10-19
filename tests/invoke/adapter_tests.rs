use super::fixtures::{Worker, method};
use qubit_reflect::{Invocation, InvocationOutput};

#[test]
fn test_method_descriptor_invokes_generated_local_adapter() {
    let mut worker = Worker::default();
    let output = method("add")
        .invoke(Invocation::on_mut(&mut worker).arg(7_i32))
        .expect("the generated method adapter should run");

    assert!(matches!(output, InvocationOutput::Unit));
    assert_eq!(worker.value, 7);
}

#[test]
fn test_method_descriptor_returns_owned_output_from_generated_adapter() {
    let worker = Worker { value: 5 };
    let output = method("sum")
        .invoke(Invocation::on_ref(&worker).arg(3_i32).arg(4_i32))
        .expect("the generated method adapter should run");

    assert_eq!(output.into_owned::<i32>().expect("sum returns i32"), 12);
}
