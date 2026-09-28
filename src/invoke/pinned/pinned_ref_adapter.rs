// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed pinned invocation components.

use super::pinned_ref_invocation::PinnedRefInvocation;
use super::pinned_ref_invocation_failure::PinnedRefInvocationFailure;
use crate::invoke::InvocationOutput;
use crate::registry::ReflectRegistry;

/// A generated entry point for a method whose receiver is `Pin<&T>`.
///
/// # Type Parameters
///
/// - `T`: Concrete receiver type accepted by the generated method.
/// - `M`: Dynamic ownership mode used by invocation arguments.
///
/// # Examples
///
/// ```
/// use qubit_reflect::invoke::{InvocationOutput, PinnedRefAdapter, PinnedRefInvocation, PinnedRefInvocationFailure};
/// use qubit_reflect::registry::ReflectRegistry;
/// use qubit_reflect::value::Local;
///
/// fn adapter<'registry, 'call>(
///     _: &'registry ReflectRegistry,
///     _invocation: PinnedRefInvocation<'call, u8, Local>,
/// ) -> Result<InvocationOutput<'call, Local>, PinnedRefInvocationFailure<'call, u8, Local>> {
///     unreachable!("the example only checks the adapter signature")
/// }
/// let _: PinnedRefAdapter<u8, Local> = adapter;
/// ```
pub type PinnedRefAdapter<T, M> =
    for<'registry, 'call> fn(
        &'registry ReflectRegistry,
        PinnedRefInvocation<'call, T, M>,
    ) -> Result<InvocationOutput<'call, M>, PinnedRefInvocationFailure<'call, T, M>>;
