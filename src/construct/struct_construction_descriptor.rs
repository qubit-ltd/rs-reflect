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
use crate::value::ThreadSafe;

/// Immutable construction entry points for one reflected struct root.
pub struct StructConstructionDescriptor {
    local_constructor: fn() -> &'static StructConstructor<Local>,
    local_updater: Option<fn() -> &'static StructUpdater<Local>>,
    cached_local_constructor: OnceLock<&'static StructConstructor<Local>>,
    cached_local_updater: OnceLock<Option<&'static StructUpdater<Local>>>,
    thread_safe_constructor: Option<fn() -> &'static StructConstructor<ThreadSafe>>,
    thread_safe_updater: Option<fn() -> &'static StructUpdater<ThreadSafe>>,
    cached_thread_safe_constructor: OnceLock<Option<&'static StructConstructor<ThreadSafe>>>,
    cached_thread_safe_updater: OnceLock<Option<&'static StructUpdater<ThreadSafe>>>,
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
            thread_safe_constructor: None,
            thread_safe_updater: None,
            cached_thread_safe_constructor: OnceLock::new(),
            cached_thread_safe_updater: OnceLock::new(),
        }
    }

    /// Attaches generated thread-safe construction entry points.
    #[doc(hidden)]
    #[must_use]
    pub const fn with_thread_safe(
        mut self,
        constructor: fn() -> &'static StructConstructor<ThreadSafe>,
        updater: Option<fn() -> &'static StructUpdater<ThreadSafe>>,
    ) -> Self {
        self.thread_safe_constructor = Some(constructor);
        self.thread_safe_updater = updater;
        self
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

    /// Returns the thread-safe constructor when the derive declaration opted
    /// into thread-safe adapters and satisfied their bounds.
    #[must_use]
    pub fn thread_safe_constructor(&self) -> Option<&'static StructConstructor<ThreadSafe>> {
        *self
            .cached_thread_safe_constructor
            .get_or_init(|| self.thread_safe_constructor.map(|factory| factory()))
    }

    /// Returns the thread-safe updater when generated.
    #[must_use]
    pub fn thread_safe_updater(&self) -> Option<&'static StructUpdater<ThreadSafe>> {
        *self
            .cached_thread_safe_updater
            .get_or_init(|| self.thread_safe_updater.map(|factory| factory()))
    }
}
