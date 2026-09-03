// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Complete ownership recovery for pre-execution construction failures.

use std::fmt;

use crate::construct::ConstructionError;
use crate::value::DynamicOwned;
use crate::value::Mode;

/// One caller-owned value retained after construction validation fails.
pub enum RecoveredConstructionValue<M: Mode> {
    /// The owned base of a failed update, always first in update recovery.
    Base(DynamicOwned<M>),
    /// A named value retaining its original query spelling and caller order.
    Named {
        /// Original caller-supplied field name.
        name: Box<str>,
        /// Untouched caller-owned dynamic value.
        value: DynamicOwned<M>,
    },
    /// A positional value retaining its original zero-based index.
    Positional {
        /// Original caller-supplied position.
        index: usize,
        /// Untouched caller-owned dynamic value.
        value: DynamicOwned<M>,
    },
}

impl<M: Mode> fmt::Debug for RecoveredConstructionValue<M> {
    /// Formats binding metadata without requiring erased values to be `Debug`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Base(_) => formatter.write_str("Base(<value>)"),
            Self::Named { name, .. } => formatter
                .debug_struct("Named")
                .field("name", name)
                .field("value", &"<value>")
                .finish(),
            Self::Positional { index, .. } => formatter
                .debug_struct("Positional")
                .field("index", index)
                .field("value", &"<value>")
                .finish(),
        }
    }
}

/// A validation error paired with every untouched caller-owned input value.
pub struct ConstructionRecovery<M: Mode> {
    error: Box<ConstructionError>,
    values: Vec<RecoveredConstructionValue<M>>,
}

impl<M: Mode> ConstructionRecovery<M> {
    /// Creates recovery from one structured error and ordered owned values.
    pub(crate) fn new(error: ConstructionError, values: Vec<RecoveredConstructionValue<M>>) -> Self {
        Self {
            error: Box::new(error),
            values,
        }
    }

    /// Returns the machine-readable validation error.
    #[must_use]
    #[inline(always)]
    pub const fn error(&self) -> &ConstructionError {
        &self.error
    }

    /// Returns every recovered value in original caller order.
    ///
    /// Update recovery places the base first, followed by overrides in caller
    /// order.
    #[must_use]
    #[inline(always)]
    pub fn values(&self) -> &[RecoveredConstructionValue<M>] {
        &self.values
    }

    /// Consumes recovery and returns the structured error and ordered values.
    #[must_use]
    pub fn into_parts(self) -> (ConstructionError, Box<[RecoveredConstructionValue<M>]>) {
        (*self.error, self.values.into_boxed_slice())
    }

    /// Consumes recovery and returns all owned values in recovery order.
    #[must_use]
    pub fn into_values(self) -> Box<[RecoveredConstructionValue<M>]> {
        self.values.into_boxed_slice()
    }
}

impl<M: Mode> fmt::Debug for ConstructionRecovery<M> {
    /// Formats the error and recovery metadata without formatting erased
    /// values.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConstructionRecovery")
            .field("error", &self.error)
            .field("values", &self.values)
            .finish()
    }
}

impl<M: Mode> fmt::Display for ConstructionRecovery<M> {
    /// Delegates human-readable output to the structured construction error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl<M: Mode> std::error::Error for ConstructionRecovery<M> {}
