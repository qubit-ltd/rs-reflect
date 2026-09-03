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
    pub(crate) receiver: Option<ReceiverPlan>,
    pub(crate) parameters: Vec<ParameterPlan>,
    pub(crate) output: OutputPlan,
    pub(crate) modes: AdapterModes,
    pub(crate) availability: AvailabilityPlan,
}

impl InvocationPlan {
    /// Returns whether an executable adapter can be emitted.
    pub(crate) fn is_executable(&self) -> bool {
        matches!(self.availability, AvailabilityPlan::Executable)
    }

    /// Returns pinned receiver mutability when this is a pinned invocation.
    pub(crate) fn pinned_receiver_mutability(&self) -> Option<bool> {
        match self.receiver.as_ref() {
            Some(ReceiverPlan::Pinned { mutable }) => Some(*mutable),
            _ => None,
        }
    }

    /// Returns the standard owned receiver type retained for emission.
    pub(crate) fn owned_receiver_type(&self) -> Option<&TokenStream> {
        match self.receiver.as_ref() {
            Some(ReceiverPlan::OwnedContainer(receiver)) => Some(receiver),
            _ => None,
        }
    }

    /// Returns the extension receiver type retained for emission.
    pub(crate) fn extension_receiver_type(&self) -> Option<&TokenStream> {
        match self.receiver.as_ref() {
            Some(ReceiverPlan::Extension(receiver)) => Some(receiver),
            _ => None,
        }
    }

    /// Returns the number of analyzed positional parameters.
    pub(crate) fn parameter_count(&self) -> usize {
        self.parameters.len()
    }
}

/// Receiver facts retained for adapter emission.
#[derive(Clone, Debug)]
pub(crate) enum ReceiverPlan {
    Value,
    SharedReference,
    MutableReference,
    OwnedContainer(TokenStream),
    Pinned { mutable: bool },
    Extension(TokenStream),
    Unsupported,
}

/// One parameter's validated invocation facts.
#[derive(Clone, Debug)]
pub(crate) struct ParameterPlan {
    pub(crate) index: usize,
    pub(crate) supported: bool,
    pub(crate) unsupported_unsized: bool,
}

/// Validated output category retained for adapter emission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OutputPlan {
    Unit,
    Never,
    Owned,
    SharedBorrow,
    MutableBorrow,
    Opaque,
    Unsupported,
}

/// Requested local, thread-safe, catching, and asynchronous modes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AdapterModes {
    pub(crate) thread_safe: bool,
    pub(crate) catching: bool,
    pub(crate) asynchronous: bool,
}

/// Whether code can emit an executable adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AvailabilityPlan {
    Executable,
    DescribedOnly(Vec<UnavailableReasonPlan>),
}

/// One stable reason an adapter cannot be emitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UnavailableReasonPlan {
    UnsupportedReceiver,
    UnspecializedGeneric,
    UnsafeMethod,
    UnsupportedAbi,
    Variadic,
    UnsupportedBorrowedReturn,
    OpaqueReturn,
    UnsupportedUnsizedValue,
    UnprovenDefaultConstraint,
    UnprovenAssociatedType,
    PinnedModeConflict,
    DisabledByPolicy,
}
