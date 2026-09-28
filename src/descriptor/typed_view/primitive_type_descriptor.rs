// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::descriptor::PrimitiveKind;

/// The typed view of a primitive descriptor.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let primitive = TypeDescriptor::of::<u32>().as_primitive().expect("primitive type");
/// assert_eq!(primitive.kind(), qubit_reflect::descriptor::PrimitiveKind::U32);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct PrimitiveTypeDescriptor {
    /// Primitive category represented by the root.
    kind: PrimitiveKind,
}

impl PrimitiveTypeDescriptor {
    /// Creates a primitive view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `kind`: Primitive category represented by the root.
    ///
    /// # Returns
    ///
    /// Returns the typed view for `kind`.
    pub(crate) const fn new(kind: PrimitiveKind) -> Self {
        Self { kind }
    }

    /// Returns the exact primitive represented by this view.
    ///
    /// # Returns
    ///
    /// Returns the represented primitive category.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> PrimitiveKind {
        self.kind
    }
}
