// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::__private::LazyTypeRef;
use crate::__private::TypeRefSource;
use crate::descriptor::SmartPointerKind;
use crate::descriptor::TypeRef;

/// The typed view of a standard smart pointer.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let pointer = TypeDescriptor::of::<Box<u8>>().as_smart_pointer().expect("smart pointer type");
/// assert_eq!(pointer.kind(), qubit_reflect::descriptor::SmartPointerKind::Box);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct SmartPointerTypeDescriptor {
    /// Standard smart-pointer family.
    kind: SmartPointerKind,
    /// Eager or lazily resolved pointee type.
    pointee: TypeRefSource,
}

impl SmartPointerTypeDescriptor {
    /// Creates a smart-pointer view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `kind`: Standard smart-pointer family.
    /// - `pointee`: Eager pointee type reference.
    ///
    /// # Returns
    ///
    /// Returns a smart-pointer view backed by the supplied type reference.
    pub(crate) const fn new(kind: SmartPointerKind, pointee: &'static TypeRef) -> Self {
        Self {
            kind,
            pointee: TypeRefSource::Eager(pointee),
        }
    }

    /// Creates a smart-pointer view whose pointee is resolved on first
    /// navigation.
    ///
    /// # Parameters
    ///
    /// - `kind`: Standard smart-pointer family.
    /// - `pointee`: Lazy pointee type source.
    ///
    /// # Returns
    ///
    /// Returns a smart-pointer view backed by the lazy type source.
    pub(crate) const fn new_lazy(kind: SmartPointerKind, pointee: &'static LazyTypeRef) -> Self {
        Self {
            kind,
            pointee: TypeRefSource::Lazy(pointee),
        }
    }

    /// Returns the concrete smart-pointer family.
    ///
    /// # Returns
    ///
    /// Returns the family represented by this view.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> SmartPointerKind {
        self.kind
    }

    /// Returns the pointee type.
    ///
    /// # Returns
    ///
    /// Returns the static resolved or symbolic pointee type, initializing a
    /// lazy reference on first access.
    #[must_use]
    #[inline]
    pub fn pointee_type(&self) -> &'static TypeRef {
        self.pointee.get()
    }
}
