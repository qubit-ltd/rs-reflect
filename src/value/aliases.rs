// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Ergonomic aliases for common dynamic reflection value modes.

use crate::value::DynamicMut;
use crate::value::DynamicOwned;
use crate::value::DynamicRef;
use crate::value::Local;
use crate::value::ThreadSafe;

/// A locally scoped shared dynamic value borrow.
///
/// # Examples
///
/// ```
/// use qubit_reflect::ReflectedRef;
/// let value = 7_u8;
/// let reflected = ReflectedRef::new(&value);
/// assert_eq!(reflected.downcast_ref::<u8>(), Some(&7));
/// ```
pub type ReflectedRef<'a> = DynamicRef<'a, Local>;
/// A locally scoped mutable dynamic value borrow.
///
/// # Examples
///
/// ```
/// use qubit_reflect::ReflectedMut;
/// let mut value = 7_u8;
/// let mut reflected = ReflectedMut::new(&mut value);
/// *reflected.downcast_mut::<u8>().expect("the reflected value is a u8") = 8;
/// assert_eq!(value, 8);
/// ```
pub type ReflectedMut<'a> = DynamicMut<'a, Local>;
/// A locally scoped owned dynamic value.
///
/// # Examples
///
/// ```
/// use qubit_reflect::ReflectedOwned;
/// let reflected = ReflectedOwned::new(7_u8);
/// assert_eq!(reflected.downcast_ref::<u8>(), Some(&7));
/// ```
pub type ReflectedOwned = DynamicOwned<Local>;
/// A thread-safe shared dynamic value borrow.
///
/// # Examples
///
/// ```
/// use qubit_reflect::SendReflectedRef;
/// let value = 7_u8;
/// let reflected = SendReflectedRef::new(&value);
/// assert_eq!(reflected.downcast_ref::<u8>(), Some(&7));
/// ```
pub type SendReflectedRef<'a> = DynamicRef<'a, ThreadSafe>;
/// A thread-safe mutable dynamic value borrow.
///
/// # Examples
///
/// ```
/// use qubit_reflect::SendReflectedMut;
/// let mut value = 7_u8;
/// let mut reflected = SendReflectedMut::new(&mut value);
/// *reflected.downcast_mut::<u8>().expect("the reflected value is a u8") = 8;
/// assert_eq!(value, 8);
/// ```
pub type SendReflectedMut<'a> = DynamicMut<'a, ThreadSafe>;
/// A thread-safe owned dynamic value.
///
/// # Examples
///
/// ```
/// use qubit_reflect::SendReflectedOwned;
/// let reflected = SendReflectedOwned::new(7_u8);
/// assert_eq!(reflected.downcast_ref::<u8>(), Some(&7));
/// ```
pub type SendReflectedOwned = DynamicOwned<ThreadSafe>;
