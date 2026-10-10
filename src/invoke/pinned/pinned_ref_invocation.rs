// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed pinned invocation components.

use std::pin::Pin;

use super::pinned_ref_invocation_failure::PinnedRefInvocationFailure;
use super::pinned_ref_invocation_recovery::PinnedRefInvocationRecovery;
use super::pinned_validated_ref_invocation::PinnedValidatedRefInvocation;
use crate::descriptor::ParameterDescriptor;
use crate::identity::MemberId;
use crate::invoke::ArgumentExpectation;
use crate::invoke::Invocation;
use crate::invoke::InvocationArg;
use crate::invoke::InvocationBinding;
use crate::invoke::InvocationMode;

/// Invocation input for a `Pin<&T>` receiver.
///
/// # Type Parameters
///
/// - `'call`: Lifetime retained by borrowed receiver and arguments.
/// - `T`: Concrete receiver type kept behind the pin.
/// - `M`: Dynamic ownership mode of invocation arguments.
///
/// The receiver is never erased, so this type can invoke methods on `!Unpin`
/// values without reconstructing a pin proof from an ordinary reference.
///
/// # Examples
///
/// ```
/// use qubit_reflect::invoke::PinnedRefInvocation;
/// use qubit_reflect::value::Local;
///
/// let value = Box::pin(7_u8);
/// let invocation = PinnedRefInvocation::<_, Local>::new(value.as_ref(), []);
/// assert_eq!(*invocation.receiver().get_ref(), 7);
/// ```
pub struct PinnedRefInvocation<'call, T: ?Sized, M: InvocationMode> {
    pub(in crate::invoke::pinned) receiver: Pin<&'call T>,
    pub(in crate::invoke::pinned) invocation: Invocation<'call, M>,
}

impl<'call, T: ?Sized, M: InvocationMode> PinnedRefInvocation<'call, T, M> {
    /// Creates an invocation from a pinned shared receiver and ordered
    /// arguments.
    ///
    /// # Type Parameters
    ///
    /// - `I`: Iterable collection of positional arguments.
    ///
    /// # Parameters
    ///
    /// - `receiver`: Pinned shared receiver retained for the call lifetime.
    /// - `arguments`: Positional arguments in caller order.
    ///
    /// # Returns
    ///
    /// Returns a typed pinned invocation.
    pub fn new<I>(receiver: Pin<&'call T>, arguments: I) -> Self
    where
        I: IntoIterator<Item = InvocationArg<'call, M>>,
    {
        Self {
            receiver,
            invocation: Invocation::associated(arguments),
        }
    }

    /// Creates an invocation from a pinned shared receiver and caller-ordered
    /// named or positional bindings.
    ///
    /// A descriptor-aware method-instance entry point validates and reorders
    /// these bindings before the generated pinned adapter runs.
    ///
    /// # Type Parameters
    ///
    /// - `I`: Iterable collection of named or positional bindings.
    ///
    /// # Parameters
    ///
    /// - `receiver`: Pinned shared receiver retained for the call lifetime.
    /// - `bindings`: Named or positional arguments in caller order.
    ///
    /// # Returns
    ///
    /// Returns a typed pinned invocation retaining those bindings.
    pub fn from_bindings<I>(receiver: Pin<&'call T>, bindings: I) -> Self
    where
        I: IntoIterator<Item = InvocationBinding<'call, M>>,
    {
        Self {
            receiver,
            invocation: Invocation::associated_bindings(bindings),
        }
    }

    /// Returns the pinned shared receiver without weakening its pin guarantee.
    ///
    /// # Returns
    ///
    /// Returns the original pinned shared receiver.
    #[must_use]
    #[inline]
    pub const fn receiver(&self) -> Pin<&'call T> {
        self.receiver
    }

    /// Returns arguments in their current caller or declaration order.
    ///
    /// # Returns
    ///
    /// Returns the positional argument slice.
    #[must_use]
    #[inline]
    pub fn arguments(&self) -> &[InvocationArg<'call, M>] {
        self.invocation.arguments()
    }

    /// Returns the original name of one caller-ordered binding.
    ///
    /// `Some(name)` identifies a named binding. `None` identifies either a
    /// positional binding or an index outside the supplied input range.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based caller binding position.
    ///
    /// # Returns
    ///
    /// Returns the original binding name, or `None` when absent.
    #[must_use]
    pub fn argument_name(&self, index: usize) -> Option<&str> {
        self.invocation.argument_name(index)
    }

    /// Resolves bindings against one concrete method declaration.
    ///
    /// # Parameters
    ///
    /// - `method_identity`: Identity attached to a binding failure.
    /// - `parameters`: Method parameters in declaration order.
    ///
    /// # Returns
    ///
    /// Returns a typed invocation with arguments ordered for the declaration.
    ///
    /// # Errors
    ///
    /// Returns a pinned failure retaining the original receiver and bindings.
    /// Validates argument count, passing modes, and exact erased types.
    ///
    /// On error no input is extracted; the returned failure retains both the
    /// original pin and all arguments for inspection or retry.
    ///
    /// # Parameters
    ///
    /// - `method_identity`: Method identity attached to any failure.
    /// - `arguments`: Expected argument modes and exact types.
    ///
    /// # Returns
    ///
    /// Returns validated pinned input.
    ///
    /// # Errors
    ///
    /// Returns a structured validation failure with the pin and all arguments.
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

    /// Resolves bindings against one concrete method declaration.
    ///
    /// # Parameters
    ///
    /// - `method_identity`: Identity attached to a binding failure.
    /// - `parameters`: Method parameters in declaration order.
    ///
    /// # Returns
    ///
    /// Returns a typed invocation with arguments ordered for the declaration.
    ///
    /// # Errors
    ///
    /// Returns a pinned failure retaining the original receiver and bindings.
    pub(crate) fn bind_arguments(
        self,
        method_identity: &MemberId,
        parameters: &[ParameterDescriptor],
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
}
