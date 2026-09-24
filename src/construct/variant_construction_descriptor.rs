// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Descriptor-queryable construction entry points for reflected enum variants.

use std::sync::OnceLock;

use crate::construct::VariantConstructor;
use crate::value::Local;
use crate::value::ThreadSafe;

/// Immutable construction entry point for one reflected enum variant.
pub struct VariantConstructionDescriptor {
    local_constructor: fn() -> &'static VariantConstructor<Local>,
    cached_local_constructor: OnceLock<&'static VariantConstructor<Local>>,
    thread_safe_constructor: Option<fn() -> &'static VariantConstructor<ThreadSafe>>,
    cached_thread_safe_constructor: OnceLock<&'static VariantConstructor<ThreadSafe>>,
}

impl VariantConstructionDescriptor {
    /// Creates a generated local owned variant-construction entry point.
    #[doc(hidden)]
    pub const fn new(local_constructor: fn() -> &'static VariantConstructor<Local>) -> Self {
        Self {
            local_constructor,
            cached_local_constructor: OnceLock::new(),
            thread_safe_constructor: None,
            cached_thread_safe_constructor: OnceLock::new(),
        }
    }

    /// Attaches a generated thread-safe constructor for this variant.
    #[doc(hidden)]
    #[must_use]
    pub const fn with_thread_safe(mut self, constructor: fn() -> &'static VariantConstructor<ThreadSafe>) -> Self {
        self.thread_safe_constructor = Some(constructor);
        self
    }

    /// Returns the local owned constructor for this variant.
    #[must_use]
    #[inline(always)]
    pub fn local_constructor(&self) -> &'static VariantConstructor<Local> {
        self.cached_local_constructor.get_or_init(self.local_constructor)
    }

    /// Returns the thread-safe constructor when the declaring enum opted in.
    #[must_use]
    pub fn thread_safe_constructor(&self) -> Option<&'static VariantConstructor<ThreadSafe>> {
        self.thread_safe_constructor
            .map(|constructor| *self.cached_thread_safe_constructor.get_or_init(constructor))
    }
}
