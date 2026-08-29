//! Integration tests for borrowed dynamic values.

use qubit_reflect::value::{ReflectedMut, ReflectedRef};

#[test]
/// Confirms a failed shared downcast leaves the original borrow usable.
fn test_ref_downcast_failure_keeps_the_borrow_usable() {
    let number = 42_u32;
    let value = ReflectedRef::new(&number);

    assert_eq!(value.downcast_ref::<String>(), None);
    assert_eq!(value.downcast_ref::<u32>(), Some(&42));
    assert!(value.is::<u32>());
}

#[test]
/// Confirms a failed mutable downcast leaves the original borrow usable.
fn test_mut_downcast_failure_keeps_the_borrow_usable() {
    let mut number = 42_u32;
    let mut value = ReflectedMut::new(&mut number);

    assert_eq!(value.downcast_mut::<String>(), None);
    *value
        .downcast_mut::<u32>()
        .expect("the mutable borrow remains usable after a failed downcast") = 43;
    drop(value);

    assert_eq!(number, 43);
}

#[test]
/// Confirms shared and mutable borrows expose safe `Any` interoperability.
fn test_borrowed_any_interoperation_preserves_type_identity() {
    let mut number = 42_u32;
    let value = ReflectedRef::new(&number);
    assert_eq!(
        value.as_any().and_then(|value| value.downcast_ref::<u32>()),
        Some(&42)
    );

    let mut value = ReflectedMut::new(&mut number);
    *value
        .as_any_mut()
        .and_then(|value| value.downcast_mut::<u32>())
        .expect("the stored borrow is a u32") = 43;
    drop(value);

    assert_eq!(number, 43);
}
