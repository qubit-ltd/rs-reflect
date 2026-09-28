// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! parameter passing mode definitions.

/// How a non-receiver parameter is passed to a reflected method.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::ParameterPassingMode;
/// assert_eq!(ParameterPassingMode::SharedBorrow, ParameterPassingMode::SharedBorrow);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ParameterPassingMode {
    /// The method consumes an owned argument.
    Owned,
    /// The method borrows an argument immutably.
    SharedBorrow,
    /// The method borrows an argument mutably.
    MutableBorrow,
}
