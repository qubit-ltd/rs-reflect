// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Why the effective implementation came from its current source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MethodImplementationSource {
    /// The implementation defines the method body directly.
    Declared,
    /// The method is required by the trait and has no default body.
    Required,
    /// The implementation uses the trait's default body.
    Defaulted,
    /// The implementation overrides a trait default body.
    Overridden,
}
