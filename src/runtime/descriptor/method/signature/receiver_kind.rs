// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// The receiver form declared by a method.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiverKind {
    /// The method takes ownership of the receiver.
    Owned,
    /// The method borrows the receiver immutably.
    Shared,
    /// The method borrows the receiver mutably.
    Mutable,
    /// The method takes a pinned shared receiver.
    PinnedShared,
    /// The method takes a pinned mutable receiver.
    PinnedMutable,
}
