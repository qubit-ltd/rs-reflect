// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Invocation input validated against one generated method signature.

use std::fmt;

use super::binding_recovery::BindingRecovery;
use crate::identity::MemberId;
use crate::invoke::Invocation;
use crate::invoke::InvocationArg;
use crate::invoke::InvocationErrorKind;
use crate::invoke::InvocationFailure;
use crate::invoke::InvocationMode;
use crate::invoke::InvocationReceiver;
use crate::invoke::ReceiverAdaptationResult;

/// Invocation input proven to match one generated method adapter signature.
///
/// Owned values remain inside their dynamic wrappers until the adapter consumes
/// this state, so validating later inputs cannot lose earlier owned inputs.
///
/// # Type Parameters
///
/// - `'call`: Lifetime shared by borrowed receivers and arguments.
/// - `M`: Dynamic ownership mode of the validated inputs.
///
/// # Examples
///
/// ```
/// use qubit_reflect::identity::{FragmentIdentity, MemberId};
/// use qubit_reflect::invoke::{ArgumentExpectation, Invocation, InvocationArg};
/// use qubit_reflect::value::{DynamicOwned, Local};
///
/// let method = MemberId::new(
///     "example::Worker",
///     "run",
///     0,
///     FragmentIdentity::new("example", "example::Worker", 1, 1, "method", 1),
/// );
/// let validated = Invocation::<Local>::associated([
///     InvocationArg::Owned(DynamicOwned::<Local>::new(7_u8)),
/// ])
/// .validate_arguments(&method, &[ArgumentExpectation::owned::<u8>()])
/// .expect("the argument has the expected type and mode");
/// assert_eq!(validated.arguments().len(), 1);
/// ```
pub struct ValidatedInvocation<'call, M: InvocationMode> {
    /// Receiver after descriptor-aware validation.
    pub(in crate::invoke::invocation) receiver: Option<InvocationReceiver<'call, M>>,
    /// Arguments reordered into declaration parameter order.
    pub(in crate::invoke::invocation) arguments: Box<[InvocationArg<'call, M>]>,
    /// Names aligned with the declaration-ordered arguments.
    pub(in crate::invoke::invocation) argument_names: Box<[Option<Box<str>>]>,
    /// Recovery map retained for adapter execution failures.
    pub(in crate::invoke::invocation) binding_recovery: Option<Box<BindingRecovery>>,
}

impl<'call, M: InvocationMode> ValidatedInvocation<'call, M> {
    /// Returns the validated receiver, or `None` for an associated function.
    ///
    /// # Returns
    ///
    /// Returns the validated receiver by shared reference, or `None` when
    /// absent.
    #[must_use]
    #[inline]
    pub const fn receiver(&self) -> Option<&InvocationReceiver<'call, M>> {
        self.receiver.as_ref()
    }

    /// Returns the validated positional arguments in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the validated arguments in parameter order.
    #[must_use]
    #[inline]
    pub fn arguments(&self) -> &[InvocationArg<'call, M>] {
        &self.arguments
    }

    /// Resolves an explicit receiver adapter from the caller-selected registry.
    ///
    /// Intrinsic capability conflicts are returned with every
    /// input intact, including caller-order named bindings. A valid absent
    /// adapter is reported separately by [`Self::adapt_receiver`].
    /// No process-global registry is consulted. The registry borrow does not
    /// escape into the adapted receiver or recovery.
    ///
    /// # Type Parameters
    ///
    /// - `R`: Concrete receiver type produced by the registered adapter.
    ///
    /// # Parameters
    ///
    /// - `registry`: Caller-selected snapshot containing receiver capabilities.
    /// - `method_identity`: Identity attached to any validation failure.
    /// - `descriptor`: Reflected target whose capabilities are queried.
    ///
    /// # Returns
    ///
    /// Returns the adapted receiver and declaration-ordered arguments.
    ///
    /// # Errors
    ///
    /// Returns a capability-resolution error for an invalid intrinsic set,
    /// an unavailable error for an absent, fact-only, or mistyped adapter,
    /// or a rejection error if the adapter rejects the receiver. Every such
    /// failure retains inputs and names in their original caller order.
    ///
    /// # Panics
    ///
    /// Propagates panics from capability providers and receiver adapters.
    pub fn adapt_receiver_in<R: 'static>(
        self,
        registry: &crate::registry::ReflectRegistry,
        method_identity: &MemberId,
        descriptor: &crate::descriptor::TypeDescriptor,
    ) -> ReceiverAdaptationResult<'call, R, M>
    where
        M: 'static,
    {
        let adapter = match registry.capability_lookup(descriptor, crate::invoke::receiver_adapter_key::<R, M>()) {
            Ok(crate::capability::CapabilityLookup::Found(adapter)) => Some(adapter),
            Ok(
                crate::capability::CapabilityLookup::Missing
                | crate::capability::CapabilityLookup::FactOnly(_)
                | crate::capability::CapabilityLookup::AdapterTypeMismatch { .. },
            ) => None,
            Err(error) => {
                return Err(self.reject(method_identity, InvocationErrorKind::CapabilityResolution(error)));
            }
        };
        self.adapt_receiver(method_identity, adapter)
    }

    /// Rejects validated input while restoring the caller's original bindings.
    fn reject(self, method_identity: &MemberId, kind: InvocationErrorKind) -> InvocationFailure<'call, M> {
        Invocation {
            receiver: self.receiver,
            arguments: self.arguments,
            argument_names: self.argument_names,
            binding_recovery: self.binding_recovery,
        }
        .reject(method_identity, kind)
    }

    /// Applies an optional explicit-receiver capability without losing the
    /// invocation recovery retained during descriptor-aware argument binding.
    ///
    /// A missing capability or rejected receiver restores arguments to exact
    /// caller order, including every original named-binding label. Successful
    /// conversion returns declaration-ordered arguments for generated method
    /// extraction.
    ///
    /// # Type Parameters
    ///
    /// - `R`: Concrete receiver type produced by the adapter.
    ///
    /// # Parameters
    ///
    /// - `method_identity`: Identity attached to any validation failure.
    /// - `adapter`: Explicit receiver conversion, or `None` when unavailable.
    ///
    /// # Returns
    ///
    /// Returns the adapted receiver and declaration-ordered arguments.
    ///
    /// # Errors
    ///
    /// Returns a structured failure with the complete original invocation when
    /// the receiver is absent, no adapter exists, or the adapter rejects it.
    ///
    /// # Panics
    ///
    /// Propagates panics raised by the receiver adapter.
    pub fn adapt_receiver<R: 'static>(
        self,
        method_identity: &MemberId,
        adapter: Option<&crate::invoke::ReceiverAdapter<R, M>>,
    ) -> ReceiverAdaptationResult<'call, R, M> {
        let Self {
            receiver,
            arguments,
            argument_names,
            binding_recovery,
        } = self;
        let expected_name = std::any::type_name::<R>();
        let Some(receiver) = receiver else {
            return Err(Invocation {
                receiver: None,
                arguments,
                argument_names,
                binding_recovery,
            }
            .reject(
                method_identity,
                InvocationErrorKind::ReceiverAdapterRejected { expected_name },
            ));
        };
        let Some(adapter) = adapter else {
            return Err(Invocation {
                receiver: Some(receiver),
                arguments,
                argument_names,
                binding_recovery,
            }
            .reject(
                method_identity,
                InvocationErrorKind::ReceiverAdapterUnavailable { expected_name },
            ));
        };
        match adapter(receiver) {
            Ok(receiver) => Ok((receiver, arguments)),
            Err(receiver) => Err(Invocation {
                receiver: Some(receiver),
                arguments,
                argument_names,
                binding_recovery,
            }
            .reject(
                method_identity,
                InvocationErrorKind::ReceiverAdapterRejected { expected_name },
            )),
        }
    }

    /// Consumes the validation state so an adapter may extract owned values.
    ///
    /// # Returns
    ///
    /// Returns the receiver and declaration-ordered arguments. The receiver is
    /// `Some` for a receiver method and `None` for an associated function.
    #[must_use]
    pub fn into_parts(self) -> (Option<InvocationReceiver<'call, M>>, Box<[InvocationArg<'call, M>]>) {
        (self.receiver, self.arguments)
    }
}

impl<M: InvocationMode> fmt::Debug for ValidatedInvocation<'_, M> {
    /// Formats modes and argument count without requiring erased values to be
    /// `Debug`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValidatedInvocation")
            .field("receiver_mode", &self.receiver.as_ref().map(InvocationReceiver::mode))
            .field("argument_count", &self.arguments.len())
            .finish()
    }
}
