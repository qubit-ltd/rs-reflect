//! Recovery payloads for invocation failures before user code runs.

use std::fmt;

use crate::invoke::Invocation;
use crate::invoke::InvocationArg;
use crate::invoke::InvocationError;
use crate::invoke::InvocationMode;
use crate::invoke::InvocationReceiver;

/// Complete invocation input retained after pre-execution validation fails.
pub struct InvocationRecovery<'call, M: InvocationMode> {
    receiver: Option<InvocationReceiver<'call, M>>,
    arguments: Box<[InvocationArg<'call, M>]>,
}

impl<'call, M: InvocationMode> InvocationRecovery<'call, M> {
    /// Creates recovery from the untouched invocation input.
    pub(crate) fn new(
        receiver: Option<InvocationReceiver<'call, M>>,
        arguments: Box<[InvocationArg<'call, M>]>,
    ) -> Self {
        Self { receiver, arguments }
    }

    /// Returns the recovered receiver, or `None` for an associated function.
    pub const fn receiver(&self) -> Option<&InvocationReceiver<'call, M>> {
        self.receiver.as_ref()
    }

    /// Returns all recovered arguments in their original positional order.
    pub fn arguments(&self) -> &[InvocationArg<'call, M>] {
        &self.arguments
    }

    /// Consumes the recovery and returns the receiver and ordered arguments.
    pub fn into_parts(self) -> (Option<InvocationReceiver<'call, M>>, Box<[InvocationArg<'call, M>]>) {
        (self.receiver, self.arguments)
    }

    /// Reconstitutes the exact invocation so a caller can inspect or retry it.
    pub fn into_invocation(self) -> Invocation<'call, M> {
        let (receiver, arguments) = self.into_parts();
        Invocation::from_parts(receiver, arguments)
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
            .finish()
    }
}

/// A validation error paired with the complete recoverable invocation input.
pub struct InvocationFailure<'call, M: InvocationMode> {
    /// Structured reason the invocation could not enter user code.
    pub error: InvocationError,
    /// Untouched receiver and arguments retained by the runtime.
    pub recovery: InvocationRecovery<'call, M>,
}

impl<M: InvocationMode> fmt::Debug for InvocationFailure<'_, M> {
    /// Formats the error and recovery metadata without formatting erased
    /// values.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InvocationFailure")
            .field("error", &self.error)
            .field("recovery", &self.recovery)
            .finish()
    }
}

impl<M: InvocationMode> fmt::Display for InvocationFailure<'_, M> {
    /// Delegates the human-readable diagnostic to the structured error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl<M: InvocationMode> std::error::Error for InvocationFailure<'_, M> {}
