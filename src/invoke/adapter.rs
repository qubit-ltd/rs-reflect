// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Function-pointer contracts used by generated invocation adapters.

use crate::invoke::Invocation;
use crate::invoke::InvocationFailure;
use crate::invoke::InvocationOutput;
use crate::registry::ReflectRegistry;

/// A mode-specific adapter that validates and invokes one concrete method.
///
/// The higher-ranked call lifetime prevents generated adapters from extending
/// receiver, argument, output, or future borrows beyond a single invocation.
/// The independent registry lifetime is used only during synchronous receiver
/// resolution. Neither outputs, futures, nor recovery retain that borrow.
pub type InvocationAdapter<M> = for<'registry, 'call> fn(
    &'registry ReflectRegistry,
    Invocation<'call, M>,
) -> Result<
    InvocationOutput<'call, M>,
    InvocationFailure<'call, M>,
>;

/// A mode-specific adapter that captures a user panic after validation.
///
/// The outer result is reserved for pre-execution validation failure; the
/// inner result distinguishes a normal invocation output from a caught panic.
/// Receiver capabilities resolve only in the supplied registry. Its borrow
/// ends before the returned output or recovery is used.
pub type CatchingInvocationAdapter<M> =
    for<'registry, 'call> fn(
        &'registry ReflectRegistry,
        Invocation<'call, M>,
    ) -> CatchingInvocationResult<'call, M>;

/// Result of an explicit catching invocation.
pub type CatchingInvocationResult<'call, M> = Result<
    Result<InvocationOutput<'call, M>, crate::invoke::InvocationPanic>,
    InvocationFailure<'call, M>,
>;
