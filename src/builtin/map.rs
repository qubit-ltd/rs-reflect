//! Reflection descriptors for standard map collections.

use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::builtin::interner;
use crate::descriptor::MapKind;
use crate::descriptor::Reflect;
use crate::descriptor::TypeDescriptor;

/// Builds the descriptor shared by every map family specialization.
fn descriptor<T: ?Sized + 'static, K: Reflect, V: Reflect>(kind: MapKind) -> TypeDescriptor {
    TypeDescriptor::new_map_lazy::<T>(
        std::any::type_name::<T>(),
        kind,
        crate::__private::descriptor::lazy_type_ref::<K>(),
        crate::__private::descriptor::lazy_type_ref::<V>(),
    )
}

impl<K: Reflect, V: Reflect, Hasher: 'static> Reflect for HashMap<K, V, Hasher> {
    /// Returns the interned descriptor for this hash-map specialization.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| descriptor::<Self, K, V>(MapKind::HashMap))
    }
}

impl<K: Reflect, V: Reflect> Reflect for BTreeMap<K, V> {
    /// Returns the interned descriptor for this B-tree-map specialization.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| descriptor::<Self, K, V>(MapKind::BTreeMap))
    }
}
