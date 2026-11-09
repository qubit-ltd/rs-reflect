// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Source categories for an optional shared borrow returned by a method.

/// One source whose shared borrow can be returned by an optional reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OptionalBorrowOrigin {
    /// The method receiver is the source of the returned reference.
    Receiver,
    /// A method parameter is the source of the returned reference.
    Parameter(
        /// Zero-based source parameter index.
        usize,
    ),
}
