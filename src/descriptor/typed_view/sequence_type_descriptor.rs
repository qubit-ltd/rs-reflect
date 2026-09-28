// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::SequenceKind;
use crate::__private::LazyTypeRef;
use crate::__private::TypeRefSource;
use crate::descriptor::TypeRef;

/// The typed view of an ordered sequence descriptor.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let sequence = TypeDescriptor::of::<Vec<u8>>().as_sequence().expect("sequence type");
/// assert_eq!(sequence.kind(), qubit_reflect::descriptor::SequenceKind::Vec);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct SequenceTypeDescriptor {
    /// Standard-library sequence family.
    kind: SequenceKind,
    /// Eager or lazily resolved element type.
    element: TypeRefSource,
}

impl SequenceTypeDescriptor {
    /// Creates a sequence view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `kind`: Standard-library sequence family.
    /// - `element`: Eager element type reference.
    ///
    /// # Returns
    ///
    /// Returns a sequence view backed by the supplied type reference.
    pub(crate) const fn new(kind: SequenceKind, element: &'static TypeRef) -> Self {
        Self {
            kind,
            element: TypeRefSource::Eager(element),
        }
    }

    /// Creates a sequence view whose element resolves on first navigation.
    ///
    /// # Parameters
    ///
    /// - `kind`: Standard-library sequence family.
    /// - `element`: Lazy source for the element type.
    ///
    /// # Returns
    ///
    /// Returns a sequence view backed by the lazy type reference.
    pub(crate) const fn new_lazy(kind: SequenceKind, element: &'static LazyTypeRef) -> Self {
        Self {
            kind,
            element: TypeRefSource::Lazy(element),
        }
    }

    /// Returns the concrete standard-library sequence family.
    ///
    /// # Returns
    ///
    /// Returns the sequence family represented by this view.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> SequenceKind {
        self.kind
    }

    /// Returns the sequence element type.
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
