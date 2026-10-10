// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed pinned invocation components.

use std::pin::Pin;

use super::pinned_mut_invocation::PinnedMutInvocation;
use crate::invoke::Invocation;
use crate::invoke::InvocationArg;
use crate::invoke::InvocationMode;

/// Complete input retained after a pinned mutable invocation fails validation.
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
/// use qubit_reflect::invoke::{ArgumentExpectation, PinnedMutInvocationFailure, PinnedMutInvocationRecovery, PinnedMutInvocation};
/// use qubit_reflect::value::Local;
///
/// let identity = MemberId::new(
///     "example::Worker",
///     "run",
///     0,
///     FragmentIdentity::new("example", "example::Worker", 1, 1, "method", 1),
/// );
/// let mut value = Box::pin(7_u8);
/// let Err(failure) = PinnedMutInvocation::new(value.as_mut(), [])
///     .validate(&identity, &[ArgumentExpectation::owned::<u8>()])
/// else {
///     panic!("one required argument is missing");
/// };
/// let mut recovery: PinnedMutInvocationRecovery<'_, u8, Local> = failure.into_recovery();
/// assert_eq!(*recovery.receiver().as_ref().get_ref(), 7);
/// ```
pub struct PinnedMutInvocationRecovery<'call, T: ?Sized, M: InvocationMode> {
    /// Pinned receiver retained from the failed invocation for inspection or
    /// retry.
    pub(in crate::invoke::pinned) receiver: Pin<&'call mut T>,
    /// Original caller arguments retained in their caller-supplied order.
    pub(in crate::invoke::pinned) invocation: Invocation<'call, M>,
}

impl<'call, T: ?Sized, M: InvocationMode> PinnedMutInvocationRecovery<'call, T, M> {
    /// Returns a reborrowed pinned mutable receiver.
    ///
    /// # Returns
    ///
    /// Returns an exclusive pinned reborrow of the original receiver.
    #[must_use]
    #[inline]
    pub fn receiver(&mut self) -> Pin<&mut T> {
        self.receiver.as_mut()
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
    #[inline]
    pub fn argument_name(&self, index: usize) -> Option<&str> {
        self.invocation.argument_name(index)
    }

    /// Reconstitutes the exact typed invocation for inspection or retry.
    ///
    /// # Returns
    ///
    /// Returns the pinned invocation with its complete original input.
    pub fn into_invocation(self) -> PinnedMutInvocation<'call, T, M> {
        PinnedMutInvocation {
            receiver: self.receiver,
            invocation: self.invocation,
        }
    }
}
