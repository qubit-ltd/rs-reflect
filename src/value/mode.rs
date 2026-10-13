// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

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
///
/// # Examples
///
/// ```
/// use qubit_reflect::value::{Local, Mode, ThreadSafe};
///
/// fn accepts_dynamic_mode<M: Mode>() {}
/// accepts_dynamic_mode::<Local>();
/// accepts_dynamic_mode::<ThreadSafe>();
/// ```
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
/// # Examples
///
/// ```
/// use qubit_reflect::value::{Local, Mode};
///
/// fn accepts_local<M: Mode>() {}
/// accepts_local::<Local>();
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::{DynamicRef, Local};
///
/// fn assert_send<T: Send>(_: T) {}
/// let value = 1_u32;
/// assert_send(DynamicRef::<Local>::new(&value));
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::{DynamicRef, Local};
///
/// fn assert_sync<T: Sync>(_: T) {}
/// let value = 1_u32;
/// assert_sync(DynamicRef::<Local>::new(&value));
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::{DynamicMut, Local};
///
/// fn assert_send<T: Send>(_: T) {}
/// let mut value = 1_u32;
/// assert_send(DynamicMut::<Local>::new(&mut value));
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::{DynamicMut, Local};
///
/// fn assert_sync<T: Sync>(_: T) {}
/// let mut value = 1_u32;
/// assert_sync(DynamicMut::<Local>::new(&mut value));
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::{DynamicOwned, Local};
///
/// fn assert_send<T: Send>(_: T) {}
/// assert_send(DynamicOwned::<Local>::new(1_u32));
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::{DynamicOwned, Local};
///
/// fn assert_sync<T: Sync>(_: T) {}
/// assert_sync(DynamicOwned::<Local>::new(1_u32));
/// ```
pub enum Local {}

/// The dynamic value mode with compile-time `Send` and `Sync` constraints.
///
/// Thread-safe constructors reject values that do not meet their documented
/// bounds at compile time.
///
/// # Examples
///
/// ```
/// use qubit_reflect::value::{Mode, ThreadSafe};
///
/// fn accepts_thread_safe<M: Mode>() {}
/// accepts_thread_safe::<ThreadSafe>();
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::{DynamicOwned, ThreadSafe};
/// use std::rc::Rc;
///
/// let _ = SendDynamicOwned::<Local>::new(Rc::new(1_u32));
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::{DynamicOwned, ThreadSafe};
/// use std::cell::Cell;
///
/// let _ = SendDynamicOwned::<Local>::new(Cell::new(1_u32));
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::{DynamicRef, ThreadSafe};
/// use std::cell::Cell;
///
/// let value = Cell::new(1_u32);
/// let _ = SendDynamicRef::<Local>::new(&value);
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::{DynamicMut, ThreadSafe};
/// use std::rc::Rc;
///
/// let mut value = Rc::new(1_u32);
/// let _ = SendDynamicMut::<Local>::new(&mut value);
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::{DynamicMut, ThreadSafe};
/// use std::cell::Cell;
///
/// let mut value = Cell::new(1_u32);
/// let _ = SendDynamicMut::<Local>::new(&mut value);
/// ```
///
/// Dynamic borrows cannot be promoted to `'static`.
///
/// ```compile_fail
/// use qubit_reflect::value::{DynamicRef, Local};
///
/// fn promote<'a>(value: DynamicRef<'a, Local>) -> DynamicRef<'static, Local> {
///     value
/// }
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::{DynamicMut, Local};
///
/// fn promote<'a>(value: DynamicMut<'a, Local>) -> DynamicMut<'static, Local> {
///     value
/// }
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::{DynamicRef, ThreadSafe};
///
/// fn promote<'a>(value: DynamicRef<'a, ThreadSafe>) -> DynamicRef<'static, ThreadSafe> {
///     value
/// }
/// ```
///
/// ```compile_fail
/// use qubit_reflect::value::{DynamicMut, ThreadSafe};
///
/// fn promote<'a>(value: DynamicMut<'a, ThreadSafe>) -> DynamicMut<'static, ThreadSafe> {
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
