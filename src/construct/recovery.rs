// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Complete ownership recovery for pre-execution construction failures.

use std::any::Any;
use std::fmt;

use crate::construct::ConstructionError;
use crate::value::DynamicOwned;
use crate::value::Local;
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
/// use qubit_reflect::construct::RecoveredAnyValue;
///
/// let value = RecoveredAnyValue::Named {
///     name: "title".into(),
///     value: Box::new(String::from("draft")),
/// };
/// assert!(matches!(value, RecoveredAnyValue::Named { .. }));
/// ```
#[must_use]
pub(crate) enum RecoveredConstructionValue<M: Mode> {
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

/// A recovered ordinary Rust value with the caller's original input identity.
#[must_use]
pub enum RecoveredAnyValue {
    /// The owned base value of a failed update.
    Base(Box<dyn Any>),
    /// A named input value, retaining its source name and order.
    Named {
        /// The field name supplied by the caller.
        name: Box<str>,
        /// The original rejected field value.
        value: Box<dyn Any>,
    },
    /// A positional input value, retaining its source index and order.
    Positional {
        /// The original zero-based position.
        index: usize,
        /// The original rejected positional value.
        value: Box<dyn Any>,
    },
}

impl RecoveredConstructionValue<Local> {
    /// Consumes this entry and returns its original value as `Box<dyn Any>`.
    pub fn into_any(self) -> RecoveredAnyValue {
        match self {
            Self::Base(value) => RecoveredAnyValue::Base(
                value
                    .into_any()
                    .unwrap_or_else(|_| unreachable!("local dynamic values contain Any")),
            ),
            Self::Named { name, value } => RecoveredAnyValue::Named {
                name,
                value: value
                    .into_any()
                    .unwrap_or_else(|_| unreachable!("local dynamic values contain Any")),
            },
            Self::Positional { index, value } => RecoveredAnyValue::Positional {
                index,
                value: value
                    .into_any()
                    .unwrap_or_else(|_| unreachable!("local dynamic values contain Any")),
            },
        }
    }
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
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// #[cfg(feature = "derive")]
/// fn main() {
/// use qubit_reflect::construct::NamedConstructionInput;
/// use qubit_reflect::Reflect;
/// use qubit_reflect::TypeDescriptor;
///
/// mod example {
///     use qubit_reflect::Reflect;
///     #[Reflect]
///     pub struct User { pub name: String }
/// }
///
/// let recovery = match TypeDescriptor::of::<example::User>().construct_struct::<example::User>(NamedConstructionInput::new()) {
///     Err(recovery) => recovery,
///     Ok(_) => panic!("expected missing-field recovery"),
/// };
/// assert!(recovery.to_string().contains("missing required construction field"));
/// }
/// #[cfg(not(feature = "derive"))]
/// fn main() {}
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
    #[inline]
    pub const fn error(&self) -> &ConstructionError {
        &self.error
    }

    /// Consumes recovery and returns the structured error and ordered values.
    ///
    /// # Returns
    ///
    /// Returns the error and all retained values in recovery order.
    pub(crate) fn into_parts(self) -> (ConstructionError, Box<[RecoveredConstructionValue<M>]>) {
        (*self.error, self.values.into_boxed_slice())
    }
}

impl ConstructionRecovery<Local> {
    /// Consumes recovery into its error and plain erased Rust values.
    pub fn into_any_parts(self) -> (ConstructionError, Box<[RecoveredAnyValue]>) {
        let (error, values) = self.into_parts();
        (
            error,
            values.into_vec().into_iter().map(|value| value.into_any()).collect(),
        )
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
