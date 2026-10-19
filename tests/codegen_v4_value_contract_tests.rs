use qubit_reflect::__private::codegen_v4::descriptor::{Reflect, TypeRegistration};
use qubit_reflect::__private::codegen_v4::value::{
    DynamicMut, DynamicOwned, DynamicRef, Local, ThreadSafe,
};

struct ContractValue(String);
#[derive(Debug)]
struct OtherValue;

impl Reflect for ContractValue {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        static REGISTRATION: TypeRegistration<ContractValue> = TypeRegistration::new();
        &REGISTRATION
    }
}

impl Reflect for OtherValue {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        static REGISTRATION: TypeRegistration<OtherValue> = TypeRegistration::new();
        &REGISTRATION
    }
}

#[test]
fn external_codegen_value_facade_preserves_owned_and_borrowed_values() {
    let owned = DynamicOwned::<Local>::new(ContractValue(String::from("value")));
    assert!(owned.is::<ContractValue>());
    assert_eq!(owned.downcast_ref::<ContractValue>().unwrap().0, "value");

    let failure = owned
        .downcast::<OtherValue>()
        .expect_err("an unrelated type must not consume the dynamic value");
    let mut owned = failure.into_value();
    owned
        .downcast_mut::<ContractValue>()
        .unwrap()
        .0
        .push_str(" retained");
    assert_eq!(
        owned.downcast::<ContractValue>().unwrap().0,
        "value retained"
    );

    let value = ContractValue(String::from("borrowed"));
    let shared = DynamicRef::<Local>::new(&value);
    assert_eq!(shared.downcast::<ContractValue>().unwrap().0, "borrowed");

    let mut value = ContractValue(String::from("mutable"));
    let mut dynamic = DynamicMut::<Local>::new(&mut value);
    dynamic
        .downcast_mut::<ContractValue>()
        .unwrap()
        .0
        .push_str(" borrow");
    let view = dynamic.as_ref();
    assert_eq!(
        view.downcast_ref::<ContractValue>().unwrap().0,
        "mutable borrow"
    );
    drop(view);
    dynamic
        .reborrow()
        .downcast_mut::<ContractValue>()
        .unwrap()
        .0
        .push_str("ed");
    drop(dynamic);
    assert_eq!(value.0, "mutable borrowed");
}

#[test]
fn thread_safe_codegen_values_prove_auto_traits() {
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<DynamicOwned<ThreadSafe>>();
    assert_send_sync::<DynamicRef<'static, ThreadSafe>>();
    assert_send_sync::<DynamicMut<'static, ThreadSafe>>();

    let owned = DynamicOwned::<ThreadSafe>::new(ContractValue(String::from("safe")));
    assert_eq!(owned.downcast_ref::<ContractValue>().unwrap().0, "safe");
}

#[test]
fn external_codegen_value_facade_supports_string_slices() {
    let text = String::from("shared text");
    let dynamic = DynamicRef::<Local>::new_str(&text);
    assert_eq!(dynamic.into_str().unwrap(), "shared text");

    let mut text = String::from("mutable text");
    let dynamic = DynamicMut::<Local>::new_str_mut(text.as_mut_str());
    dynamic.into_str_mut().unwrap().make_ascii_uppercase();
    assert_eq!(text, "MUTABLE TEXT");
}
