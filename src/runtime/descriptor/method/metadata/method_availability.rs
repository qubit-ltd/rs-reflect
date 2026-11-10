// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Callability reasons recorded during registration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MethodAvailability {
    /// The method has a usable invocation adapter.
    Callable,
    /// A generic specialization must be selected before invocation.
    RequiresSpecialization,
    /// Multiple matching specializations are available, so one must be
    /// selected.
    RequiresSpecializationSelection {
        /// Number of matching specializations that require disambiguation.
        count: usize,
    },
    /// The method has no usable invocation adapter for the recorded
    /// implementation.
    Unavailable {
        /// Reasons the method cannot be invoked dynamically.
        reasons: &'static [InvocationUnavailableReason],
    },
}
