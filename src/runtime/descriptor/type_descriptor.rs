use std::any::TypeId;

use crate::runtime::context::ReflectionContext;
use crate::runtime::identity::TypeIdentity;

use super::metadata::TypeMetadata;
use super::reflect::Reflect;

#[derive(Clone, Copy)]
pub(crate) struct TypeDescriptor<'ctx> {
    metadata: &'static TypeMetadata,
    context: &'ctx ReflectionContext,
}

impl TypeDescriptor<'static> {
    pub(crate) fn of<T: Reflect>() -> Self {
        Self::new(
            T::__reflect_registration().metadata(),
            ReflectionContext::global(),
        )
    }
}

impl<'ctx> TypeDescriptor<'ctx> {
    pub(crate) fn of_in<T: Reflect + ?Sized>(context: &'ctx ReflectionContext) -> Self {
        Self::new(super::metadata::TypeMetadata::of::<T>(), context)
    }
}

impl<'ctx> TypeDescriptor<'ctx> {
    pub(crate) fn new(metadata: &'static TypeMetadata, context: &'ctx ReflectionContext) -> Self {
        Self { metadata, context }
    }

    pub(crate) fn type_id(self) -> TypeId {
        self.metadata.type_id()
    }

    pub(crate) fn type_identity(self) -> TypeIdentity {
        TypeIdentity {
            type_id: self.type_id(),
            rust_name: self.metadata.rust_name(),
        }
    }

    pub(crate) fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
}
