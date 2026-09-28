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

/// The typed view of a fixed-length array descriptor.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let array = TypeDescriptor::of::<[u8; 4]>().as_array().expect("array type");
/// assert_eq!(array.length(), 4);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ArrayTypeDescriptor {
    /// Eager or lazily resolved element type.
    element: TypeRefSource,
    /// Fixed number of array elements.
    length: usize,
}

impl ArrayTypeDescriptor {
    /// Creates an array view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `element`: Eager element type reference.
    /// - `length`: Fixed array length.
    ///
    /// # Returns
    ///
    /// Returns an array view backed by the supplied type reference.
    pub(crate) const fn new(element: &'static TypeRef, length: usize) -> Self {
        Self {
            element: TypeRefSource::Eager(element),
            length,
        }
    }

    /// Creates an array view whose element resolves on first navigation.
    ///
    /// # Parameters
    ///
    /// - `element`: Lazy source for the element type.
    /// - `length`: Fixed array length.
    ///
    /// # Returns
    ///
    /// Returns an array view backed by the lazy type reference.
    pub(crate) const fn new_lazy(element: &'static LazyTypeRef, length: usize) -> Self {
        Self {
            element: TypeRefSource::Lazy(element),
            length,
        }
    }

    /// Returns the repeated element type.
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

    /// Returns the compile-time array length.
    ///
    /// # Returns
    ///
    /// Returns the fixed number of elements.
    #[must_use]
    #[inline]
    pub const fn length(&self) -> usize {
        self.length
    }
}
