// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Safe function-pointer boundaries used by generated field adapters.

use std::any::TypeId;

use crate::access::FieldAccessError;
use crate::value::DynamicMut;
use crate::value::DynamicOwned;
use crate::value::DynamicRef;
use crate::value::Local;
use crate::value::ThreadSafe;

/// A shared field adapter preserving the input target's borrow lifetime.
pub type FieldGetAdapter = for<'a> fn(DynamicRef<'a, Local>) -> Result<DynamicRef<'a, Local>, FieldAccessError>;

/// A mutable field adapter preserving the input target's exclusive borrow
/// lifetime.
pub type FieldGetMutAdapter = for<'a> fn(DynamicMut<'a, Local>) -> Result<DynamicMut<'a, Local>, FieldAccessError>;

/// A whole-value replacement adapter for a field.
pub type FieldSetAdapter = for<'a> fn(DynamicMut<'a, Local>, DynamicOwned<Local>) -> Result<(), FieldAccessError>;

/// A non-consuming validation hook run immediately before a field set adapter.
///
/// Generated enum fields use this hook to reject an inactive variant while
/// the descriptor still owns and can recover the replacement value.
#[doc(hidden)]
pub type FieldSetPreflightAdapter = for<'a> fn(&DynamicMut<'a, Local>) -> Result<(), FieldAccessError>;

/// A thread-safe shared field adapter preserving the input borrow lifetime.
pub type ThreadSafeFieldGetAdapter =
    for<'a> fn(DynamicRef<'a, ThreadSafe>) -> Result<DynamicRef<'a, ThreadSafe>, FieldAccessError>;

/// A thread-safe mutable field adapter preserving the exclusive borrow.
pub type ThreadSafeFieldGetMutAdapter =
    for<'a> fn(DynamicMut<'a, ThreadSafe>) -> Result<DynamicMut<'a, ThreadSafe>, FieldAccessError>;

/// A thread-safe whole-value field replacement adapter.
pub type ThreadSafeFieldSetAdapter =
    for<'a> fn(DynamicMut<'a, ThreadSafe>, DynamicOwned<ThreadSafe>) -> Result<(), FieldAccessError>;

/// A thread-safe non-consuming validation hook for field replacement.
#[doc(hidden)]
pub type ThreadSafeFieldSetPreflightAdapter = for<'a> fn(&DynamicMut<'a, ThreadSafe>) -> Result<(), FieldAccessError>;

/// Returns the exact type identity carried by a local shared dynamic value.
pub(crate) fn dynamic_ref_type_id(value: &DynamicRef<'_, Local>) -> TypeId {
    match value.as_any() {
        Some(value) => value.type_id(),
        None => {
            debug_assert!(value.as_str().is_some());
            TypeId::of::<str>()
        }
    }
}

/// Returns the exact type identity carried by a local mutable dynamic value.
pub(crate) fn dynamic_mut_type_id(value: &DynamicMut<'_, Local>) -> TypeId {
    match value.as_any() {
        Some(value) => value.type_id(),
        None => {
            debug_assert!(value.as_str().is_some());
            TypeId::of::<str>()
        }
    }
}

/// Returns the exact type identity carried by a local owned dynamic value.
pub(crate) fn dynamic_owned_type_id(value: &DynamicOwned<Local>) -> TypeId {
    value
        .as_any()
        .expect("local owned dynamic values are always Any-compatible")
        .type_id()
}

/// Returns the exact identity carried by a thread-safe shared value.
pub(crate) fn thread_safe_ref_type_id(value: &DynamicRef<'_, ThreadSafe>) -> TypeId {
    value
        .as_any()
        .map_or_else(TypeId::of::<str>, |value| (value as &dyn std::any::Any).type_id())
}

/// Returns the exact identity carried by a thread-safe mutable value.
pub(crate) fn thread_safe_mut_type_id(value: &DynamicMut<'_, ThreadSafe>) -> TypeId {
    value
        .as_any()
        .map_or_else(TypeId::of::<str>, |value| (value as &dyn std::any::Any).type_id())
}

/// Returns the exact identity carried by a thread-safe owned value.
pub(crate) fn thread_safe_owned_type_id(value: &DynamicOwned<ThreadSafe>) -> TypeId {
    (value
        .as_any()
        .expect("thread-safe owned dynamic values are always Any-compatible") as &dyn std::any::Any)
        .type_id()
}
