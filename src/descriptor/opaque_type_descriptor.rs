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
pub struct OpaqueTypeDescriptor {
    type_id: fn() -> TypeId,
    type_name: fn() -> &'static str,
}

impl OpaqueTypeDescriptor {
    /// Creates an immutable opaque member descriptor for `T`.
    ///
    /// Its diagnostic name is resolved from `T` only when queried, so static
    /// generated descriptor data cannot substitute a different type name.
    #[doc(hidden)]
    pub(crate) const fn new<T: ?Sized + 'static>() -> Self {
        Self {
            type_id: type_id_of::<T>,
            type_name: type_name_of::<T>,
        }
    }

    /// Returns the exact process-local Rust type identity.
    #[must_use]
    #[inline]
    pub fn type_id(&self) -> TypeId {
        (self.type_id)()
    }

    /// Returns the diagnostic Rust type name.
    #[must_use]
    #[inline]
    pub fn type_name(&self) -> &'static str {
        (self.type_name)()
    }
}

impl fmt::Debug for OpaqueTypeDescriptor {
    /// Formats the diagnostic identity without attempting root-descriptor
    /// navigation.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpaqueTypeDescriptor")
            .field("type_id", &self.type_id())
            .field("type_name", &self.type_name())
            .finish()
    }
}
