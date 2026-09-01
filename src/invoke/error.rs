// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Structured invocation validation and caught-panic errors.

use std::any::Any;
use std::fmt;

use crate::identity::MemberId;
use crate::invoke::InvocationInputMode;

/// The machine-readable reason pre-execution invocation validation failed.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum InvocationErrorKind {
    /// The supplied receiver shape differs from the method signature.
    #[error("invocation receiver mode mismatch: expected {expected:?}, got {actual:?}")]
    ReceiverModeMismatch {
        /// Required receiver mode, or `None` for an associated function.
        expected: Option<InvocationInputMode>,
        /// Supplied receiver mode, or `None` when no receiver was supplied.
        actual: Option<InvocationInputMode>,
    },
    /// The supplied receiver has the wrong exact Rust type.
    #[error("invocation receiver type mismatch")]
    ReceiverTypeMismatch {
        /// Exact expected process-local Rust identity.
        expected: std::any::TypeId,
        /// Exact actual process-local Rust identity.
        actual: std::any::TypeId,
        /// Expected Rust type name retained for diagnostics.
        expected_name: &'static str,
    },
    /// A method requires an explicitly registered receiver conversion
    /// capability, but its target descriptor has none for the receiver type.
    #[error("invocation receiver adapter is not registered for `{expected_name}`")]
    ReceiverAdapterUnavailable {
        /// The explicit receiver type required by the method signature.
        expected_name: &'static str,
    },
    /// A registered receiver conversion rejected the supplied receiver while
    /// returning it untouched for recovery.
    #[error("invocation receiver adapter rejected the supplied receiver for `{expected_name}`")]
    ReceiverAdapterRejected {
        /// The explicit receiver type required by the method signature.
        expected_name: &'static str,
    },
    /// The number of supplied positional arguments differs from the signature.
    #[error("invocation argument count mismatch: expected {expected}, got {actual}")]
    ArgumentCountMismatch {
        /// Required positional argument count.
        expected: usize,
        /// Supplied positional argument count.
        actual: usize,
    },
    /// A named binding does not match any declared parameter name.
    #[error("invocation argument {input_index} has unknown name `{name}`")]
    UnknownArgumentName {
        /// Zero-based index in the caller's original binding order.
        input_index: usize,
        /// Caller-supplied name.
        name: Box<str>,
    },
    /// A name matches more than one declared parameter.
    #[error("invocation argument {input_index} has ambiguous name `{name}`")]
    AmbiguousArgumentName {
        /// Zero-based index in the caller's original binding order.
        input_index: usize,
        /// Caller-supplied name.
        name: Box<str>,
        /// Declaration-order parameter indices sharing the name.
        parameter_indices: Box<[usize]>,
    },
    /// A name refers to a wildcard or destructuring parameter.
    #[error("invocation argument {input_index} cannot bind parameter {parameter_index} by name `{name}`")]
    NamedArgumentUnavailable {
        /// Zero-based index in the caller's original binding order.
        input_index: usize,
        /// Declaration-order parameter index.
        parameter_index: usize,
        /// Caller-supplied name.
        name: Box<str>,
    },
    /// More than one input attempts to bind the same parameter.
    #[error("invocation argument {input_index} duplicates parameter {parameter_index}")]
    DuplicateArgumentBinding {
        /// Zero-based index in the caller's original binding order.
        input_index: usize,
        /// Declaration-order parameter index already occupied.
        parameter_index: usize,
    },
    /// A positional input remains after every parameter is occupied.
    #[error("invocation positional argument {input_index} has no unoccupied parameter")]
    PositionalArgumentOverflow {
        /// Zero-based index in the caller's original binding order.
        input_index: usize,
    },
    /// No supplied input binds one declared parameter.
    #[error("invocation is missing parameter {parameter_index}")]
    MissingArgumentBinding {
        /// Declaration-order parameter index.
        parameter_index: usize,
        /// Identifier name when the parameter has one.
        name: Option<&'static str>,
    },
    /// A raw adapter received a named binding without method descriptors.
    #[error("invocation argument {input_index} named `{name}` requires descriptor-aware binding")]
    NamedBindingRequiresDescriptor {
        /// Zero-based index in the caller's original binding order.
        input_index: usize,
        /// Caller-supplied name.
        name: Box<str>,
    },
    /// One argument has an incompatible ownership or borrowing mode.
    #[error("invocation argument {index} mode mismatch: expected {expected:?}, got {actual:?}")]
    ArgumentModeMismatch {
        /// Zero-based positional argument index, excluding the receiver.
        index: usize,
        /// Required passing mode.
        expected: InvocationInputMode,
        /// Supplied passing mode.
        actual: InvocationInputMode,
    },
    /// One argument has the wrong exact Rust type.
    #[error("invocation argument {index} type mismatch")]
    ArgumentTypeMismatch {
        /// Zero-based positional argument index, excluding the receiver.
        index: usize,
        /// Exact expected process-local Rust identity.
        expected: std::any::TypeId,
        /// Exact actual process-local Rust identity.
        actual: std::any::TypeId,
        /// Expected Rust type name retained for diagnostics.
        expected_name: &'static str,
    },
}

/// A pre-execution invocation validation error with method context.
///
/// Every error is produced before any owned receiver or argument is extracted.
/// The enclosing invocation failure therefore carries complete recovery input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvocationError {
    method_identity: Box<MemberId>,
    kind: Box<InvocationErrorKind>,
}

impl InvocationError {
    /// Creates an invocation error for one exact method member.
    pub fn new(method_identity: MemberId, kind: InvocationErrorKind) -> Self {
        Self {
            method_identity: Box::new(method_identity),
            kind: Box::new(kind),
        }
    }

    /// Returns the structured identity of the method being invoked.
    #[must_use]
    #[inline(always)]
    pub fn method_identity(&self) -> &MemberId {
        &self.method_identity
    }

    /// Returns the stable machine-readable validation reason.
    #[must_use]
    #[inline(always)]
    pub const fn kind(&self) -> &InvocationErrorKind {
        &self.kind
    }
}

impl fmt::Display for InvocationError {
    /// Formats method context followed by the non-stable kind diagnostic.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "failed to invoke {} at index {} on `{}`: {}",
            self.method_identity.kind(),
            self.method_identity.index(),
            self.method_identity.declaring_identity(),
            self.kind,
        )
    }
}

impl std::error::Error for InvocationError {
    /// Returns the machine-readable validation kind as the underlying cause.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.kind.as_ref())
    }
}

/// A user panic captured only by an explicitly generated catching adapter.
///
/// Ordinary invocation does not construct this type and propagates panic
/// unchanged. The structured member identity remains available independently
/// of the panic payload's unstable diagnostic text.
pub struct InvocationPanic {
    method_identity: Box<MemberId>,
    payload: Box<dyn Any + Send>,
}

impl InvocationPanic {
    /// Creates a caught-panic value retaining method identity and payload.
    pub fn new(method_identity: MemberId, payload: Box<dyn Any + Send>) -> Self {
        Self {
            method_identity: Box::new(method_identity),
            payload,
        }
    }

    /// Returns the structured identity of the method that panicked.
    #[must_use]
    #[inline(always)]
    pub fn method_identity(&self) -> &MemberId {
        &self.method_identity
    }

    /// Returns the retained panic payload without interpreting its text.
    #[must_use]
    #[inline(always)]
    pub fn payload(&self) -> &(dyn Any + Send) {
        self.payload.as_ref()
    }

    /// Extracts a payload of exact type `T`.
    ///
    /// A type mismatch returns the original caught-panic value without losing
    /// its method identity or payload.
    pub fn downcast_payload<T: Any + Send>(self) -> Result<T, Self> {
        let Self {
            method_identity,
            payload,
        } = self;
        match payload.downcast::<T>() {
            Ok(payload) => Ok(*payload),
            Err(payload) => Err(Self {
                method_identity,
                payload,
            }),
        }
    }
}

impl fmt::Debug for InvocationPanic {
    /// Formats method identity while leaving the opaque payload uninterpreted.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InvocationPanic")
            .field("method_identity", &self.method_identity())
            .field("payload_type_id", &self.payload().type_id())
            .finish()
    }
}

impl fmt::Display for InvocationPanic {
    /// Formats a diagnostic that does not make payload text a stable protocol.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "reflected {} at index {} on `{}` panicked",
            self.method_identity.kind(),
            self.method_identity.index(),
            self.method_identity.declaring_identity(),
        )
    }
}

impl std::error::Error for InvocationPanic {}
