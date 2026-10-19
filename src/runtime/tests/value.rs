use std::sync::atomic::{AtomicUsize, Ordering};

use crate::runtime::descriptor::{Reflect, TypeRegistration};
use crate::runtime::value::{
    DynamicMut, DynamicOwned, DynamicRef, DynamicValue, DynamicValueKind, Local, ThreadSafe,
};

static DROPS: AtomicUsize = AtomicUsize::new(0);

struct Dropped;
impl Drop for Dropped {
    fn drop(&mut self) {
        DROPS.fetch_add(1, Ordering::SeqCst);
    }
}
impl Reflect for Dropped {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        static REGISTRATION: TypeRegistration<Dropped> = TypeRegistration::new();
        &REGISTRATION
    }
}

impl Reflect for u64 {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        static REGISTRATION: TypeRegistration<u64> = TypeRegistration::new();
        &REGISTRATION
    }
}

impl Reflect for [u64] {}

#[test]
fn downcast_failure_returns_the_same_owned_value() {
    let value = DynamicOwned::<Local>::new(String::from("retained"));
    let failure = value
        .downcast::<u64>()
        .expect_err("wrong target type must fail");
    assert_eq!(failure.expected(), std::any::TypeId::of::<u64>());
    assert_eq!(failure.actual(), std::any::TypeId::of::<String>());
    let restored = failure.into_value();
    assert_eq!(
        restored
            .downcast::<String>()
            .expect("original type remains available"),
        "retained"
    );
}

#[test]
fn owned_value_is_dropped_exactly_once_after_failed_downcast_recovery() {
    DROPS.store(0, Ordering::SeqCst);
    let value = DynamicOwned::<Local>::new(Dropped);
    let value = value
        .downcast::<u64>()
        .expect_err("wrong target type must fail")
        .into_value();
    assert_eq!(DROPS.load(Ordering::SeqCst), 0);
    drop(value);
    assert_eq!(DROPS.load(Ordering::SeqCst), 1);
}

#[test]
fn thread_safe_owned_values_can_be_downgraded_to_local() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<DynamicOwned<ThreadSafe>>();
    assert_send_sync::<DynamicRef<'static, ThreadSafe>>();
    assert_send_sync::<DynamicMut<'static, ThreadSafe>>();
    assert_send_sync::<DynamicValue<'static, ThreadSafe>>();

    let value = DynamicOwned::<ThreadSafe>::new(String::from("safe")).into_local();
    assert_eq!(
        value.downcast_ref::<String>().map(String::as_str),
        Some("safe")
    );
}

#[test]
fn dynamic_value_reports_kind_and_exposes_a_shared_view() {
    let owned = DynamicValue::Owned(DynamicOwned::<Local>::new(String::from("owned")));
    assert_eq!(owned.kind(), DynamicValueKind::Owned);
    assert_eq!(owned.type_id(), std::any::TypeId::of::<String>());
    assert_eq!(
        owned.as_ref().downcast_ref::<String>().map(String::as_str),
        Some("owned")
    );

    let text = String::from("borrowed");
    let shared = DynamicValue::Ref(DynamicRef::<Local>::new(&text));
    assert_eq!(shared.kind(), DynamicValueKind::Ref);
    assert_eq!(
        shared.as_ref().downcast_ref::<String>().map(String::as_str),
        Some("borrowed")
    );

    let mut text = String::from("mutable");
    let mutable = DynamicValue::Mut(DynamicMut::<Local>::new(&mut text));
    assert_eq!(mutable.kind(), DynamicValueKind::Mut);
    assert_eq!(mutable.ty().type_id(), std::any::TypeId::of::<String>());
    assert_eq!(
        mutable
            .as_ref()
            .downcast_ref::<String>()
            .map(String::as_str),
        Some("mutable")
    );
}

#[test]
fn dynamic_shared_references_preserve_dst_metadata() {
    let text: &str = "hello";
    let dynamic = DynamicRef::<Local>::new(text);
    assert_eq!(
        dynamic
            .downcast::<str>()
            .expect("str adapter preserves its length"),
        "hello"
    );

    let values: &[u64] = &[2, 3, 5];
    let dynamic = DynamicRef::<Local>::new(values);
    assert_eq!(
        dynamic
            .downcast::<[u64]>()
            .expect("slice adapter preserves its length"),
        &[2, 3, 5]
    );
}

#[test]
fn borrowed_downcast_failures_return_the_original_borrow_handles() {
    let text: &str = "kept";
    let failure = DynamicRef::<Local>::new(text)
        .downcast::<[u64]>()
        .expect_err("wrong target type must fail");
    assert_eq!(failure.actual(), std::any::TypeId::of::<str>());
    assert_eq!(
        failure
            .into_value()
            .downcast::<str>()
            .expect("original shared borrow remains available"),
        "kept"
    );

    let mut values = [4_u64, 5];
    let failure = DynamicMut::<Local>::new(&mut values[..])
        .downcast::<str>()
        .expect_err("wrong target type must fail");
    assert_eq!(failure.actual(), std::any::TypeId::of::<[u64]>());
    assert_eq!(
        &*failure
            .into_value()
            .downcast::<[u64]>()
            .expect("original mutable borrow remains available"),
        &[4, 5]
    );
}

#[test]
fn dynamic_mut_reborrow_and_downcast_preserve_the_original_borrow() {
    let mut values = [1_u64, 2, 3];
    let mut dynamic = DynamicMut::<Local>::new(&mut values[..]);
    {
        let mut reborrowed = dynamic.reborrow();
        reborrowed
            .downcast_mut::<[u64]>()
            .expect("slice type matches")[1] = 8;
    }
    assert_eq!(
        &*dynamic.downcast::<[u64]>().expect("slice type matches"),
        &[1, 8, 3]
    );
}
