// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::__private::LazyTypeRefList;
use crate::__private::TypeRefListSource;
use crate::descriptor::TypeRef;

/// The typed view of a tuple descriptor.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let tuple = TypeDescriptor::of::<(u8, bool)>().as_tuple().expect("tuple type");
/// assert_eq!(tuple.arity(), 2);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct TupleTypeDescriptor {
    /// Eager or lazily resolved tuple element references.
    elements: TypeRefListSource,
}

impl TupleTypeDescriptor {
    /// Creates a tuple view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `elements`: Eager tuple element type references in declaration order.
    ///
    /// # Returns
    ///
    /// Returns a tuple view backed by the supplied static slice.
    pub(crate) const fn new(elements: &'static [TypeRef]) -> Self {
        Self {
            elements: TypeRefListSource::Eager(elements),
        }
    }

    /// Creates a tuple view whose element list resolves on first navigation.
    ///
    /// # Parameters
    ///
    /// - `elements`: Lazy source for tuple element references.
    ///
    /// # Returns
    ///
    /// Returns a tuple view backed by the lazy element list.
    pub(crate) const fn new_lazy(elements: &'static LazyTypeRefList) -> Self {
        Self {
            elements: TypeRefListSource::Lazy(elements),
        }
    }

    /// Returns the tuple element types in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the resolved static element list, initializing it on first
    /// access.
    #[must_use]
    #[inline]
    pub fn elements(&self) -> &'static [TypeRef] {
        self.elements.get()
    }

    /// Returns the tuple arity. The unit type `()` therefore has arity zero.
    ///
    /// # Returns
    ///
    /// Returns the number of tuple elements.
    #[must_use]
    #[inline]
    pub const fn arity(&self) -> usize {
        match self.elements {
            TypeRefListSource::Eager(elements) => elements.len(),
            TypeRefListSource::Lazy(elements) => elements.len(),
        }
    }
}
