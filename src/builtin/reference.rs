//! Reflection descriptors for shared and mutable static references.

use crate::builtin::interner;
use crate::descriptor::ReferenceKind;
use crate::descriptor::Reflect;
use crate::descriptor::TypeDescriptor;

impl<T: Reflect + ?Sized> Reflect for &'static T {
    /// Returns the interned descriptor for this shared-reference
    /// specialization.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| {
            TypeDescriptor::new_reference_lazy::<Self>(
                std::any::type_name::<Self>(),
                ReferenceKind::Shared,
                crate::__private::descriptor::lazy_type_ref::<T>(),
            )
        })
    }
}

impl<T: Reflect + ?Sized> Reflect for &'static mut T {
    /// Returns the interned descriptor for this mutable-reference
    /// specialization.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| {
            TypeDescriptor::new_reference_lazy::<Self>(
                std::any::type_name::<Self>(),
                ReferenceKind::Mutable,
                crate::__private::descriptor::lazy_type_ref::<T>(),
            )
        })
    }
}
