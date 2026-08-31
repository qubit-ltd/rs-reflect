// qubit-style: allow public-type-layout
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
    argument_names: Box<[Option<Box<str>>]>,
}

impl<'call, M: InvocationMode> InvocationRecovery<'call, M> {
    /// Creates recovery from the untouched invocation input.
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
    pub const fn receiver(&self) -> Option<&InvocationReceiver<'call, M>> {
        self.receiver.as_ref()
    }

    /// Returns all recovered arguments in their original caller order.
    pub fn arguments(&self) -> &[InvocationArg<'call, M>] {
        &self.arguments
    }

    /// Returns the original name of one recovered caller binding.
    ///
    /// `Some(name)` identifies a named binding. `None` identifies either a
    /// positional binding or an index outside the recovered input range.
    pub fn argument_name(&self, index: usize) -> Option<&str> {
        self.argument_names.get(index).and_then(|name| name.as_deref())
    }

    /// Consumes the recovery and returns the receiver and caller-ordered
    /// arguments.
    pub fn into_parts(self) -> (Option<InvocationReceiver<'call, M>>, Box<[InvocationArg<'call, M>]>) {
        (self.receiver, self.arguments)
    }

    /// Reconstitutes the exact invocation so a caller can inspect or retry it.
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

impl<M: InvocationMode> std::error::Error for InvocationFailure<'_, M> {
    /// Returns the structured invocation error as the underlying cause.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}
