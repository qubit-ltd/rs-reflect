// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! FunctionPointerKind category metadata.

/// The safety qualifier of a function pointer.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::FunctionPointerKind;
/// let kind = FunctionPointerKind::Safe;
/// assert_eq!(kind, FunctionPointerKind::Safe);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FunctionPointerKind {
    /// A safe function pointer.
    Safe,
    /// An unsafe function pointer.
    Unsafe,
}
