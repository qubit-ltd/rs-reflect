use default_trait_thread_safe_consumer::LocalTarget;
use default_trait_thread_safe_consumer::SharedTarget;
use default_trait_thread_safe_consumer::local_marker_count;
use default_trait_thread_safe_consumer::new_local_target;
use qubit_reflect::Invocation;
use qubit_reflect::ReflectionContext;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

#[test]
fn external_default_method_has_thread_safe_adapter_for_send_target() {
    let method = ReflectionContext::global()
        .type_of::<SharedTarget>()
        .method("render")
        .expect("the external default method is registered");
    let target = SharedTarget;
    let output = method
        .invoke(Invocation::<ThreadSafe>::on_ref(&target).arg(String::from("shared")))
        .unwrap_or_else(|_| panic!("the Send + Sync target has a ThreadSafe adapter"));
    let output = output
        .into_owned::<String>()
        .unwrap_or_else(|_| panic!("the default method returns String"));
    assert_eq!(output, "rendered: shared");
}

#[test]
fn external_default_method_keeps_local_adapter_for_rc_target() {
    let method = ReflectionContext::global()
        .type_of::<LocalTarget>()
        .method("render")
        .expect("the external default method is registered");
    let target = new_local_target();
    assert_eq!(local_marker_count(&target), 1);
    let output = method
        .invoke(Invocation::<Local>::on_ref(&target).arg(String::from("local")))
        .unwrap_or_else(|_| panic!("the Rc target retains a Local adapter"));
    let output = output
        .into_owned::<String>()
        .unwrap_or_else(|_| panic!("the default method returns String"));
    assert_eq!(output, "rendered: local");
}
