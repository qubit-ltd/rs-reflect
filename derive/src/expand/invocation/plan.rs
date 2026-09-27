// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Parser-independent invocation plan produced before token emission.

// qubit-style: allow multiple-public-types

use proc_macro2::TokenStream;

/// Complete invocation facts shared by impl and trait expansion.
#[derive(Clone, Debug)]
pub(crate) struct InvocationPlan {
    /// Classified receiver shape, when the method has a receiver.
    pub(crate) receiver: Option<ReceiverPlan>,
    /// Per-parameter safety and layout facts in declaration order.
    pub(crate) parameters: Vec<ParameterPlan>,
    /// Classified output representation.
    pub(crate) output: OutputPlan,
    /// Requested execution modes.
    pub(crate) modes: AdapterModes,
    /// Whether an executable adapter is available and why otherwise.
    pub(crate) availability: AvailabilityPlan,
}

impl InvocationPlan {
    /// Returns whether an executable adapter can be emitted.
    ///
    /// # Returns
    ///
    /// Returns `true` when the availability plan is executable.
    pub(crate) fn is_executable(&self) -> bool {
        matches!(self.availability, AvailabilityPlan::Executable)
    }

    /// Returns pinned receiver mutability when this is a pinned invocation.
    ///
    /// # Returns
    ///
    /// Returns `Some(true)` for mutable pin, `Some(false)` for shared pin, or
    /// `None`.
    pub(crate) fn pinned_receiver_mutability(&self) -> Option<bool> {
        match self.receiver.as_ref() {
            Some(ReceiverPlan::Pinned { mutable }) => Some(*mutable),
            _ => None,
        }
    }

    /// Returns the standard owned receiver type retained for emission.
    ///
    /// # Returns
    ///
    /// Returns the owned container type, or `None` for other receiver plans.
    pub(crate) fn owned_receiver_type(&self) -> Option<&TokenStream> {
        match self.receiver.as_ref() {
            Some(ReceiverPlan::OwnedContainer(receiver)) => Some(receiver),
            _ => None,
        }
    }

    /// Returns the extension receiver type retained for emission.
    ///
    /// # Returns
    ///
    /// Returns the extension receiver type, or `None` for other receiver plans.
    pub(crate) fn extension_receiver_type(&self) -> Option<&TokenStream> {
        match self.receiver.as_ref() {
            Some(ReceiverPlan::Extension(receiver)) => Some(receiver),
            _ => None,
        }
    }

    /// Returns the number of analyzed positional parameters.
    ///
    /// # Returns
    ///
    /// Returns the parameter count retained by this plan.
    pub(crate) fn parameter_count(&self) -> usize {
        self.parameters.len()
    }
}

/// Receiver facts retained for adapter emission.
#[derive(Clone, Debug)]
pub(crate) enum ReceiverPlan {
    /// Method consumes its receiver by value.
    Value,
    /// Method receives a shared reference.
    SharedReference,
    /// Method receives an exclusive mutable reference.
    MutableReference,
    /// Method receives a standard owned container around the target.
    OwnedContainer(TokenStream),
    /// Method receives a pinned shared or mutable reference.
    Pinned { mutable: bool },
    /// Method receives a non-core explicit receiver using an adapter
    /// capability.
    Extension(TokenStream),
    /// Receiver syntax has no supported dynamic adaptation.
    Unsupported,
}

/// One parameter's validated invocation facts.
#[derive(Clone, Debug)]
pub(crate) struct ParameterPlan {
    /// Zero-based position in the method signature.
    pub(crate) index: usize,
    /// Whether the parameter has a supported dynamic representation.
    pub(crate) supported: bool,
    /// Whether it contains a reference to an unsupported unsized value.
    pub(crate) unsupported_unsized: bool,
}

/// Validated output category retained for adapter emission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OutputPlan {
    /// Method returns `()`.
    Unit,
    /// Method returns the never type `!`.
    Never,
    /// Method returns an owned sized value.
    Owned,
    /// Method returns a shared borrow.
    SharedBorrow,
    /// Method returns an exclusive mutable borrow.
    MutableBorrow,
    /// Method returns an opaque `impl Trait` value.
    Opaque,
    /// Return form has no supported dynamic representation.
    Unsupported,
}

/// Requested local, thread-safe, catching, and asynchronous modes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AdapterModes {
    /// Whether the adapter must enforce thread-safe bounds.
    pub(crate) thread_safe: bool,
    /// Whether the adapter should catch unwinding panics.
    pub(crate) catching: bool,
    /// Whether the method is asynchronous.
    pub(crate) asynchronous: bool,
}

/// Whether code can emit an executable adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AvailabilityPlan {
    /// The method can be called through a generated adapter.
    Executable,
    /// Only metadata is available, with ordered reasons explaining why.
    DescribedOnly(Vec<UnavailableReasonPlan>),
}

/// One stable reason an adapter cannot be emitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UnavailableReasonPlan {
    /// Receiver shape cannot be adapted safely.
    UnsupportedReceiver,
    /// Generic parameters have no concrete specialization.
    UnspecializedGeneric,
    /// Method is declared unsafe.
    UnsafeMethod,
    /// Method uses a non-default ABI.
    UnsupportedAbi,
    /// Method is variadic.
    Variadic,
    /// Borrowed output lifetime cannot be retained safely.
    UnsupportedBorrowedReturn,
    /// Method returns an opaque `impl Trait` value.
    OpaqueReturn,
    /// A parameter or return contains an unsupported unsized value.
    UnsupportedUnsizedValue,
    /// A trait default method has predicates that cannot be proven.
    UnprovenDefaultConstraint,
    /// A trait default method depends on an unproven associated type.
    UnprovenAssociatedType,
    /// Pinned receiver conflicts with requested adapter modes.
    PinnedModeConflict,
    /// A reflection attribute disables invocation.
    DisabledByPolicy,
}
