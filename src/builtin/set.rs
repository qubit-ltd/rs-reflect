//! Reflection descriptors for standard set collections.

use std::collections::BTreeSet;
use std::collections::HashSet;

use crate::builtin::interner;
use crate::descriptor::Reflect;
use crate::descriptor::SetKind;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeRef;

/// Allocates a process-lifetime resolved reference to `T`.
fn resolved<T: Reflect>() -> &'static TypeRef {
    Box::leak(Box::new(TypeRef::Resolved(T::type_descriptor())))
}

/// Builds the descriptor shared by every set family specialization.
fn descriptor<Set: ?Sized + 'static, Element: Reflect>(kind: SetKind) -> TypeDescriptor {
    TypeDescriptor::new_set::<Set>(std::any::type_name::<Set>(), kind, resolved::<Element>())
}

impl<T: Reflect, Hasher: 'static> Reflect for HashSet<T, Hasher> {
    /// Returns the interned descriptor for this hash-set specialization.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| descriptor::<Self, T>(SetKind::HashSet))
    }
}

impl<T: Reflect> Reflect for BTreeSet<T> {
    /// Returns the interned descriptor for this B-tree-set specialization.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| descriptor::<Self, T>(SetKind::BTreeSet))
    }
}
