// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Reflection descriptors for standard map collections.

use std::any::type_name;
use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::builtin::interner;
use crate::descriptor::MapKind;
use crate::descriptor::Reflect;
use crate::descriptor::TypeDescriptor;

/// Builds the descriptor shared by every map family specialization.
///
/// # Type Parameters
///
/// - `T`: Concrete map type represented by the descriptor.
/// - `K`: Reflected key type.
/// - `V`: Reflected value type.
///
/// # Parameters
///
/// - `kind`: Standard map family represented by `T`.
///
/// # Returns
///
/// A descriptor with deferred relationships to the key and value types.
#[must_use]
fn descriptor<T: ?Sized + 'static, K: Reflect, V: Reflect>(kind: MapKind) -> TypeDescriptor {
    TypeDescriptor::new_map_lazy::<T>(
        type_name::<T>(),
        kind,
        crate::__private::descriptor::lazy_type_ref::<K>(),
        crate::__private::descriptor::lazy_type_ref::<V>(),
    )
}

impl<K: Reflect, V: Reflect, Hasher: 'static> Reflect for HashMap<K, V, Hasher> {
    /// Returns the interned descriptor for this hash-map specialization.
    ///
    /// # Returns
    ///
    /// The descriptor recording the hash-map family and its key and value
    /// relationships.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| descriptor::<Self, K, V>(MapKind::HashMap))
    }
}

impl<K: Reflect, V: Reflect> Reflect for BTreeMap<K, V> {
    /// Returns the interned descriptor for this B-tree-map specialization.
    ///
    /// # Returns
    ///
    /// The descriptor recording the B-tree-map family and its key and value
    /// relationships.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| descriptor::<Self, K, V>(MapKind::BTreeMap))
    }
}
