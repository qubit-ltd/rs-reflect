// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Recoverable invocation dispatch failures with unconstrained input types.

use std::error::Error;
use std::fmt;

use crate::invoke::InvocationDispatchMode;
use crate::invoke::InvocationDispatchReason;

/// An unavailable invocation entry together with the caller's original input.
///
/// Input `I` is retained without validation, extraction or execution. Its own
/// borrowing and thread-safety constraints determine this wrapper's lifetime,
/// `Send` and `Sync` behavior. Diagnostics never format the input and impose no
/// `Debug` or `'static` bound on it.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::InvocationAdapter;
/// use qubit_reflect::invoke::Invocation;
/// use qubit_reflect::invoke::InvocationArg;
/// use qubit_reflect::invoke::InvocationDispatchReason;
/// use qubit_reflect::registry::RegistrySnapshotBuilder;
/// use qubit_reflect::value::DynamicOwned;
/// use qubit_reflect::value::Local;
///
/// // The opaque identity has no callable invocation entries.
/// fn opaque_entry() {}
///
/// let registry = RegistrySnapshotBuilder::new().build().expect("empty registry");
/// let input = Invocation::<Local>::associated([
///     InvocationArg::Owned(DynamicOwned::<Local>::new(47_u8)),
/// ]);
/// let adapter = InvocationAdapter::new(opaque_entry);
/// let unavailable = match adapter.invoke_local(&registry, input) {
///     Err(unavailable) => unavailable,
///     Ok(_) => panic!("opaque adapter must have no local entry"),
/// };
/// assert_eq!(unavailable.reason(), &InvocationDispatchReason::MissingEntry);
/// let recovered = unavailable.into_invocation();
/// assert_eq!(recovered.arguments().len(), 1);
/// let InvocationArg::Owned(value) = &recovered.arguments()[0] else {
///     panic!("dispatch must preserve the original owned argument");
/// };
/// assert_eq!(value.downcast_ref::<u8>(), Some(&47));
/// ```
#[must_use]
pub struct InvocationUnavailable<I> {
    /// Invocation entry requested before dispatch failed.
    mode: InvocationDispatchMode,
    /// Structured explanation of why that entry was unavailable.
    reason: InvocationDispatchReason,
    /// Original input retained without validation or execution.
    invocation: I,
}

impl<I> InvocationUnavailable<I> {
    /// Creates a failure preserving `invocation` for the requested `mode` and
    /// `reason`.
    #[inline]
    pub(crate) fn new(mode: InvocationDispatchMode, reason: InvocationDispatchReason, invocation: I) -> Self {
        Self {
            mode,
            reason,
            invocation,
        }
    }

    /// Returns the invocation entry requested by the caller.
    #[must_use]
    #[inline]
    pub const fn mode(&self) -> InvocationDispatchMode {
        self.mode
    }

    /// Returns the reason the requested entry was unavailable.
    #[must_use]
    #[inline]
    pub const fn reason(&self) -> &InvocationDispatchReason {
        &self.reason
    }

    /// Borrows the original input without modifying it.
    #[must_use]
    #[inline]
    pub const fn invocation(&self) -> &I {
        &self.invocation
    }

    /// Consumes the failure and returns the original input without copying it.
    #[must_use]
    #[inline]
    pub fn into_invocation(self) -> I {
        self.invocation
    }

    /// Consumes the failure and returns its mode, reason and original input.
    #[must_use]
    #[inline]
    pub fn into_parts(self) -> (InvocationDispatchMode, InvocationDispatchReason, I) {
        (self.mode, self.reason, self.invocation)
    }
}

impl<I> fmt::Debug for InvocationUnavailable<I> {
    /// Formats only dispatch context, without requiring or exposing input
    /// diagnostics.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InvocationUnavailable")
            .field("mode", &self.mode)
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

impl<I> fmt::Display for InvocationUnavailable<I> {
    /// Formats the requested mode and its unavailable-entry explanation.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invocation dispatch {:?} unavailable: ", self.mode)?;
        match &self.reason {
            InvocationDispatchReason::NoAdapter { reasons } => {
                write!(formatter, "no invocation adapter ({reasons:?})")
            }
            InvocationDispatchReason::MissingEntry => formatter.write_str("missing invocation entry"),
            InvocationDispatchReason::CatchingNotRequested => formatter.write_str("panic capture was not requested"),
            InvocationDispatchReason::PanicAbort => formatter.write_str("panic=abort prevents panic capture"),
            InvocationDispatchReason::PinnedReceiverTypeMismatch => {
                formatter.write_str("pinned receiver type mismatch")
            }
        }
    }
}

impl<I> Error for InvocationUnavailable<I> {
    /// Returns `None`: entry unavailability has no underlying error source.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}
