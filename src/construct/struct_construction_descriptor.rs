// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Descriptor-queryable construction entry points for reflected structs.

use std::sync::OnceLock;

use crate::construct::StructConstructor;
use crate::construct::StructUpdater;
use crate::value::Local;

/// Immutable construction entry points for one reflected struct root.
pub struct StructConstructionDescriptor {
    local_constructor: fn() -> &'static StructConstructor<Local>,
    local_updater: Option<fn() -> &'static StructUpdater<Local>>,
    cached_local_constructor: OnceLock<&'static StructConstructor<Local>>,
    cached_local_updater: OnceLock<Option<&'static StructUpdater<Local>>>,
}

impl StructConstructionDescriptor {
    /// Creates generated entry points for a concrete reflected struct.
    #[doc(hidden)]
    pub const fn new(
        local_constructor: fn() -> &'static StructConstructor<Local>,
        local_updater: Option<fn() -> &'static StructUpdater<Local>>,
    ) -> Self {
        Self {
            local_constructor,
            local_updater,
            cached_local_constructor: OnceLock::new(),
            cached_local_updater: OnceLock::new(),
        }
    }

    /// Returns the local owned from-zero constructor.
    #[must_use]
    #[inline(always)]
    pub fn local_constructor(&self) -> &'static StructConstructor<Local> {
        self.cached_local_constructor.get_or_init(self.local_constructor)
    }

    /// Returns the local owned whole-field updater when generated.
    #[must_use]
    #[inline(always)]
    pub fn local_updater(&self) -> Option<&'static StructUpdater<Local>> {
        *self
            .cached_local_updater
            .get_or_init(|| self.local_updater.map(|factory| factory()))
    }
}
