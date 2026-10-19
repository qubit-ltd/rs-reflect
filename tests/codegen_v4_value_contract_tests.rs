use qubit_reflect::__private::codegen_v4::descriptor::{Reflect, TypeRegistration};
use qubit_reflect::__private::codegen_v4::value::{
    DynamicMut, DynamicOwned, DynamicRef, Local, ThreadSafe, ValueFor,
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

#[test]
fn external_codegen_value_facade_accepts_builtin_scalars_and_unit() {
    fn assert_builtin<T: ValueFor<Local>>(value: T) {
        drop(DynamicOwned::<Local>::new(value));
    }

    assert_builtin(false);
    assert_builtin('x');
    assert_builtin(1_i8);
    assert_builtin(2_i16);
    assert_builtin(3_i32);
    assert_builtin(4_i64);
    assert_builtin(5_i128);
    assert_builtin(6_isize);
    assert_builtin(7_u8);
    assert_builtin(8_u16);
    assert_builtin(9_u32);
    assert_builtin(10_u64);
    assert_builtin(11_u128);
    assert_builtin(12_usize);
    assert_builtin(1.0_f32);
    assert_builtin(2.0_f64);
    assert_builtin(String::from("builtin string"));
    assert_builtin(());
}
