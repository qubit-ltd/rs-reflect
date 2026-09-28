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

/// The typed view of an optional descriptor.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let optional = TypeDescriptor::of::<Option<u8>>().as_optional().expect("optional type");
/// assert!(optional.element_type().as_resolved().is_some());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct OptionalTypeDescriptor {
    /// Eager or lazily resolved optional element type.
    element: TypeRefSource,
}

impl OptionalTypeDescriptor {
    /// Creates an optional view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `element`: Eager optional element type reference.
    ///
    /// # Returns
    ///
    /// Returns an optional view backed by the supplied type reference.
    pub(crate) const fn new(element: &'static TypeRef) -> Self {
        Self {
            element: TypeRefSource::Eager(element),
        }
    }

    /// Creates an optional view whose element is resolved on first
    /// navigation.
    ///
    /// # Parameters
    ///
    /// - `element`: Lazy source for the optional element type.
    ///
    /// # Returns
    ///
    /// Returns an optional view backed by the lazy type reference.
    pub(crate) const fn new_lazy(element: &'static LazyTypeRef) -> Self {
        Self {
            element: TypeRefSource::Lazy(element),
        }
    }

    /// Returns the optional element type.
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
