// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::Local;
use super::ThreadSafe;
use super::ValueMode;
use super::sealed::MutableValue;
use crate::runtime::descriptor::Reflect;
use crate::runtime::value as runtime_value;

/// A sealed marker for values that can be represented by a mutable dynamic
/// borrow.
///
/// # Type Parameters
///
/// * `M` — value mode that selects the mutable-borrow representation.
pub trait ValueForMut<M: ValueMode>: Reflect + MutableValue<M> {
    // empty
}

impl<T: Reflect + ?Sized> ValueForMut<Local> for T where T: runtime_value::RuntimeValueForMut<runtime_value::Local> {}

impl<T: Reflect + Send + Sync + ?Sized> ValueForMut<ThreadSafe> for T where
    T: runtime_value::RuntimeValueForMut<runtime_value::ThreadSafe>
{
}
