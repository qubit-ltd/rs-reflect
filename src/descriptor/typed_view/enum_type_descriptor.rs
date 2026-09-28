// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::descriptor::EnumRepr;

/// The typed view of a declared enum.
///
/// # Examples
///
/// ```
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// #[cfg(feature = "derive")]
/// fn main() {
/// use qubit_reflect::Reflect;
/// use qubit_reflect::TypeDescriptor;
/// #[derive(Reflect)]
/// #[reflect(crate = qubit_reflect)]
/// enum State { Ready }
/// let state = TypeDescriptor::of::<State>().as_enum().expect("enum type");
/// assert!(state.representations().is_empty());
/// }
/// #[cfg(not(feature = "derive"))]
/// fn main() {}
/// ```
#[derive(Clone, Copy, Debug)]
pub struct EnumTypeDescriptor {
    /// Explicit representation components in canonical order.
    representations: &'static [EnumRepr],
}

impl EnumTypeDescriptor {
    /// Creates an enum view from normalized explicit representation metadata.
    ///
    /// # Parameters
    ///
    /// - `representations`: Explicit components in canonical order.
    ///
    /// # Returns
    ///
    /// Returns an enum view retaining those representation components.
    pub(crate) const fn new(representations: &'static [EnumRepr]) -> Self {
        Self { representations }
    }

    /// Returns normalized explicit `repr(...)` components.
    ///
    /// An empty slice means the enum has no explicit representation
    /// declaration. Components use canonical order and never contain
    /// diagnostic text.
    ///
    /// # Returns
    ///
    /// Returns the explicit representation components, or an empty slice
    /// when no `repr` attribute was declared.
    #[must_use]
    #[inline]
    pub const fn representations(&self) -> &'static [EnumRepr] {
        self.representations
    }
}
