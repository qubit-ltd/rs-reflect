// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// The declared kind and optional type expression for a return value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReturnKind {
    /// The method does not return normally.
    Never,
    /// The method returns unit.
    Unit,
    /// The method returns an owned value.
    Owned,
    /// The method returns a shared reference.
    Shared,
    /// The method returns an optional shared reference.
    OptionalShared,
    /// The method returns a shared slice.
    SharedSlice,
    /// The method returns a mutable reference.
    Mutable,
    /// The method returns a future.
    Future,
    /// The return type is retained only as opaque metadata.
    Opaque,
}
