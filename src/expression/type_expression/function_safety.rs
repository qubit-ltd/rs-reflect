// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Structural representations of Rust type expressions.

//! FunctionSafety structure and operations.

/// A function pointer's safety qualifier.
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::FunctionSafety;
/// let safety = FunctionSafety::Unsafe;
/// assert_eq!(safety, FunctionSafety::Unsafe);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FunctionSafety {
    /// A safe function pointer.
    Safe,
    /// An `unsafe fn` pointer.
    Unsafe,
}
