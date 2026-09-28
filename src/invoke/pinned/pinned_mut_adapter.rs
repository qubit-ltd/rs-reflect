// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed pinned invocation components.

use super::pinned_mut_invocation::PinnedMutInvocation;
use super::pinned_mut_invocation_failure::PinnedMutInvocationFailure;
use crate::invoke::InvocationOutput;
use crate::registry::ReflectRegistry;

/// A generated entry point for a method whose receiver is `Pin<&mut T>`.
///
/// # Type Parameters
///
/// - `T`: Concrete receiver type accepted by the generated method.
/// - `M`: Dynamic ownership mode used by invocation arguments.
///
/// # Examples
///
/// ```
/// use qubit_reflect::invoke::{InvocationOutput, PinnedMutAdapter, PinnedMutInvocation, PinnedMutInvocationFailure};
/// use qubit_reflect::registry::ReflectRegistry;
/// use qubit_reflect::value::Local;
///
/// fn adapter<'registry, 'call>(
///     _: &'registry ReflectRegistry,
///     _invocation: PinnedMutInvocation<'call, u8, Local>,
/// ) -> Result<InvocationOutput<'call, Local>, PinnedMutInvocationFailure<'call, u8, Local>> {
///     unreachable!("the example only checks the adapter signature")
/// }
/// let _: PinnedMutAdapter<u8, Local> = adapter;
/// ```
pub type PinnedMutAdapter<T, M> =
    for<'registry, 'call> fn(
        &'registry ReflectRegistry,
        PinnedMutInvocation<'call, T, M>,
    ) -> Result<InvocationOutput<'call, M>, PinnedMutInvocationFailure<'call, T, M>>;
