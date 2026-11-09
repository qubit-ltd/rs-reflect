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
use super::sealed::OwnedValue;
use crate::runtime::descriptor::Reflect;
use crate::runtime::value as runtime_value;

/// A sealed marker for sized values that can be stored in a dynamic owner.
///
/// # Type Parameters
///
/// * `M` — value mode that selects the owner representation.
pub trait ValueFor<M: ValueMode>: Reflect + Sized + 'static + OwnedValue<M> {
    // empty
}

impl<T: Reflect + Sized + 'static> ValueFor<Local> for T where T: runtime_value::RuntimeValueFor<runtime_value::Local> {}

impl<T: Reflect + Send + Sync + Sized + 'static> ValueFor<ThreadSafe> for T where
    T: runtime_value::RuntimeValueFor<runtime_value::ThreadSafe>
{
}
