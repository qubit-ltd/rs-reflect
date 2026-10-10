// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Process-local identities for reflected type declarations.

use std::any::TypeId;

/// The process-local identity of one reflected generic type declaration.
///
/// This identity distinguishes declarations inside one process. It is not a
/// persistent or cross-build identifier.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::TypeDefinitionId;
/// struct Marker;
/// let id = TypeDefinitionId::of::<Marker>();
/// assert_eq!(id.marker_type_id(), std::any::TypeId::of::<Marker>());
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TypeDefinitionId(TypeId);

impl TypeDefinitionId {
    /// Creates the declaration identity represented by generated marker `T`.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Process-local marker type identifying the generic declaration.
    ///
    /// # Returns
    ///
    /// Returns the process-local identity for `T`.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub fn of<T: 'static>() -> Self {
        Self(TypeId::of::<T>())
    }

    /// Returns the underlying process-local marker identity.
    ///
    /// # Returns
    ///
    /// Returns the marker's `TypeId`, which is not stable across processes.
    #[must_use]
    #[inline]
    pub const fn marker_type_id(self) -> TypeId {
        self.0
    }
}
