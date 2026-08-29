//! Integration tests for borrowed dynamic values.

use qubit_reflect::value::{ReflectedMut, ReflectedRef};

/// Confirms a failed shared downcast leaves the original borrow usable.
#[test]
fn test_ref_downcast_failure_keeps_the_borrow_usable() {
    let number = 42_u32;
    let value = ReflectedRef::new(&number);

    assert_eq!(value.downcast_ref::<String>(), None);
    assert_eq!(value.downcast_ref::<u32>(), Some(&42));
    assert!(value.is::<u32>());
}

/// Confirms a failed mutable downcast leaves the original borrow usable.
#[test]
fn test_mut_downcast_failure_keeps_the_borrow_usable() {
    let mut number = 42_u32;
    {
        let mut value = ReflectedMut::new(&mut number);

        assert_eq!(value.downcast_mut::<String>(), None);
        *value
            .downcast_mut::<u32>()
            .expect("the mutable borrow remains usable after a failed downcast") = 43;
    }

    assert_eq!(number, 43);
}

/// Confirms shared and mutable borrows expose safe `Any` interoperability.
#[test]
fn test_borrowed_any_interoperation_preserves_type_identity() {
    let mut number = 42_u32;
    let value = ReflectedRef::new(&number);
    assert_eq!(
        value.as_any().and_then(|value| value.downcast_ref::<u32>()),
        Some(&42)
    );

    {
        let mut value = ReflectedMut::new(&mut number);
        *value
            .as_any_mut()
            .and_then(|value| value.downcast_mut::<u32>())
            .expect("the stored borrow is a u32") = 43;
    }

    assert_eq!(number, 43);
}

/// Confirms consuming a shared wrapper exposes the original borrow lifetime
/// and returns the wrapper unchanged after an exact-type mismatch.
#[test]
fn test_ref_consuming_downcast_preserves_borrow_and_recovers_mismatch() {
    let number = 42_u32;
    let value = ReflectedRef::new(&number);
    let value = match value.downcast::<String>() {
        Ok(_) => panic!("a mismatched consuming downcast must fail"),
        Err(value) => value,
    };

    let number_ref = value.downcast::<u32>().unwrap_or_else(|_| {
        panic!("an exact consuming downcast should return the original borrow")
    });
    assert_eq!(number_ref, &42);
}

/// Confirms consuming a mutable wrapper returns an exclusive borrow with the
/// original lifetime and preserves the wrapper after a mismatch.
#[test]
fn test_mut_consuming_downcast_preserves_borrow_and_recovers_mismatch() {
    let mut number = 42_u32;
    let value = ReflectedMut::new(&mut number);
    let value = match value.downcast::<String>() {
        Ok(_) => panic!("a mismatched consuming downcast must fail"),
        Err(value) => value,
    };
    *value.downcast::<u32>().unwrap_or_else(|_| {
        panic!("an exact consuming downcast should return the original mutable borrow")
    }) = 43;

    assert_eq!(number, 43);
}
