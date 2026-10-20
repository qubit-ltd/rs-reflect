// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::rc::Rc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use crate::runtime::descriptor::Reflect;
use crate::runtime::descriptor::TypeRegistration;
use crate::runtime::error::RecoverableFailure;
use crate::runtime::value::DynamicMut;
use crate::runtime::value::DynamicOwned;
use crate::runtime::value::DynamicRef;
use crate::runtime::value::DynamicValue;
use crate::runtime::value::DynamicValueKind;
use crate::runtime::value::Local;
use crate::runtime::value::ThreadSafe;

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

#[test]
fn downcast_failure_returns_the_same_owned_value() {
    let value = DynamicOwned::<Local>::new(String::from("retained"));
    let failure = value.downcast::<u64>().expect_err("wrong target type must fail");
    assert_eq!(failure.expected(), std::any::TypeId::of::<u64>());
    assert_eq!(failure.actual(), std::any::TypeId::of::<String>());
    assert_eq!(
        RecoverableFailure::error(&failure).expected(),
        std::any::TypeId::of::<u64>()
    );
    assert_eq!(
        RecoverableFailure::recovery(&failure)
            .downcast_ref::<String>()
            .map(String::as_str),
        Some("retained")
    );
    let (mismatch, restored) = RecoverableFailure::into_parts(failure);
    assert_eq!(mismatch.actual(), std::any::TypeId::of::<String>());
    assert_eq!(
        restored.downcast::<String>().expect("original type remains available"),
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
    assert_eq!(value.downcast_ref::<String>().map(String::as_str), Some("safe"));
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
        mutable.as_ref().downcast_ref::<String>().map(String::as_str),
        Some("mutable")
    );
}

#[test]
fn dynamic_shared_references_preserve_dst_metadata() {
    let text: &str = "hello";
    let dynamic = DynamicRef::<Local>::new(text);
    assert_eq!(
        dynamic.downcast::<str>().expect("str adapter preserves its length"),
        "hello"
    );

    let values: &[u64] = &[2, 3, 5];
    let dynamic = DynamicRef::<Local>::new(values);
    assert_eq!(
        dynamic.downcast::<[u64]>().expect("slice adapter preserves its length"),
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
        reborrowed.downcast_mut::<[u64]>().expect("slice type matches")[1] = 8;
    }
    assert_eq!(&*dynamic.downcast::<[u64]>().expect("slice type matches"), &[1, 8, 3]);
}

#[test]
fn local_mode_accepts_non_thread_safe_reflected_values() {
    let value = DynamicOwned::<Local>::new(Rc::new(String::from("local")));
    assert_eq!(
        value.downcast_ref::<Rc<String>>().map(|value| value.as_str()),
        Some("local")
    );

    let recovered = value
        .downcast::<Rc<String>>()
        .expect("Local mode accepts an Rc-backed value");
    assert_eq!(recovered.as_str(), "local");
}

#[test]
fn thread_safe_shared_and_mutable_views_preserve_type_and_mutation() {
    let text = String::from("shared");
    let shared = DynamicRef::<ThreadSafe>::new(&text);
    assert!(shared.clone().is::<String>());
    assert_eq!(shared.downcast_ref::<String>().map(String::as_str), Some("shared"));
    let cloned = shared.clone();
    assert_eq!(cloned.downcast::<String>().expect("matching shared type"), "shared");

    let mut text = String::from("before");
    let mut mutable = DynamicMut::<ThreadSafe>::new(&mut text);
    assert!(mutable.is::<String>());
    assert_eq!(mutable.downcast_ref::<String>().map(String::as_str), Some("before"));
    mutable
        .downcast_mut::<String>()
        .expect("matching mutable type")
        .push_str(" after");
    assert_eq!(
        mutable.as_ref().downcast_ref::<String>().map(String::as_str),
        Some("before after")
    );
    mutable.downcast::<String>().expect("matching mutable type").push('!');
    assert_eq!(text, "before after!");
}

#[test]
fn dynamic_value_converts_owned_and_borrowed_forms_without_losing_failures() {
    let owned = DynamicValue::Owned(DynamicOwned::<Local>::new(String::from("owned")));
    assert_eq!(
        owned
            .into_owned::<u64>()
            .expect_err("wrong owned type must return the tagged value")
            .kind(),
        DynamicValueKind::Owned
    );
    assert_eq!(
        DynamicValue::Owned(DynamicOwned::<Local>::new(String::from("owned")))
            .into_owned::<String>()
            .unwrap_or_else(|_| panic!("matching owned type")),
        "owned"
    );

    let text: Box<str> = String::from("borrowed").into_boxed_str();
    let shared = DynamicValue::Ref(DynamicRef::<Local>::new(&*text));
    assert_eq!(
        shared
            .into_ref::<str>()
            .unwrap_or_else(|_| panic!("matching shared DST type")),
        "borrowed"
    );
    let shared = DynamicValue::Ref(DynamicRef::<Local>::new(&*text));
    assert_eq!(
        shared
            .into_ref::<[u8]>()
            .expect_err("wrong shared type must return the tagged view")
            .kind(),
        DynamicValueKind::Ref
    );
    assert_eq!(
        DynamicValue::Owned(DynamicOwned::<Local>::new(String::from("owned")))
            .into_ref::<String>()
            .expect_err("an owner cannot be recovered as an original borrow")
            .kind(),
        DynamicValueKind::Owned
    );

    let mut text = String::from("mutable");
    let mutable = DynamicValue::Mut(DynamicMut::<Local>::new(&mut text));
    mutable
        .into_mut::<String>()
        .unwrap_or_else(|_| panic!("matching mutable type"))
        .push_str(" view");
    assert_eq!(text, "mutable view");
}

#[test]
fn downcast_failure_exposes_structured_mismatch_and_borrowed_recovery() {
    let failure = DynamicRef::<Local>::new("text")
        .downcast::<[u8]>()
        .expect_err("str is not a byte slice");
    let mismatch = RecoverableFailure::error(&failure);
    assert_eq!(mismatch.expected(), std::any::TypeId::of::<[u8]>());
    assert_eq!(mismatch.actual(), std::any::TypeId::of::<str>());
    assert_eq!(mismatch.expected_name(), std::any::type_name::<[u8]>());
    assert_eq!(mismatch.actual_name(), std::any::type_name::<str>());
    assert!(mismatch.to_string().contains("type mismatch"));
    assert!(std::error::Error::source(mismatch).is_none());
    assert!(format!("{failure:?}").contains("DowncastFailure"));

    let recovered = failure.into_value();
    assert_eq!(
        recovered
            .downcast::<str>()
            .expect("failure retains the original borrowed value"),
        "text"
    );
}
