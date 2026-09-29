// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Owned member facts shared by applications of one trait declaration.

use crate::descriptor::AssociatedConstDescriptor;
use crate::descriptor::AssociatedTypeDescriptor;
use crate::descriptor::MethodDescriptor;

/// Member slices owned by a trait definition after one-time initialization.
///
/// The definition retains these facts in source order before any concrete
/// application exists and shares them across all applications.
#[derive(Debug)]
pub(super) struct TraitDefinitionMembers {
    /// Owned declared methods in source order.
    pub(super) methods: Box<[MethodDescriptor]>,
    /// Owned declared associated types in source order.
    pub(super) associated_types: Box<[AssociatedTypeDescriptor]>,
    /// Owned declared associated constants in source order.
    pub(super) associated_consts: Box<[AssociatedConstDescriptor]>,
}
