// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Captured panic payloads from reflected invocation.

use std::any::Any;
use std::fmt;

use crate::identity::MemberId;

/// A user panic captured only by an explicitly generated catching adapter.
///
/// Ordinary invocation does not construct this type and propagates panic
/// unchanged. The structured member identity remains available independently
/// of the panic payload's unstable diagnostic text.
///
/// # Examples
///
/// ```
/// use qubit_reflect::identity::{FragmentIdentity, MemberId};
/// use qubit_reflect::invoke::InvocationPanic;
///
/// let identity = MemberId::new(
///     "example::Worker",
///     "run",
///     0,
///     FragmentIdentity::new("example", "example::Worker", 1, 1, "method", 1),
/// );
/// let panic = InvocationPanic::new(identity, Box::new(String::from("failed")));
/// assert_eq!(panic.payload().downcast_ref::<String>().map(String::as_str), Some("failed"));
/// ```
#[must_use]
pub struct InvocationPanic {
    method_identity: Box<MemberId>,
    payload: Box<dyn Any + Send>,
}

impl InvocationPanic {
    /// Creates a caught-panic value retaining method identity and payload.
    ///
    /// # Parameters
    ///
    /// - `method_identity`: Identity of the method that panicked.
    /// - `payload`: Original panic payload captured by the adapter.
    ///
    /// # Returns
    ///
    /// Returns a panic record retaining the identity and payload.
    #[must_use = "the panic record retains the method identity and payload"]
    pub fn new(method_identity: MemberId, payload: Box<dyn Any + Send>) -> Self {
        Self {
            method_identity: Box::new(method_identity),
            payload,
        }
    }

    /// Returns the structured identity of the method that panicked.
    ///
    /// # Returns
    ///
    /// Returns the exact method member identity.
    #[must_use]
    #[inline]
    pub fn method_identity(&self) -> &MemberId {
        &self.method_identity
    }

    /// Returns the retained panic payload without interpreting its text.
    ///
    /// # Returns
    ///
    /// Returns the original opaque panic payload.
    #[must_use]
    #[inline]
    pub fn payload(&self) -> &(dyn Any + Send) {
        self.payload.as_ref()
    }

    /// Extracts a payload of exact type `T`.
    ///
    /// A type mismatch returns the original caught-panic value without losing
    /// its method identity or payload.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete panic payload type requested by the caller.
    ///
    /// # Returns
    ///
    /// Returns the payload as `T`, or the original panic record on mismatch.
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
