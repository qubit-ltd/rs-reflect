// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::__private::LazyTypeRef;
use crate::__private::TypeRefSource;
use crate::descriptor::ReferenceKind;
use crate::descriptor::TypeRef;

/// The typed view of a Rust reference.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let reference = TypeDescriptor::of::<&'static u8>().as_reference().expect("reference type");
/// assert_eq!(reference.kind(), qubit_reflect::descriptor::ReferenceKind::Shared);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ReferenceTypeDescriptor {
    /// Shared or mutable borrowing category.
    kind: ReferenceKind,
    /// Eager or lazily resolved referenced type.
    target: TypeRefSource,
}

impl ReferenceTypeDescriptor {
    /// Creates a reference view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `kind`: Shared or mutable reference category.
    /// - `target`: Eager referenced type.
    ///
    /// # Returns
    ///
    /// Returns a reference view backed by the supplied type.
    pub(crate) const fn new(kind: ReferenceKind, target: &'static TypeRef) -> Self {
        Self {
            kind,
            target: TypeRefSource::Eager(target),
        }
    }

    /// Creates a reference view whose target resolves on first navigation.
    ///
    /// # Parameters
    ///
    /// - `kind`: Shared or mutable reference category.
    /// - `target`: Lazy referenced type source.
    ///
    /// # Returns
    ///
    /// Returns a reference view backed by the lazy type source.
    pub(crate) const fn new_lazy(kind: ReferenceKind, target: &'static LazyTypeRef) -> Self {
        Self {
            kind,
            target: TypeRefSource::Lazy(target),
        }
    }

    /// Returns whether the reference is shared or mutable.
    ///
    /// # Returns
    ///
    /// Returns the reference borrowing category.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> ReferenceKind {
        self.kind
    }

    /// Returns the referenced type.
    ///
    /// # Returns
    ///
    /// Returns the static resolved or symbolic target type, initializing a
    /// lazy reference on first access.
    #[must_use]
    #[inline]
    pub fn target_type(&self) -> &'static TypeRef {
        self.target.get()
    }
}
