//! Reflection descriptors for optional values.

use crate::builtin::interner;
use crate::descriptor::Reflect;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeRef;

/// Allocates a process-lifetime resolved reference to `T`.
fn resolved<T: Reflect>() -> &'static TypeRef {
    Box::leak(Box::new(TypeRef::Resolved(T::type_descriptor())))
}

impl<T: Reflect> Reflect for Option<T> {
    /// Returns the interned descriptor for this optional-value specialization.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| {
            TypeDescriptor::new_optional::<Self>(
                std::any::type_name::<Self>(),
                                resolved::<T>(),
            )
        })
    }
}
