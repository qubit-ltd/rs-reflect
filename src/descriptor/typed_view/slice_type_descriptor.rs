// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::__private::LazyTypeRef;
use crate::__private::TypeRefSource;
use crate::descriptor::TypeRef;

/// The typed view of an unsized slice.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let slice = TypeDescriptor::of::<[u8]>().as_slice().expect("slice type");
/// assert!(slice.element_type().as_resolved().is_some());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct SliceTypeDescriptor {
    /// Eager or lazily resolved slice element type.
    element: TypeRefSource,
}

impl SliceTypeDescriptor {
    /// Creates a slice view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `element`: Eager slice element type reference.
    ///
    /// # Returns
    ///
    /// Returns a slice view backed by the supplied type reference.
    pub(crate) const fn new(element: &'static TypeRef) -> Self {
        Self {
            element: TypeRefSource::Eager(element),
        }
    }

    /// Creates a slice view whose element resolves on first navigation.
    ///
    /// # Parameters
    ///
    /// - `element`: Lazy slice element type source.
    ///
    /// # Returns
    ///
    /// Returns a slice view backed by the lazy type source.
    pub(crate) const fn new_lazy(element: &'static LazyTypeRef) -> Self {
        Self {
            element: TypeRefSource::Lazy(element),
        }
    }

    /// Returns the slice element type.
    ///
    /// # Returns
    ///
    /// Returns the static resolved or symbolic element type, initializing a
    /// lazy reference on first access.
    #[must_use]
    #[inline]
    pub fn element_type(&self) -> &'static TypeRef {
        self.element.get()
    }
}
