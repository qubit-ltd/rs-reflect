//! Reflection descriptors for raw pointer types.

use crate::builtin::interner;
use crate::descriptor::Mutability;
use crate::descriptor::Reflect;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeRef;

/// Allocates a process-lifetime resolved reference to `T`.
fn resolved<T: Reflect + ?Sized>() -> &'static TypeRef {
    Box::leak(Box::new(TypeRef::Resolved(T::type_descriptor())))
}

impl<T: Reflect + ?Sized> Reflect for *const T {
    /// Returns the interned descriptor for this const raw-pointer specialization.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| {
            TypeDescriptor::new_raw_pointer::<Self>(
                std::any::type_name::<Self>(),
                std::any::type_name::<Self>(),
                Mutability::Const,
                resolved::<T>(),
            )
        })
    }
}

impl<T: Reflect + ?Sized> Reflect for *mut T {
    /// Returns the interned descriptor for this mutable raw-pointer specialization.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| {
            TypeDescriptor::new_raw_pointer::<Self>(
                std::any::type_name::<Self>(),
                std::any::type_name::<Self>(),
                Mutability::Mutable,
                resolved::<T>(),
            )
        })
    }
}
