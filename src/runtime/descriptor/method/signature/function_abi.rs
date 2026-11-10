// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Function ABI facts retained from the source declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FunctionAbi {
    /// Rust's native ABI.
    Rust,
    /// The C ABI.
    C,
    /// The system ABI for the target platform.
    System,
    /// A named ABI other than the standard variants.
    Other(
        /// ABI name written in the source declaration.
        &'static str,
    ),
}
