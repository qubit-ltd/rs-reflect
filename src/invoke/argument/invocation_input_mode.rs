// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Ownership and borrowing modes for invocation inputs.

/// The ownership or borrowing mode of an invocation input.
///
/// # Examples
///
/// ```
/// use qubit_reflect::invoke::InvocationInputMode;
///
/// assert_eq!(InvocationInputMode::Owned, InvocationInputMode::Owned);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InvocationInputMode {
    /// The input is consumed by the invoked method.
    Owned,
    /// The method receives a shared borrow.
    Ref,
    /// The method receives an exclusive mutable borrow.
    Mut,
}
