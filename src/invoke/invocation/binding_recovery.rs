// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Caller-order metadata retained during descriptor-aware binding.

/// Caller-order metadata retained while a generated adapter validates inputs.
pub(in crate::invoke::invocation) struct BindingRecovery {
    /// Caller argument index for each descriptor-ordered argument.
    pub(in crate::invoke::invocation) caller_index_for_argument: Box<[usize]>,
    /// Caller-provided names in original argument order.
    pub(in crate::invoke::invocation) caller_names: Box<[Option<Box<str>>]>,
}
