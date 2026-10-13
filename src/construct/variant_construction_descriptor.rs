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

/// Immutable construction entry point for one reflected enum variant.
///
/// # Examples
///
/// ```
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// #[cfg(feature = "derive")]
/// fn main() {
/// use qubit_reflect::Reflect;
/// use qubit_reflect::TypeDescriptor;
///
/// mod example {
///     use qubit_reflect::Reflect;
///     #[Reflect]
///     pub enum State { Ready }
/// }
///
/// let variant = TypeDescriptor::of::<example::State>().variant("Ready").expect("derived variant");
/// let construction = variant.construction().expect("generated constructor");
/// assert_eq!(construction.local_constructor().variant().rust_name(), "Ready");
/// }
/// #[cfg(not(feature = "derive"))]
/// fn main() {}
/// ```
pub struct VariantConstructionDescriptor {
    /// Factory for the local-mode variant constructor.
    local_constructor: fn() -> &'static VariantConstructor<Local>,
    /// Lazily cached local constructor instance.
    cached_local_constructor: OnceLock<&'static VariantConstructor<Local>>,
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
        }
    }

    /// Returns the local owned constructor for this variant.
    ///
    /// # Returns
    ///
    /// Returns the cached local-mode constructor.
    pub fn local_constructor(&self) -> &'static VariantConstructor<Local> {
        self.cached_local_constructor.get_or_init(self.local_constructor)
    }
}
