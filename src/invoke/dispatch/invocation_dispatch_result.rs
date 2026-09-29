// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Result of selecting an invocation entry without losing unavailable input.

use crate::invoke::InvocationUnavailable;

/// The result of dispatching input `I` to an invocation entry returning `R`.
///
/// `Err` preserves the original input and an availability reason before any
/// validation or execution. `Ok` contains the entry's complete result; it does
/// not imply that validation or method execution succeeded. No additional
/// trait bounds are imposed on either type parameter.
///
/// # Examples
///
/// ```
/// use qubit_reflect::invoke::InvocationDispatchResult;
///
/// let result: InvocationDispatchResult<(), Result<u8, &str>> = Ok(Err("invalid input"));
/// assert_eq!(result.expect("dispatch succeeded"), Err("invalid input"));
/// ```
pub type InvocationDispatchResult<I, R> = Result<R, InvocationUnavailable<I>>;
