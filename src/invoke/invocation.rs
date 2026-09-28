// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Invocation collection and all-input validation.

mod binding_recovery;
mod binding_validation;
#[path = "invocation/invocation.rs"]
mod invocation_data;
mod validated_invocation;

pub use invocation_data::Invocation;
pub use validated_invocation::ValidatedInvocation;

/// Result of converting a validated dynamic receiver through a typed adapter.
///
/// # Type Parameters
///
/// - `'call`: Lifetime retained by borrowed arguments in a failure.
/// - `R`: Concrete receiver type produced by the adapter.
/// - `M`: Dynamic ownership mode of the invocation arguments.
///
/// # Examples
///
/// ```
/// use qubit_reflect::invoke::ReceiverAdaptationResult;
/// use qubit_reflect::value::Local;
///
/// let result: ReceiverAdaptationResult<'static, u8, Local> = Ok((7, Box::new([])));
/// assert!(matches!(result, Ok((7, arguments)) if arguments.is_empty()));
/// ```
pub type ReceiverAdaptationResult<'call, R, M> =
    Result<(R, Box<[crate::invoke::InvocationArg<'call, M>]>), crate::invoke::InvocationFailure<'call, M>>;
