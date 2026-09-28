// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::__private::LazyTypeRef;
use crate::__private::TypeRefSource;
use crate::descriptor::Mutability;
use crate::descriptor::TypeRef;

/// The typed view of a raw pointer.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let pointer = TypeDescriptor::of::<*const u8>().as_raw_pointer().expect("raw pointer type");
/// assert_eq!(pointer.mutability(), qubit_reflect::descriptor::Mutability::Const);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct RawPointerTypeDescriptor {
    /// Const or mutable raw-pointer category.
    mutability: Mutability,
    /// Eager or lazily resolved pointee type.
    pointee: TypeRefSource,
}

impl RawPointerTypeDescriptor {
    /// Creates a raw-pointer view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `mutability`: Const or mutable pointer category.
    /// - `pointee`: Eager pointee type reference.
    ///
    /// # Returns
    ///
    /// Returns a raw-pointer view backed by the supplied type.
    pub(crate) const fn new(mutability: Mutability, pointee: &'static TypeRef) -> Self {
        Self {
            mutability,
            pointee: TypeRefSource::Eager(pointee),
        }
    }

    /// Creates a raw-pointer view whose pointee resolves on first navigation.
    ///
    /// # Parameters
    ///
    /// - `mutability`: Const or mutable pointer category.
    /// - `pointee`: Lazy pointee type source.
    ///
    /// # Returns
    ///
    /// Returns a raw-pointer view backed by the lazy type source.
    pub(crate) const fn new_lazy(mutability: Mutability, pointee: &'static LazyTypeRef) -> Self {
        Self {
            mutability,
            pointee: TypeRefSource::Lazy(pointee),
        }
    }

    /// Returns whether the pointer is const or mutable.
    ///
    /// # Returns
    ///
    /// Returns the pointer's mutability category.
    #[must_use]
    #[inline]
    pub const fn mutability(&self) -> Mutability {
        self.mutability
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
