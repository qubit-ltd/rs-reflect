//! Reflection descriptors for standard smart pointers.

use std::rc::Rc;
use std::sync::Arc;

use crate::builtin::interner;
use crate::descriptor::Reflect;
use crate::descriptor::SmartPointerKind;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeRef;

/// Allocates a process-lifetime resolved reference to `T`.
fn resolved<T: Reflect + ?Sized>() -> &'static TypeRef {
    Box::leak(Box::new(TypeRef::Resolved(T::type_descriptor())))
}

macro_rules! impl_smart_pointer {
    ($type:ident, $kind:expr) => {
        impl<T: Reflect + ?Sized> Reflect for $type<T> {
            /// Returns the interned descriptor for this smart-pointer specialization.
            fn type_descriptor() -> &'static TypeDescriptor {
                interner::intern::<Self>(|| {
                    TypeDescriptor::new_smart_pointer::<Self>(
                        std::any::type_name::<Self>(),
                                                $kind,
                        resolved::<T>(),
                    )
                })
            }
        }
    };
}

impl_smart_pointer!(Box, SmartPointerKind::Box);
impl_smart_pointer!(Rc, SmartPointerKind::Rc);
impl_smart_pointer!(Arc, SmartPointerKind::Arc);
