// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Descriptor-queryable construction entry points for reflected enum variants.

use crate::construct::VariantConstructor;
use crate::value::Local;

/// Immutable construction entry point for one reflected enum variant.
#[derive(Clone, Copy)]
pub struct VariantConstructionDescriptor {
    local_constructor: fn() -> &'static VariantConstructor<Local>,
}

impl VariantConstructionDescriptor {
    /// Creates a generated local owned variant-construction entry point.
    #[doc(hidden)]
    pub const fn new(local_constructor: fn() -> &'static VariantConstructor<Local>) -> Self {
        Self { local_constructor }
    }

    /// Returns the local owned constructor for this variant.
    #[must_use]
    #[inline(always)]
    pub fn local_constructor(&self) -> &'static VariantConstructor<Local> {
        (self.local_constructor)()
    }
}
