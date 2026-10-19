use std::any::TypeId;

use crate::runtime::capability::capabilities::Capabilities;
use crate::runtime::construct::ConstructionDescriptor;
use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::method::{MethodCatalog, MethodDescriptor};
use crate::runtime::error::{ContextError, LookupError};
use crate::runtime::identity::TypeIdentity;
use crate::runtime::lookup::Lookup;
use std::sync::Arc;

use super::metadata::TypeMetadata;
use super::reflect::Reflect;

#[derive(Clone, Copy)]
pub struct TypeDescriptor<'ctx> {
    metadata: &'static TypeMetadata,
    context: &'ctx ReflectionContext,
}

impl TypeDescriptor<'static> {
    pub fn of<T: Reflect>() -> Self {
        Self::new(
            T::__reflect_registration().metadata(),
            ReflectionContext::global(),
        )
    }
}

impl<'ctx> TypeDescriptor<'ctx> {
    pub fn of_in<T: Reflect + ?Sized>(context: &'ctx ReflectionContext) -> Self {
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

    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }

    pub(crate) fn capabilities(self) -> Result<Option<Arc<Capabilities>>, ContextError> {
        let identity = self.type_identity();
        self.context
            .capabilities_for_type(identity.type_id, identity.rust_name)
    }

    pub(crate) fn construction_plans(
        self,
    ) -> Result<Box<[ConstructionDescriptor<'ctx>]>, ContextError> {
        self.context.construction_plans(self.type_id())
    }

    pub fn lookup_method(
        self,
        name: &str,
    ) -> Result<Lookup<MethodDescriptor<'ctx>>, ContextError> {
        Ok(
            MethodCatalog::new(self, self.context.methods_for(self.type_id())?)
                .lookup_method(name)?,
        )
    }

    pub fn method(self, name: &str) -> Result<MethodDescriptor<'ctx>, LookupError> {
        self.lookup_method(name)
            .map_err(LookupError::Context)?
            .into_unique(
                crate::runtime::error::LookupSubject::Method,
                crate::runtime::error::LookupQuery::Name(name.into()),
            )
    }
}
