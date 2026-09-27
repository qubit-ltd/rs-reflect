// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Opaque member-local views for types whose structure is hidden.

use std::any::TypeId;
use std::fmt;

use super::type_ref::type_id_of;
use super::type_ref::type_name_of;

/// A concrete member type whose internal structure was explicitly hidden.
///
/// This object is a member-local view, not a second root
/// [`TypeDescriptor`](crate::descriptor::TypeDescriptor). It
/// retains only exact process-local identity and diagnostic naming until safe
/// whole-value adapters are added.
///
/// # Examples
///
/// ```
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// #[cfg(feature = "derive")]
/// fn main() {
/// use qubit_reflect::Reflect;
/// use qubit_reflect::TypeDescriptor;
/// use qubit_reflect::descriptor::TypeRef;
///
/// #[derive(Reflect)]
/// #[reflect(crate = qubit_reflect)]
/// struct Record {
///     #[reflect(opaque)]
///     value: u8,
/// }
///
/// let field = TypeDescriptor::of::<Record>()
///     .field("value")
///     .expect("the derived field exists");
/// let TypeRef::Opaque(opaque) = field.field_type() else {
///     panic!("the field is explicitly opaque");
/// };
/// assert_eq!(opaque.type_name(), std::any::type_name::<u8>());
/// }
/// #[cfg(not(feature = "derive"))]
/// fn main() {}
/// ```
pub struct OpaqueTypeDescriptor {
    type_id: fn() -> TypeId,
    type_name: fn() -> &'static str,
}

impl OpaqueTypeDescriptor {
    /// Creates an immutable opaque member descriptor for `T`.
    ///
    /// Its diagnostic name is resolved from `T` only when queried, so static
    /// generated descriptor data cannot substitute a different type name.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete or unsized member type whose identity is retained.
    ///
    /// # Returns
    ///
    /// Returns an opaque descriptor for `T`.
    #[doc(hidden)]
    pub(crate) const fn new<T: ?Sized + 'static>() -> Self {
        Self {
            type_id: type_id_of::<T>,
            type_name: type_name_of::<T>,
        }
    }

    /// Returns the exact process-local Rust type identity.
    ///
    /// # Returns
    ///
    /// Returns the `TypeId` of the hidden member type.
    #[must_use]
    #[inline]
    pub fn type_id(&self) -> TypeId {
        (self.type_id)()
    }

    /// Returns the diagnostic Rust type name.
    ///
    /// # Returns
    ///
    /// Returns the compiler-provided type name of the hidden member type.
    #[must_use]
    #[inline]
    pub fn type_name(&self) -> &'static str {
        (self.type_name)()
    }
}

impl fmt::Debug for OpaqueTypeDescriptor {
    /// Formats the diagnostic identity without attempting root-descriptor
    /// navigation.
    ///
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the opaque type identity.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting the identity, or the formatter error.
    ///
    /// # Errors
    ///
    /// Returns an error reported by `formatter`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpaqueTypeDescriptor")
            .field("type_id", &self.type_id())
            .field("type_name", &self.type_name())
            .finish()
    }
}
