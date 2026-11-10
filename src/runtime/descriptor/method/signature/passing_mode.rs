// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// How a method parameter receives its value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PassingMode {
    /// The argument is passed by value.
    Owned,
    /// The argument is passed through a shared reference.
    Shared,
    /// The argument is passed through a mutable reference.
    Mutable,
}
