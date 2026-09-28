// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Positional invocation argument values.

use crate::invoke::InvocationMode;
use crate::value::DynamicMut;
use crate::value::DynamicOwned;
use crate::value::DynamicRef;

/// One non-receiver positional invocation argument.
///
/// # Examples
///
/// ```
/// use qubit_reflect::invoke::{InvocationArg, InvocationInputMode};
/// use qubit_reflect::value::{DynamicOwned, Local};
///
/// let argument = InvocationArg::<Local>::Owned(DynamicOwned::<Local>::new(7_u8));
/// assert_eq!(argument.mode(), InvocationInputMode::Owned);
/// ```
pub enum InvocationArg<'call, M: InvocationMode> {
    /// An owned argument that may be consumed only after validation succeeds.
    Owned(DynamicOwned<M>),
    /// A shared borrowed argument.
    Ref(DynamicRef<'call, M>),
    /// A mutable borrowed argument.
    Mut(DynamicMut<'call, M>),
}
