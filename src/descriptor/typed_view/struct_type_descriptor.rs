// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::descriptor::StructKind;

/// The typed view of a declared struct.
///
/// # Examples
///
/// ```
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// #[cfg(feature = "derive")]
/// fn main() {
/// use qubit_reflect::Reflect;
/// use qubit_reflect::TypeDescriptor;
/// #[derive(Reflect)]
/// #[reflect(crate = qubit_reflect)]
/// struct Record { value: u8 }
/// let record = TypeDescriptor::of::<Record>().as_struct().expect("struct type");
/// assert_eq!(record.kind(), qubit_reflect::descriptor::StructKind::Named);
/// }
/// #[cfg(not(feature = "derive"))]
/// fn main() {}
/// ```
#[derive(Clone, Copy, Debug)]
pub struct StructTypeDescriptor {
    /// Declared struct shape.
    kind: StructKind,
}

impl StructTypeDescriptor {
    /// Creates a struct view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `kind`: Declared struct shape.
    ///
    /// # Returns
    ///
    /// Returns the typed view for `kind`.
    pub(crate) const fn new(kind: StructKind) -> Self {
        Self { kind }
    }

    /// Returns whether the struct is named, tuple-shaped, a newtype, or
    /// unit-shaped.
    ///
    /// # Returns
    ///
    /// Returns the source-level struct shape.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> StructKind {
        self.kind
    }
}
