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
use super::sealed::SharedValue;
use crate::runtime::descriptor::Reflect;
use crate::runtime::value as runtime_value;

/// A sealed marker for values that can be represented by a shared dynamic
/// borrow.
///
/// # Type Parameters
///
/// * `M` — value mode that selects the shared-borrow representation.
pub trait ValueForRef<M: ValueMode>: Reflect + SharedValue<M> {
    // empty
}

impl<T: Reflect + ?Sized> ValueForRef<Local> for T where T: runtime_value::RuntimeValueForRef<runtime_value::Local> {}

impl<T: Reflect + Sync + ?Sized> ValueForRef<ThreadSafe> for T where
    T: runtime_value::RuntimeValueForRef<runtime_value::ThreadSafe>
{
}
