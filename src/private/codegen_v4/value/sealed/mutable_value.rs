// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::Mode;
use crate::runtime::descriptor::Reflect;
use crate::runtime::value as runtime_value;

/// Marks a type that can be erased into a mutable dynamic borrow.
///
/// # Type Parameters
///
/// * `M` — value mode that selects the runtime borrow representation.
pub trait MutableValue<M: Mode>: Reflect + runtime_value::RuntimeValueForMut<M::Runtime> {
    // empty
}

impl<T: Reflect + ?Sized, M: Mode> MutableValue<M> for T where T: runtime_value::RuntimeValueForMut<M::Runtime> {}
