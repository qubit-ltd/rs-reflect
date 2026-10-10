// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Machine-readable invocation validation failures.

use crate::invoke::InvocationInputMode;

/// The machine-readable reason pre-execution invocation validation failed.
///
/// # Examples
///
/// ```
/// use qubit_reflect::invoke::InvocationErrorKind;
///
/// let kind = InvocationErrorKind::ArgumentCountMismatch { expected: 2, actual: 1 };
/// assert!(kind.to_string().contains("expected 2, got 1"));
/// ```
#[must_use]
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum InvocationErrorKind {
    /// Intrinsic explicit receiver capabilities contain conflicting facts.
    #[error("receiver capability resolution failed: {0}")]
    CapabilityResolution(#[source] crate::capability::CapabilityConflict),
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
