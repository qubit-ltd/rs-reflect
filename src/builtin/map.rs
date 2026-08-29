//! Reflection descriptors for standard map collections.

use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::builtin::interner;
use crate::descriptor::MapKind;
use crate::descriptor::Reflect;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeRef;

/// Allocates a process-lifetime resolved reference to `T`.
fn resolved<T: Reflect>() -> &'static TypeRef {
    Box::leak(Box::new(TypeRef::Resolved(T::type_descriptor())))
}

/// Builds the descriptor shared by every map family specialization.
fn descriptor<T: ?Sized + 'static, K: Reflect, V: Reflect>(kind: MapKind) -> TypeDescriptor {
    TypeDescriptor::new_map::<T>(
        std::any::type_name::<T>(),
        std::any::type_name::<T>(),
        kind,
        resolved::<K>(),
        resolved::<V>(),
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
