//! Integration tests for thread-safe dynamic value mode.

use qubit_reflect::value::SendReflectedMut;
use qubit_reflect::value::SendReflectedOwned;
use qubit_reflect::value::SendReflectedRef;

/// Verifies that a wrapper retains both thread-safety auto traits.
fn assert_send_and_sync<T: Send + Sync>(_: T) {}

/// Confirms a thread-safe shared borrow crosses the mode boundary safely.
#[test]
fn test_thread_safe_ref_can_be_downgraded_without_losing_the_borrow() {
    let number = 42_u32;
    assert_send_and_sync(SendReflectedRef::new(&number));

    let value = SendReflectedRef::new(&number).into_local();
    assert_eq!(value.downcast_ref::<u32>(), Some(&42));
}

/// Confirms the thread-safe shared `Any` boundary supports direct type
/// inspection before its independent downgrade path.
#[test]
fn test_thread_safe_ref_any_and_downcast_are_available_before_downgrade() {
    let number = 42_u32;
    let value = SendReflectedRef::new(&number);

    assert!(value.as_any().is_some());
    assert_eq!(value.downcast_ref::<u32>(), Some(&42));
    assert_eq!(value.downcast_ref::<String>(), None);

    let value = value.into_local();
    assert_eq!(value.downcast_ref::<u32>(), Some(&42));
}

/// Confirms a thread-safe mutable borrow crosses the mode boundary safely.
#[test]
fn test_thread_safe_mut_can_be_downgraded_without_losing_the_borrow() {
    let mut number = 42_u32;
    assert_send_and_sync(SendReflectedMut::new(&mut number));

    {
        let mut value = SendReflectedMut::new(&mut number).into_local();
        *value
            .downcast_mut::<u32>()
            .expect("the mutable borrow survives the mode downgrade") = 43;
    }

    assert_eq!(number, 43);
}

/// Confirms the thread-safe mutable `Any` boundary supports direct mutation
/// before its independent downgrade path.
#[test]
fn test_thread_safe_mut_any_and_downcast_are_available_before_downgrade() {
    let mut number = 42_u32;
    let mut value = SendReflectedMut::new(&mut number);

    assert!(value.as_any().is_some());
    *value
        .as_any_mut()
        .and_then(|value| value.downcast_mut::<u32>())
        .expect("the thread-safe Any boundary retains the stored u32") = 43;
    assert_eq!(value.downcast_ref::<u32>(), Some(&43));

    let value = value.into_local();
    assert_eq!(value.downcast_ref::<u32>(), Some(&43));
}

/// Confirms the thread-safe owned `Any` boundary supports type recovery
/// without first downgrading to local mode.
#[test]
fn test_thread_safe_owned_any_and_downcast_preserve_the_original_wrapper() {
    let mut value = SendReflectedOwned::new(41_u32);

    assert_eq!(value.as_any().and_then(|value| value.downcast_ref::<u32>()), Some(&41));
    *value
        .as_any_mut()
        .and_then(|value| value.downcast_mut::<u32>())
        .expect("the thread-safe Any boundary retains the stored u32") = 42;

    let value = match value.downcast::<String>() {
        Ok(_) => panic!("a mismatched thread-safe owned downcast must fail"),
        Err(value) => value,
    };
    let number = match value.downcast::<u32>() {
        Ok(number) => number,
        Err(_) => panic!("a matching thread-safe owned downcast must recover the value"),
    };
    assert_eq!(number, 42);
}

/// Confirms thread-safe `str` variants retain their dedicated accessors after
/// downgrade.
#[test]
fn test_thread_safe_str_variants_can_be_downgraded() {
    let value = SendReflectedRef::new_str("hello");
    assert_send_and_sync(value);
    let value = SendReflectedRef::new_str("hello");
    assert!(value.as_any().is_none());
    assert_eq!(value.into_local().as_str(), Some("hello"));

    let mut text = String::from("hello");
    let value = SendReflectedMut::new_str_mut(text.as_mut_str());
    assert_send_and_sync(value);
    {
        let value = SendReflectedMut::new_str_mut(text.as_mut_str());
        assert!(value.as_any().is_none());
        let mut value = value.into_local();
        value
            .as_str_mut()
            .expect("the dedicated mutable str borrow survives the downgrade")
            .make_ascii_uppercase();
    }

    assert_eq!(text, "HELLO");
}

/// Confirms consuming thread-safe shared downcasts preserve both the original
/// borrow lifetime and wrapper recovery after an exact-type mismatch.
#[test]
fn test_thread_safe_ref_consuming_downcast_recovers_mismatch() {
    let number = 42_u32;
    let value = SendReflectedRef::new(&number);
    let value = match value.downcast::<String>() {
        Ok(_) => panic!("a mismatched consuming downcast must fail"),
        Err(value) => value,
    };

    let number_ref = value
        .downcast::<u32>()
        .unwrap_or_else(|_| panic!("an exact consuming downcast should return the thread-safe borrow"));
    assert_eq!(number_ref, &42);
}

/// Confirms consuming thread-safe mutable downcasts preserve the exclusive
/// borrow and recover the wrapper after a mismatch.
#[test]
fn test_thread_safe_mut_consuming_downcast_recovers_mismatch() {
    let mut number = 42_u32;
    let value = SendReflectedMut::new(&mut number);
    let value = match value.downcast::<String>() {
        Ok(_) => panic!("a mismatched consuming downcast must fail"),
        Err(value) => value,
    };
    *value
        .downcast::<u32>()
        .unwrap_or_else(|_| panic!("an exact consuming downcast should return the thread-safe mutable borrow")) = 43;

    assert_eq!(number, 43);
}

/// Confirms consuming typed extraction keeps dedicated thread-safe `str`
/// variants intact when they cannot participate in `Any` downcasts.
#[test]
fn test_thread_safe_str_consuming_downcast_returns_original_wrapper() {
    let value = SendReflectedRef::new_str("hello");
    let value = match value.downcast::<String>() {
        Ok(_) => panic!("a dedicated str variant must not downcast through Any"),
        Err(value) => value,
    };
    assert_eq!(value.as_str(), Some("hello"));

    let mut text = String::from("hello");
    let value = SendReflectedMut::new_str_mut(text.as_mut_str());
    let mut value = match value.downcast::<String>() {
        Ok(_) => panic!("a dedicated mutable str variant must not downcast through Any"),
        Err(value) => value,
    };
    value
        .as_str_mut()
        .expect("the original mutable str wrapper should be returned")
        .make_ascii_uppercase();
    assert_eq!(text, "HELLO");
}

/// Confirms consuming thread-safe `str` extraction returns the original shared
/// borrow and preserves a non-`str` wrapper on failure.
#[test]
fn test_thread_safe_shared_str_consuming_extraction_preserves_borrow_and_wrapper() {
    let text = String::from("hello");
    let extracted = match SendReflectedRef::new_str(text.as_str()).into_str() {
        Ok(extracted) => extracted,
        Err(_) => panic!("a dedicated thread-safe str wrapper must extract successfully"),
    };
    assert_eq!(extracted, "hello");

    let number = 42_u32;
    let value = match SendReflectedRef::new(&number).into_str() {
        Ok(_) => panic!("a thread-safe Any wrapper must not extract as str"),
        Err(value) => value,
    };
    assert_eq!(value.downcast_ref::<u32>(), Some(&42));
}

/// Confirms consuming thread-safe mutable `str` extraction returns the
/// original exclusive borrow and preserves a non-`str` wrapper on failure.
#[test]
fn test_thread_safe_mutable_str_consuming_extraction_preserves_borrow_and_wrapper() {
    let mut text = String::from("hello");
    match SendReflectedMut::new_str_mut(text.as_mut_str()).into_str_mut() {
        Ok(extracted) => extracted.make_ascii_uppercase(),
        Err(_) => panic!("a dedicated thread-safe mutable str wrapper must extract successfully"),
    }
    assert_eq!(text, "HELLO");

    let mut number = 42_u32;
    let mut value = match SendReflectedMut::new(&mut number).into_str_mut() {
        Ok(_) => panic!("a thread-safe Any wrapper must not extract as mutable str"),
        Err(value) => value,
    };
    *value
        .downcast_mut::<u32>()
        .expect("failed str extraction must return the original wrapper") = 43;
    assert_eq!(number, 43);
}
