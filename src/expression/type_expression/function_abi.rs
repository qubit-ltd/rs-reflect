// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Structural representations of Rust type expressions.

//! FunctionAbi structure and operations.

use crate::expression::ExpressionName;

/// A function pointer's calling convention.
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::FunctionAbi;
/// let abi = FunctionAbi::C;
/// assert_eq!(abi, FunctionAbi::C);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FunctionAbi {
    /// The default Rust ABI.
    Rust,
    /// The C ABI.
    C,
    /// The platform system ABI.
    System,
    /// Any explicitly named ABI not covered by a standard variant.
    Other(ExpressionName),
}
