// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Compile-time mode markers for dynamic value wrappers.

use std::marker::PhantomData;
use std::rc::Rc;

use crate::value::storage::LocalMutStorage;
use crate::value::storage::LocalOwnedStorage;
use crate::value::storage::LocalRefStorage;
use crate::value::storage::ThreadSafeMutStorage;
use crate::value::storage::ThreadSafeOwnedStorage;
use crate::value::storage::ThreadSafeRefStorage;

mod sealed {
    /// Restricts dynamic value modes to the crate-defined markers.
    pub trait Sealed {}
}

/// A sealed compile-time mode for dynamic value wrappers.
///
/// This trait is only implemented by [`Local`] and [`ThreadSafe`]. Its hidden
/// associated types preserve each mode's erased type and auto-trait boundary.
pub trait Mode: sealed::Sealed {
    /// Internal marker that determines the wrapper's auto traits.
    #[doc(hidden)]
    type Marker;
    /// Internal owned storage selected by this mode.
    #[doc(hidden)]
    type OwnedStorage;
    /// Internal shared-borrow storage selected by this mode.
    #[doc(hidden)]
    type RefStorage<'a>;
    /// Internal mutable-borrow storage selected by this mode.
    #[doc(hidden)]
    type MutStorage<'a>;
}

/// The local dynamic value mode.
///
/// Wrappers in this mode intentionally cannot implement `Send` or `Sync`.
///
/// ```compile_fail
/// use qubit_reflect::value::ReflectedRef;
///
/// fn assert_send<T: Send>(_: T) {}
/// let value = 1_u32;
/// assert_send(ReflectedRef::new(&value));
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::ReflectedRef;
///
/// fn assert_sync<T: Sync>(_: T) {}
/// let value = 1_u32;
/// assert_sync(ReflectedRef::new(&value));
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::ReflectedMut;
///
/// fn assert_send<T: Send>(_: T) {}
/// let mut value = 1_u32;
/// assert_send(ReflectedMut::new(&mut value));
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::ReflectedMut;
///
/// fn assert_sync<T: Sync>(_: T) {}
/// let mut value = 1_u32;
/// assert_sync(ReflectedMut::new(&mut value));
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::ReflectedOwned;
///
/// fn assert_send<T: Send>(_: T) {}
/// assert_send(ReflectedOwned::new(1_u32));
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::ReflectedOwned;
///
/// fn assert_sync<T: Sync>(_: T) {}
/// assert_sync(ReflectedOwned::new(1_u32));
/// ```
pub enum Local {}

/// The dynamic value mode with compile-time `Send` and `Sync` constraints.
///
/// Thread-safe constructors reject values that do not meet their documented
/// bounds at compile time.
///
/// ```compile_fail
/// use qubit_reflect::value::SendReflectedOwned;
/// use std::rc::Rc;
///
/// let _ = SendReflectedOwned::new(Rc::new(1_u32));
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::SendReflectedOwned;
/// use std::cell::Cell;
///
/// let _ = SendReflectedOwned::new(Cell::new(1_u32));
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::SendReflectedRef;
/// use std::cell::Cell;
///
/// let value = Cell::new(1_u32);
/// let _ = SendReflectedRef::new(&value);
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::SendReflectedMut;
/// use std::rc::Rc;
///
/// let mut value = Rc::new(1_u32);
/// let _ = SendReflectedMut::new(&mut value);
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::SendReflectedMut;
/// use std::cell::Cell;
///
/// let mut value = Cell::new(1_u32);
/// let _ = SendReflectedMut::new(&mut value);
/// ```
///
/// Dynamic borrows cannot be promoted to `'static`.
///
/// ```compile_fail
/// use qubit_reflect::value::ReflectedRef;
///
/// fn promote<'a>(value: ReflectedRef<'a>) -> ReflectedRef<'static> {
///     value
/// }
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::ReflectedMut;
///
/// fn promote<'a>(value: ReflectedMut<'a>) -> ReflectedMut<'static> {
///     value
/// }
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::SendReflectedRef;
///
/// fn promote<'a>(value: SendReflectedRef<'a>) -> SendReflectedRef<'static> {
///     value
/// }
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::SendReflectedMut;
///
/// fn promote<'a>(value: SendReflectedMut<'a>) -> SendReflectedMut<'static> {
///     value
/// }
/// ```
pub enum ThreadSafe {}

impl sealed::Sealed for Local {}

impl Mode for Local {
    type Marker = PhantomData<Rc<()>>;
    type OwnedStorage = LocalOwnedStorage;
    type RefStorage<'a> = LocalRefStorage<'a>;
    type MutStorage<'a> = LocalMutStorage<'a>;
}

impl sealed::Sealed for ThreadSafe {}

impl Mode for ThreadSafe {
    type Marker = ();
    type OwnedStorage = ThreadSafeOwnedStorage;
    type RefStorage<'a> = ThreadSafeRefStorage<'a>;
    type MutStorage<'a> = ThreadSafeMutStorage<'a>;
}
