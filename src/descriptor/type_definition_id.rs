// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

//! Process-local identities for reflected type declarations.

use std::any::TypeId;

/// The process-local identity of one reflected generic type declaration.
///
/// This identity distinguishes declarations inside one process. It is not a
/// persistent or cross-build identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TypeDefinitionId(TypeId);

impl TypeDefinitionId {
    /// Creates the declaration identity represented by generated marker `T`.
    #[doc(hidden)]
    #[must_use]
    pub fn of<T: 'static>() -> Self {
        Self(TypeId::of::<T>())
    }

    /// Returns the underlying process-local marker identity.
    #[must_use]
    pub const fn marker_type_id(self) -> TypeId {
        self.0
    }
}
