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
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode of the recovered value.
///
/// # Examples
///
/// ```
/// use qubit_reflect::construct::RecoveredConstructionValue;
/// use qubit_reflect::value::DynamicOwned;
/// use qubit_reflect::value::Local;
///
/// let value = RecoveredConstructionValue::Named {
///     name: "title".into(),
///     value: DynamicOwned::<Local>::new(String::from("draft")),
/// };
/// assert!(matches!(value, RecoveredConstructionValue::Named { .. }));
/// ```
#[must_use]
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
    ///
    /// # Parameters
    ///
    /// - `formatter`: Destination receiving the recovered binding metadata.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting the binding metadata.
    ///
    /// # Errors
    ///
    /// Returns the formatter's error if it cannot accept the output.
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
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode of the retained inputs.
///
/// # Examples
///
/// ```
/// use qubit_reflect::construct::NamedConstructionInput;
/// use qubit_reflect::value::DynamicOwned;
/// use qubit_reflect::value::Local;
/// use qubit_reflect::Reflect;
/// use qubit_reflect::TypeDescriptor;
///
/// #[derive(Reflect)]
/// struct User {
///     name: String,
/// }
///
/// let constructor = TypeDescriptor::of::<User>()
///     .struct_construction()
///     .expect("derived construction")
///     .local_constructor();
/// let input = NamedConstructionInput::<Local>::new(std::iter::empty::<(&str, DynamicOwned<Local>)>());
/// let recovery = match constructor.construct_named(input) {
///     Err(recovery) => recovery,
///     Ok(_) => panic!("expected missing-field recovery"),
/// };
/// assert!(recovery.to_string().contains("missing required construction field"));
/// ```
#[must_use]
pub struct ConstructionRecovery<M: Mode> {
    /// Structured reason validation stopped before generated code ran.
    error: Box<ConstructionError>,
    /// All caller-owned values retained in the documented recovery order.
    values: Vec<RecoveredConstructionValue<M>>,
}

impl<M: Mode> ConstructionRecovery<M> {
    /// Creates recovery from one structured error and ordered owned values.
    ///
    /// # Type Parameters
    ///
    /// - `M`: The dynamic ownership mode of the retained values.
    ///
    /// # Parameters
    ///
    /// - `error`: The validation error explaining why construction stopped.
    /// - `values`: The untouched caller-owned values in recovery order.
    ///
    /// # Returns
    ///
    /// Returns a recovery object owning the error and all supplied values.
    pub(crate) fn new(error: ConstructionError, values: Vec<RecoveredConstructionValue<M>>) -> Self {
        Self {
            error: Box::new(error),
            values,
        }
    }

    /// Returns the machine-readable validation error.
    ///
    /// # Returns
    ///
    /// Returns the structured error that stopped construction.
    #[must_use]
    #[inline]
    pub const fn error(&self) -> &ConstructionError {
        &self.error
    }

    /// Returns every recovered value in original caller order.
    ///
    /// Update recovery places the base first, followed by overrides in caller
    /// order.
    #[must_use]
    #[inline]
    pub fn values(&self) -> &[RecoveredConstructionValue<M>] {
        &self.values
    }

    /// Consumes recovery and returns the structured error and ordered values.
    ///
    /// # Returns
    ///
    /// Returns the error and all retained values in recovery order.
    #[must_use]
    pub fn into_parts(self) -> (ConstructionError, Box<[RecoveredConstructionValue<M>]>) {
        (*self.error, self.values.into_boxed_slice())
    }

    /// Consumes recovery and returns all owned values in recovery order.
    ///
    /// # Returns
    ///
    /// Returns every retained caller value in documented recovery order.
    #[must_use]
    pub fn into_values(self) -> Box<[RecoveredConstructionValue<M>]> {
        self.values.into_boxed_slice()
    }
}

impl<M: Mode> fmt::Debug for ConstructionRecovery<M> {
    /// Formats the error and recovery metadata without formatting erased
    /// values.
    ///
    /// # Parameters
    ///
    /// - `formatter`: The destination formatter.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting the recovery payload.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if writing to `formatter` fails.
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
    ///
    /// # Parameters
    ///
    /// - `formatter`: The destination formatter.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting the underlying error.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if writing to `formatter` fails.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl<M: Mode> std::error::Error for ConstructionRecovery<M> {}
