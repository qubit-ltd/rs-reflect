// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed pinned invocation components.

use std::pin::Pin;

use super::pinned_ref_invocation::PinnedRefInvocation;
use crate::invoke::InvocationArg;
use crate::invoke::InvocationMode;

/// Complete input retained after a pinned shared invocation fails validation.
///
/// # Type Parameters
///
/// - `'call`: Lifetime retained by the pinned receiver and arguments.
/// - `T`: Concrete receiver type kept behind the pin.
/// - `M`: Dynamic ownership mode of invocation arguments.
///
/// # Examples
///
/// ```
/// use qubit_reflect::identity::{FragmentIdentity, MemberId};
/// use qubit_reflect::invoke::{ArgumentExpectation, PinnedRefInvocation, PinnedRefInvocationRecovery};
/// use qubit_reflect::value::Local;
///
/// let identity = MemberId::new(
///     "example::Worker",
///     "run",
///     0,
///     FragmentIdentity::new("example", "example::Worker", 1, 1, "method", 1),
/// );
/// let value = Box::pin(7_u8);
/// let Err(failure) = PinnedRefInvocation::new(value.as_ref(), [])
///     .validate(&identity, &[ArgumentExpectation::owned::<u8>()])
/// else {
///     panic!("one required argument is missing");
/// };
/// let recovery: PinnedRefInvocationRecovery<'_, u8, Local> = failure.into_recovery();
/// assert_eq!(*recovery.receiver().get_ref(), 7);
/// ```
pub struct PinnedRefInvocationRecovery<'call, T: ?Sized, M: InvocationMode> {
    pub(in crate::invoke::pinned) receiver: Pin<&'call T>,
    pub(in crate::invoke::pinned) invocation: crate::invoke::Invocation<'call, M>,
}

impl<'call, T: ?Sized, M: InvocationMode> PinnedRefInvocationRecovery<'call, T, M> {
    /// Returns the recovered pinned receiver.
    ///
    /// # Returns
    ///
    /// Returns the original pinned shared receiver.
    #[must_use]
    #[inline]
    pub const fn receiver(&self) -> Pin<&'call T> {
        self.receiver
    }

    /// Returns recovered arguments in their original order.
    ///
    /// # Returns
    ///
    /// Returns the caller-ordered argument slice.
    #[must_use]
    #[inline]
    pub fn arguments(&self) -> &[InvocationArg<'call, M>] {
        self.invocation.arguments()
    }

    /// Returns the original name of one recovered caller binding.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based caller binding position.
    ///
    /// # Returns
    ///
    /// Returns the original name, or `None` for positional/out-of-range input.
    #[must_use]
    pub fn argument_name(&self, index: usize) -> Option<&str> {
        self.invocation.argument_name(index)
    }

    /// Reconstitutes the exact typed invocation for inspection or retry.
    ///
    /// # Returns
    ///
    /// Returns the pinned invocation with its complete original input.
    pub fn into_invocation(self) -> PinnedRefInvocation<'call, T, M> {
        PinnedRefInvocation {
            receiver: self.receiver,
            invocation: self.invocation,
        }
    }
}
