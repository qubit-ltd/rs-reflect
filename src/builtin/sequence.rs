//! Reflection descriptors for ordered sequence collections.

use crate::builtin::interner;
use crate::descriptor::Reflect;
use crate::descriptor::SequenceKind;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeRef;

/// Allocates a process-lifetime resolved reference to `T`.
fn resolved<T: Reflect>() -> &'static TypeRef {
    Box::leak(Box::new(TypeRef::Resolved(T::type_descriptor())))
}

impl<T: Reflect> Reflect for Vec<T> {
    /// Returns the interned descriptor for this vector specialization.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| {
            TypeDescriptor::new_sequence::<Self>(
                std::any::type_name::<Self>(),
                std::any::type_name::<Self>(),
                SequenceKind::Vec,
                resolved::<T>(),
            )
        })
    }
}
