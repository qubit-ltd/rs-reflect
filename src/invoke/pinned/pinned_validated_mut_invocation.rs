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

/// Validated input for a `Pin<&mut T>` receiver method.
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
/// use qubit_reflect::invoke::{PinnedValidatedMutInvocation, PinnedMutInvocation};
/// use qubit_reflect::value::Local;
///
/// let identity = MemberId::new(
///     "example::Worker",
///     "run",
///     0,
///     FragmentIdentity::new("example", "example::Worker", 1, 1, "method", 1),
/// );
/// let mut value = Box::pin(7_u8);
/// let validated: PinnedValidatedMutInvocation<'_, u8, Local> = PinnedMutInvocation::new(value.as_mut(), [])
///     .validate(&identity, &[])
///     .expect("an empty argument list matches");
/// assert!(validated.into_parts().1.is_empty());
/// ```
pub struct PinnedValidatedMutInvocation<'call, T: ?Sized, M: InvocationMode> {
    pub(in crate::invoke::pinned) receiver: Pin<&'call mut T>,
    pub(in crate::invoke::pinned) arguments: Box<[InvocationArg<'call, M>]>,
}

impl<'call, T: ?Sized, M: InvocationMode> PinnedValidatedMutInvocation<'call, T, M> {
    /// Consumes validation state and returns the pin proof with its arguments.
    ///
    /// # Returns
    ///
    /// Returns the pinned receiver and validated arguments.
    pub fn into_parts(self) -> (Pin<&'call mut T>, Box<[InvocationArg<'call, M>]>) {
        (self.receiver, self.arguments)
    }
}
