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
pub type ReflectedRef<'a> = DynamicRef<'a, Local>;
/// A locally scoped mutable dynamic value borrow.
pub type ReflectedMut<'a> = DynamicMut<'a, Local>;
/// A locally scoped owned dynamic value.
pub type ReflectedOwned = DynamicOwned<Local>;
/// A thread-safe shared dynamic value borrow.
pub type SendReflectedRef<'a> = DynamicRef<'a, ThreadSafe>;
/// A thread-safe mutable dynamic value borrow.
pub type SendReflectedMut<'a> = DynamicMut<'a, ThreadSafe>;
/// A thread-safe owned dynamic value.
pub type SendReflectedOwned = DynamicOwned<ThreadSafe>;
