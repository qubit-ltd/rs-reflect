//! Reflection descriptors for standard smart pointers.

use std::rc::Rc;
use std::sync::Arc;

use crate::builtin::interner;
use crate::descriptor::Reflect;
use crate::descriptor::SmartPointerKind;
use crate::descriptor::TypeDescriptor;

macro_rules! impl_smart_pointer {
    ($type:ident, $kind:expr) => {
        impl<T: Reflect + ?Sized> Reflect for $type<T> {
            /// Returns the interned descriptor for this smart-pointer specialization.
            fn type_descriptor() -> &'static TypeDescriptor {
                interner::intern::<Self>(|| {
                    TypeDescriptor::new_smart_pointer_lazy::<Self>(
                        std::any::type_name::<Self>(),
                        $kind,
                        crate::__private::descriptor::lazy_type_ref::<T>(),
                    )
                })
            }
        }
    };
}

impl_smart_pointer!(Box, SmartPointerKind::Box);
impl_smart_pointer!(Rc, SmartPointerKind::Rc);
impl_smart_pointer!(Arc, SmartPointerKind::Arc);
