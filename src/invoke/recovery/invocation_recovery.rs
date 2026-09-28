// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Complete input retained after invocation validation failure.

use std::fmt;

use crate::invoke::Invocation;
use crate::invoke::InvocationArg;
use crate::invoke::InvocationMode;
use crate::invoke::InvocationReceiver;

/// Complete invocation input retained after pre-execution validation fails.
///
/// # Examples
///
/// ```
/// use qubit_reflect::identity::{FragmentIdentity, MemberId};
/// use qubit_reflect::invoke::{Invocation, InvocationArg, ReceiverExpectation};
/// use qubit_reflect::value::{DynamicOwned, Local};
///
/// let identity = MemberId::new(
///     "example::Worker",
///     "run",
///     0,
///     FragmentIdentity::new("example", "example::Worker", 1, 1, "method", 1),
/// );
/// let failure = Invocation::associated([InvocationArg::Owned(DynamicOwned::<Local>::new(5_u8))])
///     .validate(&identity, ReceiverExpectation::owned::<()>(), &[])
///     .expect_err("an owned receiver is required");
/// let recovery = failure.into_recovery();
/// assert_eq!(recovery.arguments().len(), 1);
/// ```
pub struct InvocationRecovery<'call, M: InvocationMode> {
    receiver: Option<InvocationReceiver<'call, M>>,
    arguments: Box<[InvocationArg<'call, M>]>,
    argument_names: Box<[Option<Box<str>>]>,
}

impl<'call, M: InvocationMode> InvocationRecovery<'call, M> {
    /// Creates recovery from the untouched invocation input.
    ///
    /// # Parameters
    ///
    /// - `receiver`: Original receiver, if the invocation had one.
    /// - `arguments`: Original arguments in caller order.
    /// - `argument_names`: Original names for named bindings.
    ///
    /// # Returns
    ///
    /// Returns the recovery payload retaining the complete input.
    pub(crate) fn new(
        receiver: Option<InvocationReceiver<'call, M>>,
        arguments: Box<[InvocationArg<'call, M>]>,
        argument_names: Box<[Option<Box<str>>]>,
    ) -> Self {
        Self {
            receiver,
            arguments,
            argument_names,
        }
    }

    /// Returns the recovered receiver, or `None` for an associated function.
    ///
    /// # Returns
    ///
    /// Returns the receiver by shared reference, or `None` when absent.
    #[must_use]
    #[inline]
    pub const fn receiver(&self) -> Option<&InvocationReceiver<'call, M>> {
        self.receiver.as_ref()
    }

    /// Returns all recovered arguments in their original caller order.
    ///
    /// # Returns
    ///
    /// Returns the recovered argument slice.
    #[must_use]
    #[inline]
    pub fn arguments(&self) -> &[InvocationArg<'call, M>] {
        &self.arguments
    }

    /// Returns the original name of one recovered caller binding.
    ///
    /// `Some(name)` identifies a named binding. `None` identifies either a
    /// positional binding or an index outside the recovered input range.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based position in the original caller bindings.
    ///
    /// # Returns
    ///
    /// Returns the original name, or `None` for positional or out-of-range
    /// input.
    #[must_use]
    pub fn argument_name(&self, index: usize) -> Option<&str> {
        self.argument_names.get(index).and_then(|name| name.as_deref())
    }

    /// Consumes the recovery and returns the receiver and caller-ordered
    /// arguments.
    ///
    /// # Returns
    ///
    /// Returns the optional receiver and caller-ordered arguments.
    pub fn into_parts(self) -> (Option<InvocationReceiver<'call, M>>, Box<[InvocationArg<'call, M>]>) {
        (self.receiver, self.arguments)
    }

    /// Reconstitutes the exact invocation so a caller can inspect or retry it.
    ///
    /// # Returns
    ///
    /// Returns an invocation with the original receiver, values, order, and
    /// binding names.
    pub fn into_invocation(self) -> Invocation<'call, M> {
        Invocation::from_parts(self.receiver, self.arguments, self.argument_names)
    }
}

impl<M: InvocationMode> fmt::Debug for InvocationRecovery<'_, M> {
    /// Formats input modes and count without requiring erased values to be
    /// `Debug`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InvocationRecovery")
            .field("receiver_mode", &self.receiver.as_ref().map(InvocationReceiver::mode))
            .field("argument_count", &self.arguments.len())
            .field(
                "argument_names",
                &self
                    .argument_names
                    .iter()
                    .map(|name| name.as_deref())
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}
