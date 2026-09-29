// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Owned associated-item facts initialized by an impl definition.

use crate::descriptor::ImplAssociatedConstDescriptor;
use crate::descriptor::ImplAssociatedTypeDescriptor;

/// Associated-item slices owned by an impl definition after initialization.
///
/// The owner installs this container once and retains each slice in source
/// order for the lifetime of the definition.
#[derive(Debug)]
pub(super) struct ImplAssociatedItems {
    /// Owned explicit associated type bindings in source order.
    pub(super) types: Box<[ImplAssociatedTypeDescriptor]>,
    /// Owned explicit associated constant bindings in source order.
    pub(super) consts: Box<[ImplAssociatedConstDescriptor]>,
}
