// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed pinned invocation components.

use std::pin::Pin;

use super::pinned_mut_invocation_failure::PinnedMutInvocationFailure;
use super::pinned_mut_invocation_recovery::PinnedMutInvocationRecovery;
use super::pinned_validated_mut_invocation::PinnedValidatedMutInvocation;
use crate::identity::MemberId;
use crate::invoke::ArgumentExpectation;
use crate::invoke::InvocationArg;
use crate::invoke::InvocationBinding;
use crate::invoke::InvocationMode;

/// Invocation input for a `Pin<&mut T>` receiver.
///
/// # Type Parameters
///
/// - `'call`: Lifetime retained by borrowed receiver and arguments.
/// - `T`: Concrete receiver type kept behind the pin.
/// - `M`: Dynamic ownership mode of invocation arguments.
///
/// # Examples
///
/// ```
/// use qubit_reflect::invoke::PinnedMutInvocation;
/// use qubit_reflect::value::Local;
///
/// let mut value = Box::pin(7_u8);
/// let invocation = PinnedMutInvocation::<_, Local>::new(value.as_mut(), []);
/// assert!(invocation.arguments().is_empty());
/// ```
pub struct PinnedMutInvocation<'call, T: ?Sized, M: InvocationMode> {
    pub(in crate::invoke::pinned) receiver: Pin<&'call mut T>,
    pub(in crate::invoke::pinned) invocation: crate::invoke::Invocation<'call, M>,
}

impl<'call, T: ?Sized, M: InvocationMode> PinnedMutInvocation<'call, T, M> {
    /// Creates an invocation from a pinned mutable receiver and ordered
    /// arguments.
    ///
    /// # Type Parameters
    ///
    /// - `I`: Iterable collection of positional arguments.
    ///
    /// # Parameters
    ///
    /// - `receiver`: Pinned mutable receiver retained for the call lifetime.
    /// - `arguments`: Positional arguments in caller order.
    ///
    /// # Returns
    ///
    /// Returns a typed pinned invocation.
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
    ///
    /// # Type Parameters
    ///
    /// - `I`: Iterable collection of named or positional bindings.
    ///
    /// # Parameters
    ///
    /// - `receiver`: Pinned mutable receiver retained for the call lifetime.
    /// - `bindings`: Named or positional arguments in caller order.
    ///
    /// # Returns
    ///
    /// Returns a typed pinned invocation retaining those bindings.
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
