// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Dynamic operations supported by reflected field adapters.

use std::fmt;

/// Identifies the kind of dynamic access requested from a reflected field.
///
/// Field adapters use this value to describe reads, mutable borrows, and whole-
/// value replacements in access errors and diagnostics.
///
/// # Examples
///
/// ```
/// use qubit_reflect::access::FieldAccessOperation;
///
/// assert_eq!(FieldAccessOperation::Get.to_string(), "get");
/// assert_eq!(FieldAccessOperation::GetMut.to_string(), "get_mut");
/// assert_eq!(FieldAccessOperation::Set.to_string(), "set");
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FieldAccessOperation {
    /// Shared field access.
    Get,
    /// Mutable field access.
    GetMut,
    /// Whole-value field replacement.
    Set,
}

impl fmt::Display for FieldAccessOperation {
    /// Formats the operation using its public API spelling.
    ///
    /// # Parameters
    ///
    /// - `formatter`: Destination that receives the operation spelling.
    ///
    /// # Returns
    ///
    /// `Ok(())` after writing the spelling.
    ///
    /// # Errors
    ///
    /// Returns the formatter's error if it cannot accept the output.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Get => formatter.write_str("get"),
            Self::GetMut => formatter.write_str("get_mut"),
            Self::Set => formatter.write_str("set"),
        }
    }
}
