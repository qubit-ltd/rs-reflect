// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Maps provider fields to positions in the validated input sequence.
pub(super) struct Binding {
    /// Input position for each provider field, or `None` when omitted.
    pub(super) input_indices: Vec<Option<usize>>,
}
