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
///
/// # Examples
///
/// ```
/// use qubit_reflect::Reflect;
/// use qubit_reflect::TypeDescriptor;
///
/// #[derive(Reflect)]
/// enum State {
///     Ready,
/// }
///
/// let variant = TypeDescriptor::of::<State>().variant("Ready").expect("derived variant");
/// let construction = variant.construction().expect("generated constructor");
/// assert_eq!(construction.local_constructor().variant().rust_name(), "Ready");
/// ```
pub struct VariantConstructionDescriptor {
    /// Factory for the local-mode variant constructor.
    local_constructor: fn() -> &'static VariantConstructor<Local>,
    /// Lazily cached local constructor instance.
    cached_local_constructor: OnceLock<&'static VariantConstructor<Local>>,
    /// Optional factory for the thread-safe constructor.
    thread_safe_constructor: Option<fn() -> &'static VariantConstructor<ThreadSafe>>,
    /// Lazily cached thread-safe constructor instance.
    cached_thread_safe_constructor: OnceLock<&'static VariantConstructor<ThreadSafe>>,
}

impl VariantConstructionDescriptor {
    /// Creates a generated local owned variant-construction entry point.
    ///
    /// # Parameters
    ///
    /// - `local_constructor`: The generated local-mode constructor factory.
    ///
    /// # Returns
    ///
    /// Returns a descriptor that lazily caches its local constructor.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn new(local_constructor: fn() -> &'static VariantConstructor<Local>) -> Self {
        Self {
            local_constructor,
            cached_local_constructor: OnceLock::new(),
            thread_safe_constructor: None,
            cached_thread_safe_constructor: OnceLock::new(),
        }
    }

    /// Attaches a generated thread-safe constructor for this variant.
    ///
    /// # Parameters
    ///
    /// - `constructor`: The generated thread-safe constructor factory.
    ///
    /// # Returns
    ///
    /// Returns this descriptor with its thread-safe constructor attached.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn with_thread_safe(mut self, constructor: fn() -> &'static VariantConstructor<ThreadSafe>) -> Self {
        self.thread_safe_constructor = Some(constructor);
        self
    }

    /// Returns the local owned constructor for this variant.
    ///
    /// # Returns
    ///
    /// Returns the cached local-mode constructor.
    #[must_use]
    pub fn local_constructor(&self) -> &'static VariantConstructor<Local> {
        self.cached_local_constructor.get_or_init(self.local_constructor)
    }

    /// Returns the thread-safe constructor when the declaring enum opted in.
    ///
    /// # Returns
    ///
    /// Returns the cached thread-safe constructor, or `None` when unavailable.
    #[must_use]
    pub fn thread_safe_constructor(&self) -> Option<&'static VariantConstructor<ThreadSafe>> {
        self.thread_safe_constructor
            .map(|constructor| *self.cached_thread_safe_constructor.get_or_init(constructor))
    }
}
