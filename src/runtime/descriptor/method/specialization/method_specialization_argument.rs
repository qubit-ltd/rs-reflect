// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// A typed generic argument used by a statically registered method
/// specialization.
#[derive(Clone, Copy)]
pub struct MethodSpecializationArgument {
    /// Concrete signed or unsigned constant used in a specialization selector.
    value: MethodSpecializationArgumentValue,
}
