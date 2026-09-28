// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! receiver descriptor definitions.

/// The receiver form written by a reflected method declaration.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::ReceiverDescriptor;
/// assert!(matches!(ReceiverDescriptor::Shared, ReceiverDescriptor::Shared));
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ReceiverDescriptor {
    /// A by-value `self` receiver.
    Owned,
    /// A shared `&self` receiver.
    Shared,
    /// An exclusive `&mut self` receiver.
    Mutable,
    /// A supported explicit receiver whose source form is retained.
    Explicit(&'static str),
}
