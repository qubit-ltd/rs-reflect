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
///
/// # Examples
///
/// ```
/// #[cfg(feature = "derive")]
/// fn main() {
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// use qubit_reflect::Reflect;
/// use qubit_reflect::TypeDescriptor;
///
/// mod example {
///     use qubit_reflect::Reflect;
///     #[derive(Reflect)]
///     #[reflect(crate = qubit_reflect)]
///     pub struct User {
///         name: String,
///     }
/// }
///
/// let construction = TypeDescriptor::of::<example::User>().struct_construction().expect("generated constructor");
/// let constructor = construction.local_constructor();
/// assert!(constructor.descriptor().type_name().ends_with("User"));
/// }
/// #[cfg(not(feature = "derive"))]
/// fn main() {}
/// ```
pub struct StructConstructionDescriptor {
    /// Factory for the local-mode from-zero constructor.
    local_constructor: fn() -> &'static StructConstructor<Local>,
    /// Optional factory for the local-mode updater.
    local_updater: Option<fn() -> &'static StructUpdater<Local>>,
    /// Lazily cached local constructor instance.
    cached_local_constructor: OnceLock<&'static StructConstructor<Local>>,
    /// Lazily cached local updater availability and instance.
    cached_local_updater: OnceLock<Option<&'static StructUpdater<Local>>>,
    /// Optional factory for the thread-safe constructor.
    thread_safe_constructor: Option<fn() -> &'static StructConstructor<ThreadSafe>>,
    /// Optional factory for the thread-safe updater.
    thread_safe_updater: Option<fn() -> &'static StructUpdater<ThreadSafe>>,
    /// Lazily cached thread-safe constructor availability and instance.
    cached_thread_safe_constructor: OnceLock<Option<&'static StructConstructor<ThreadSafe>>>,
    /// Lazily cached thread-safe updater availability and instance.
    cached_thread_safe_updater: OnceLock<Option<&'static StructUpdater<ThreadSafe>>>,
}

impl StructConstructionDescriptor {
    /// Creates generated entry points for a concrete reflected struct.
    ///
    /// # Parameters
    ///
    /// - `local_constructor`: The generated local-mode constructor factory.
    /// - `local_updater`: The optional generated local-mode updater factory.
    ///
    /// # Returns
    ///
    /// Returns a descriptor that lazily caches the supplied entry points.
    #[doc(hidden)]
    #[must_use]
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
    ///
    /// # Parameters
    ///
    /// - `constructor`: The generated thread-safe constructor factory.
    /// - `updater`: The optional generated thread-safe updater factory.
    ///
    /// # Returns
    ///
    /// Returns this descriptor with its thread-safe entry points attached.
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
    ///
    /// # Returns
    ///
    /// Returns the cached local-mode constructor.
    #[must_use]
    pub fn local_constructor(&self) -> &'static StructConstructor<Local> {
        self.cached_local_constructor.get_or_init(self.local_constructor)
    }

    /// Returns the local owned whole-field updater when generated.
    ///
    /// # Returns
    ///
    /// Returns the cached updater, or `None` when no local updater was
    /// generated.
    #[must_use]
    pub fn local_updater(&self) -> Option<&'static StructUpdater<Local>> {
        *self
            .cached_local_updater
            .get_or_init(|| self.local_updater.map(|factory| factory()))
    }

    /// Returns the thread-safe constructor when the derive declaration opted
    /// into thread-safe adapters and satisfied their bounds.
    ///
    /// # Returns
    ///
    /// Returns the cached thread-safe constructor, or `None` when it is
    /// unavailable.
    #[must_use]
    pub fn thread_safe_constructor(&self) -> Option<&'static StructConstructor<ThreadSafe>> {
        *self
            .cached_thread_safe_constructor
            .get_or_init(|| self.thread_safe_constructor.map(|factory| factory()))
    }

    /// Returns the thread-safe updater when generated.
    ///
    /// # Returns
    ///
    /// Returns the cached thread-safe updater, or `None` when it was not
    /// generated.
    #[must_use]
    pub fn thread_safe_updater(&self) -> Option<&'static StructUpdater<ThreadSafe>> {
        *self
            .cached_thread_safe_updater
            .get_or_init(|| self.thread_safe_updater.map(|factory| factory()))
    }
}
