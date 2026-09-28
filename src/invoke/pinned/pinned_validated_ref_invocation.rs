// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed pinned invocation components.

use std::pin::Pin;

use crate::invoke::InvocationArg;
use crate::invoke::InvocationMode;

/// Validated input for a `Pin<&T>` receiver method.
///
/// # Type Parameters
///
/// - `'call`: Lifetime retained by borrowed arguments.
/// - `T`: Concrete receiver type kept behind the pin.
/// - `M`: Dynamic ownership mode of invocation arguments.
///
/// # Examples
///
/// ```
/// use qubit_reflect::identity::{FragmentIdentity, MemberId};
/// use qubit_reflect::invoke::{PinnedValidatedRefInvocation, PinnedRefInvocation};
/// use qubit_reflect::value::Local;
///
/// let identity = MemberId::new(
///     "example::Worker",
///     "run",
///     0,
///     FragmentIdentity::new("example", "example::Worker", 1, 1, "method", 1),
/// );
/// let value = Box::pin(7_u8);
/// let validated: PinnedValidatedRefInvocation<'_, u8, Local> = PinnedRefInvocation::new(value.as_ref(), [])
///     .validate(&identity, &[])
///     .expect("an empty argument list matches");
/// assert!(validated.into_parts().1.is_empty());
/// ```
pub struct PinnedValidatedRefInvocation<'call, T: ?Sized, M: InvocationMode> {
    pub(in crate::invoke::pinned) receiver: Pin<&'call T>,
    pub(in crate::invoke::pinned) arguments: Box<[InvocationArg<'call, M>]>,
}

impl<'call, T: ?Sized, M: InvocationMode> PinnedValidatedRefInvocation<'call, T, M> {
    /// Consumes validation state and returns the pin proof with its arguments.
    ///
    /// # Returns
    ///
    /// Returns the pinned receiver and validated arguments.
    pub fn into_parts(self) -> (Pin<&'call T>, Box<[InvocationArg<'call, M>]>) {
        (self.receiver, self.arguments)
    }
}
