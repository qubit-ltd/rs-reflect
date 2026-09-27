// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Reflection descriptors for standard set collections.

use std::any::type_name;
use std::collections::BTreeSet;
use std::collections::HashSet;

use crate::builtin::interner;
use crate::descriptor::Reflect;
use crate::descriptor::SetKind;
use crate::descriptor::TypeDescriptor;

/// Builds the descriptor shared by every set family specialization.
///
/// # Type Parameters
///
/// - `Set`: Concrete set type represented by the descriptor.
/// - `Element`: Reflected element type stored in the set.
///
/// # Parameters
///
/// - `kind`: Standard set family represented by `Set`.
///
/// # Returns
///
/// A descriptor with a deferred relationship to the element type.
#[must_use]
fn descriptor<Set: ?Sized + 'static, Element: Reflect>(kind: SetKind) -> TypeDescriptor {
    TypeDescriptor::new_set_lazy::<Set>(
        type_name::<Set>(),
        kind,
        crate::__private::descriptor::lazy_type_ref::<Element>(),
    )
}

impl<T: Reflect, Hasher: 'static> Reflect for HashSet<T, Hasher> {
    /// Returns the interned descriptor for this hash-set specialization.
    ///
    /// # Returns
    ///
    /// The descriptor recording the hash-set family and element relationship.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| descriptor::<Self, T>(SetKind::HashSet))
    }
}

impl<T: Reflect> Reflect for BTreeSet<T> {
    /// Returns the interned descriptor for this B-tree-set specialization.
    ///
    /// # Returns
    ///
    /// The descriptor recording the B-tree-set family and element relationship.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| descriptor::<Self, T>(SetKind::BTreeSet))
    }
}
