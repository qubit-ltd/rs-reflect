// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow multiple-public-types
//! Typed invocation inputs that retain a borrowed receiver's pin guarantee.

use std::fmt;
use std::pin::Pin;

use crate::identity::MemberId;
use crate::invoke::ArgumentExpectation;
use crate::invoke::InvocationArg;
use crate::invoke::InvocationBinding;
use crate::invoke::InvocationError;
use crate::invoke::InvocationMode;
use crate::invoke::InvocationOutput;

/// A generated entry point for a method whose receiver is `Pin<&T>`.
pub type PinnedRefAdapter<T, M> =
    for<'call> fn(
        PinnedRefInvocation<'call, T, M>,
    ) -> Result<InvocationOutput<'call, M>, PinnedRefInvocationFailure<'call, T, M>>;

/// A generated entry point for a method whose receiver is `Pin<&mut T>`.
pub type PinnedMutAdapter<T, M> =
    for<'call> fn(
        PinnedMutInvocation<'call, T, M>,
    ) -> Result<InvocationOutput<'call, M>, PinnedMutInvocationFailure<'call, T, M>>;

/// Invocation input for a `Pin<&T>` receiver.
///
/// The receiver is never erased, so this type can invoke methods on `!Unpin`
/// values without reconstructing a pin proof from an ordinary reference.
pub struct PinnedRefInvocation<'call, T: ?Sized, M: InvocationMode> {
    receiver: Pin<&'call T>,
    invocation: crate::invoke::Invocation<'call, M>,
}

impl<'call, T: ?Sized, M: InvocationMode> PinnedRefInvocation<'call, T, M> {
    /// Creates an invocation from a pinned shared receiver and ordered
    /// arguments.
    pub fn new<I>(receiver: Pin<&'call T>, arguments: I) -> Self
    where
        I: IntoIterator<Item = InvocationArg<'call, M>>,
    {
        Self {
            receiver,
            invocation: crate::invoke::Invocation::associated(arguments),
        }
    }

    /// Creates an invocation from a pinned shared receiver and caller-ordered
    /// named or positional bindings.
    ///
    /// A descriptor-aware method-instance entry point validates and reorders
    /// these bindings before the generated pinned adapter runs.
    pub fn from_bindings<I>(receiver: Pin<&'call T>, bindings: I) -> Self
    where
        I: IntoIterator<Item = InvocationBinding<'call, M>>,
    {
        Self {
            receiver,
            invocation: crate::invoke::Invocation::associated_bindings(bindings),
        }
    }

    /// Returns the pinned shared receiver without weakening its pin guarantee.
    #[must_use]
    #[inline(always)]
    pub const fn receiver(&self) -> Pin<&'call T> {
        self.receiver
    }

    /// Returns arguments in their current caller or declaration order.
    #[must_use]
    #[inline(always)]
    pub fn arguments(&self) -> &[InvocationArg<'call, M>] {
        self.invocation.arguments()
    }

    /// Returns the original name of one caller-ordered binding.
    ///
    /// `Some(name)` identifies a named binding. `None` identifies either a
    /// positional binding or an index outside the supplied input range.
    pub fn argument_name(&self, index: usize) -> Option<&str> {
        self.invocation.argument_name(index)
    }

    /// Resolves bindings against one concrete method declaration.
    pub(crate) fn bind_arguments(
        self,
        method_identity: &MemberId,
        parameters: &[crate::descriptor::ParameterDescriptor],
    ) -> Result<Self, PinnedRefInvocationFailure<'call, T, M>> {
        let Self { receiver, invocation } = self;
        match invocation.bind_arguments(method_identity, parameters) {
            Ok(invocation) => Ok(Self { receiver, invocation }),
            Err(failure) => Err(PinnedRefInvocationFailure {
                error: failure.error,
                recovery: PinnedRefInvocationRecovery {
                    receiver,
                    invocation: failure.recovery.into_invocation(),
                },
            }),
        }
    }

    /// Validates argument count, passing modes, and exact erased types.
    ///
    /// On error no input is extracted; the returned failure retains both the
    /// original pin and all arguments for inspection or retry.
    pub fn validate(
        self,
        method_identity: &MemberId,
        arguments: &[ArgumentExpectation],
    ) -> Result<PinnedValidatedRefInvocation<'call, T, M>, PinnedRefInvocationFailure<'call, T, M>> {
        let Self { receiver, invocation } = self;
        match invocation.validate_arguments(method_identity, arguments) {
            Ok(validated) => {
                let (_, arguments) = validated.into_parts();
                Ok(PinnedValidatedRefInvocation { receiver, arguments })
            }
            Err(failure) => Err(PinnedRefInvocationFailure {
                error: failure.error,
                recovery: PinnedRefInvocationRecovery {
                    receiver,
                    invocation: failure.recovery.into_invocation(),
                },
            }),
        }
    }
}

/// Validated input for a `Pin<&T>` receiver method.
pub struct PinnedValidatedRefInvocation<'call, T: ?Sized, M: InvocationMode> {
    receiver: Pin<&'call T>,
    arguments: Box<[InvocationArg<'call, M>]>,
}

impl<'call, T: ?Sized, M: InvocationMode> PinnedValidatedRefInvocation<'call, T, M> {
    /// Consumes validation state and returns the pin proof with its arguments.
    pub fn into_parts(self) -> (Pin<&'call T>, Box<[InvocationArg<'call, M>]>) {
        (self.receiver, self.arguments)
    }
}

/// Complete input retained after a pinned shared invocation fails validation.
pub struct PinnedRefInvocationRecovery<'call, T: ?Sized, M: InvocationMode> {
    receiver: Pin<&'call T>,
    invocation: crate::invoke::Invocation<'call, M>,
}

impl<'call, T: ?Sized, M: InvocationMode> PinnedRefInvocationRecovery<'call, T, M> {
    /// Returns the recovered pinned receiver.
    #[must_use]
    #[inline(always)]
    pub const fn receiver(&self) -> Pin<&'call T> {
        self.receiver
    }

    /// Returns recovered arguments in their original order.
    #[must_use]
    #[inline(always)]
    pub fn arguments(&self) -> &[InvocationArg<'call, M>] {
        self.invocation.arguments()
    }

    /// Returns the original name of one recovered caller binding.
    pub fn argument_name(&self, index: usize) -> Option<&str> {
        self.invocation.argument_name(index)
    }

    /// Reconstitutes the exact typed invocation for inspection or retry.
    pub fn into_invocation(self) -> PinnedRefInvocation<'call, T, M> {
        PinnedRefInvocation {
            receiver: self.receiver,
            invocation: self.invocation,
        }
    }
}

/// A pinned shared invocation validation error and its complete recovery input.
pub struct PinnedRefInvocationFailure<'call, T: ?Sized, M: InvocationMode> {
    /// Structured reason user code was not called.
    pub error: InvocationError,
    /// Original pinned receiver and arguments.
    pub recovery: PinnedRefInvocationRecovery<'call, T, M>,
}

impl<T: ?Sized, M: InvocationMode> fmt::Debug for PinnedRefInvocationFailure<'_, T, M> {
    /// Formats validation metadata without requiring erased arguments to
    /// implement `Debug`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedRefInvocationFailure")
            .field("error", &self.error)
            .finish()
    }
}

impl<T: ?Sized, M: InvocationMode> fmt::Display for PinnedRefInvocationFailure<'_, T, M> {
    /// Formats the underlying structured validation error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl<T: ?Sized, M: InvocationMode> std::error::Error for PinnedRefInvocationFailure<'_, T, M> {
    /// Returns the structured invocation error as the underlying cause.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// Invocation input for a `Pin<&mut T>` receiver.
pub struct PinnedMutInvocation<'call, T: ?Sized, M: InvocationMode> {
    receiver: Pin<&'call mut T>,
    invocation: crate::invoke::Invocation<'call, M>,
}

impl<'call, T: ?Sized, M: InvocationMode> PinnedMutInvocation<'call, T, M> {
    /// Creates an invocation from a pinned mutable receiver and ordered
    /// arguments.
    pub fn new<I>(receiver: Pin<&'call mut T>, arguments: I) -> Self
    where
        I: IntoIterator<Item = InvocationArg<'call, M>>,
    {
        Self {
            receiver,
            invocation: crate::invoke::Invocation::associated(arguments),
        }
    }

    /// Creates an invocation from a pinned mutable receiver and caller-ordered
    /// named or positional bindings.
    ///
    /// A descriptor-aware method-instance entry point validates and reorders
    /// these bindings before the generated pinned adapter runs.
    pub fn from_bindings<I>(receiver: Pin<&'call mut T>, bindings: I) -> Self
    where
        I: IntoIterator<Item = InvocationBinding<'call, M>>,
    {
        Self {
            receiver,
            invocation: crate::invoke::Invocation::associated_bindings(bindings),
        }
    }

    /// Returns arguments in their current caller or declaration order.
    #[must_use]
    #[inline(always)]
    pub fn arguments(&self) -> &[InvocationArg<'call, M>] {
        self.invocation.arguments()
    }

    /// Returns the original name of one caller-ordered binding.
    pub fn argument_name(&self, index: usize) -> Option<&str> {
        self.invocation.argument_name(index)
    }

    /// Resolves bindings against one concrete method declaration.
    pub(crate) fn bind_arguments(
        self,
        method_identity: &MemberId,
        parameters: &[crate::descriptor::ParameterDescriptor],
    ) -> Result<Self, PinnedMutInvocationFailure<'call, T, M>> {
        let Self { receiver, invocation } = self;
        match invocation.bind_arguments(method_identity, parameters) {
            Ok(invocation) => Ok(Self { receiver, invocation }),
            Err(failure) => Err(PinnedMutInvocationFailure {
                error: failure.error,
                recovery: PinnedMutInvocationRecovery {
                    receiver,
                    invocation: failure.recovery.into_invocation(),
                },
            }),
        }
    }

    /// Validates arguments while preserving the pinned mutable receiver on
    /// error.
    pub fn validate(
        self,
        method_identity: &MemberId,
        arguments: &[ArgumentExpectation],
    ) -> Result<PinnedValidatedMutInvocation<'call, T, M>, PinnedMutInvocationFailure<'call, T, M>> {
        let Self { receiver, invocation } = self;
        match invocation.validate_arguments(method_identity, arguments) {
            Ok(validated) => {
                let (_, arguments) = validated.into_parts();
                Ok(PinnedValidatedMutInvocation { receiver, arguments })
            }
            Err(failure) => Err(PinnedMutInvocationFailure {
                error: failure.error,
                recovery: PinnedMutInvocationRecovery {
                    receiver,
                    invocation: failure.recovery.into_invocation(),
                },
            }),
        }
    }
}

/// Validated input for a `Pin<&mut T>` receiver method.
pub struct PinnedValidatedMutInvocation<'call, T: ?Sized, M: InvocationMode> {
    receiver: Pin<&'call mut T>,
    arguments: Box<[InvocationArg<'call, M>]>,
}

impl<'call, T: ?Sized, M: InvocationMode> PinnedValidatedMutInvocation<'call, T, M> {
    /// Consumes validation state and returns the pin proof with its arguments.
    pub fn into_parts(self) -> (Pin<&'call mut T>, Box<[InvocationArg<'call, M>]>) {
        (self.receiver, self.arguments)
    }
}

/// Complete input retained after a pinned mutable invocation fails validation.
pub struct PinnedMutInvocationRecovery<'call, T: ?Sized, M: InvocationMode> {
    receiver: Pin<&'call mut T>,
    invocation: crate::invoke::Invocation<'call, M>,
}

impl<'call, T: ?Sized, M: InvocationMode> PinnedMutInvocationRecovery<'call, T, M> {
    /// Returns a reborrowed pinned mutable receiver.
    pub fn receiver(&mut self) -> Pin<&mut T> {
        self.receiver.as_mut()
    }

    /// Returns recovered arguments in their original order.
    #[must_use]
    #[inline(always)]
    pub fn arguments(&self) -> &[InvocationArg<'call, M>] {
        self.invocation.arguments()
    }

    /// Returns the original name of one recovered caller binding.
    pub fn argument_name(&self, index: usize) -> Option<&str> {
        self.invocation.argument_name(index)
    }

    /// Reconstitutes the exact typed invocation for inspection or retry.
    pub fn into_invocation(self) -> PinnedMutInvocation<'call, T, M> {
        PinnedMutInvocation {
            receiver: self.receiver,
            invocation: self.invocation,
        }
    }
}

/// A pinned mutable invocation validation error and its complete recovery
/// input.
pub struct PinnedMutInvocationFailure<'call, T: ?Sized, M: InvocationMode> {
    /// Structured reason user code was not called.
    pub error: InvocationError,
    /// Original pinned receiver and arguments.
    pub recovery: PinnedMutInvocationRecovery<'call, T, M>,
}

impl<T: ?Sized, M: InvocationMode> fmt::Debug for PinnedMutInvocationFailure<'_, T, M> {
    /// Formats validation metadata without requiring erased arguments to
    /// implement `Debug`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedMutInvocationFailure")
            .field("error", &self.error)
            .finish()
    }
}

impl<T: ?Sized, M: InvocationMode> fmt::Display for PinnedMutInvocationFailure<'_, T, M> {
    /// Formats the underlying structured validation error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl<T: ?Sized, M: InvocationMode> std::error::Error for PinnedMutInvocationFailure<'_, T, M> {
    /// Returns the structured invocation error as the underlying cause.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}
