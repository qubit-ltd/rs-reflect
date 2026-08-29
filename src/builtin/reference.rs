//! Reflection descriptors for shared and mutable static references.

use crate::builtin::interner;
use crate::descriptor::ReferenceKind;
use crate::descriptor::Reflect;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeRef;

/// Allocates a process-lifetime resolved reference to `T`.
fn resolved<T: Reflect + ?Sized>() -> &'static TypeRef {
    Box::leak(Box::new(TypeRef::Resolved(T::type_descriptor())))
}

impl<T: Reflect + ?Sized> Reflect for &'static T {
    /// Returns the interned descriptor for this shared-reference specialization.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| {
            TypeDescriptor::new_reference::<Self>(
                std::any::type_name::<Self>(),
                                ReferenceKind::Shared,
                resolved::<T>(),
            )
        })
    }
}

impl<T: Reflect + ?Sized> Reflect for &'static mut T {
    /// Returns the interned descriptor for this mutable-reference specialization.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| {
            TypeDescriptor::new_reference::<Self>(
                std::any::type_name::<Self>(),
                                ReferenceKind::Mutable,
                resolved::<T>(),
            )
        })
    }
}
