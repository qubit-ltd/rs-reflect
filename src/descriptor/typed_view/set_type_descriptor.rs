// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::SetKind;
use crate::__private::LazyTypeRef;
use crate::__private::TypeRefSource;
use crate::descriptor::TypeRef;

/// The typed view of a set descriptor.
///
/// # Examples
///
/// ```
/// use std::collections::HashSet;
/// use qubit_reflect::TypeDescriptor;
/// let set = TypeDescriptor::of::<HashSet<u8>>().as_set().expect("set type");
/// assert_eq!(set.kind(), qubit_reflect::descriptor::SetKind::HashSet);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct SetTypeDescriptor {
    /// Standard-library set family.
    kind: SetKind,
    /// Eager or lazily resolved element type.
    element: TypeRefSource,
}

impl SetTypeDescriptor {
    /// Creates a set view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `kind`: Standard-library set family.
    /// - `element`: Eager element type reference.
    ///
    /// # Returns
    ///
    /// Returns a set view backed by the supplied type reference.
    pub(crate) const fn new(kind: SetKind, element: &'static TypeRef) -> Self {
        Self {
            kind,
            element: TypeRefSource::Eager(element),
        }
    }

    /// Creates a set view whose element resolves on first navigation.
    ///
    /// # Parameters
    ///
    /// - `kind`: Standard-library set family.
    /// - `element`: Lazy source for the element type.
    ///
    /// # Returns
    ///
    /// Returns a set view backed by the lazy type reference.
    pub(crate) const fn new_lazy(kind: SetKind, element: &'static LazyTypeRef) -> Self {
        Self {
            kind,
            element: TypeRefSource::Lazy(element),
        }
    }

    /// Returns the concrete standard-library set family.
    ///
    /// # Returns
    ///
    /// Returns the set family represented by this view.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> SetKind {
        self.kind
    }

    /// Returns the set element type.
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
