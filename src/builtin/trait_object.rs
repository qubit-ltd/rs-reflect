//! Reflection descriptors for common dyn-compatible trait objects.

use crate::builtin::interner;
use crate::descriptor::Reflect;
use crate::descriptor::TypeDescriptor;

impl Reflect for dyn std::fmt::Debug {
    /// Returns the interned descriptor for `dyn Debug`.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| {
            TypeDescriptor::new_trait_object::<Self>(
                std::any::type_name::<Self>(),
                            )
        })
    }
}
