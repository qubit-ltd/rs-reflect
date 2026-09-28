// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! ImplAssociatedTypeDescriptor metadata and behavior.

/// One associated type explicitly bound by an impl definition.
///
/// This descriptor is constructed by generated registration code.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::ImplAssociatedTypeDescriptor;
/// let binding = ImplAssociatedTypeDescriptor::new("Item");
/// assert_eq!(binding.rust_name(), "Item");
/// ```
#[must_use]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImplAssociatedTypeDescriptor {
    /// Name used in the Rust declaration.
    rust_name: &'static str,
}

impl ImplAssociatedTypeDescriptor {
    /// Creates declaration-level associated type binding facts.
    ///
    /// # Parameters
    ///
    /// - `rust_name`: Name used by the associated type declaration.
    ///
    /// # Returns
    ///
    /// Returns the declaration-level binding facts.
    #[doc(hidden)]
    #[must_use = "the associated type declaration facts are required by generated registration"]
    pub const fn new(rust_name: &'static str) -> Self {
        Self { rust_name }
    }

    /// Returns the Rust associated type name.
    ///
    /// # Returns
    ///
    /// Returns the associated type's Rust name.
    #[must_use]
    #[inline]
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }
}
