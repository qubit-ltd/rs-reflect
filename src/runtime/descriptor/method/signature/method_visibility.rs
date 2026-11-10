// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Visibility recorded for a method declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MethodVisibility {
    /// The method is public.
    Public,
    /// The method has restricted visibility.
    Restricted,
    /// The method is private.
    Private,
}
