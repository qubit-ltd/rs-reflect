// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;

use super::metadata::TypeMetadata;
use super::reflect::Reflect;
use crate::runtime::capability::Capabilities;
use crate::runtime::capability::Capability;
use crate::runtime::capability::CapabilityError;
use crate::runtime::capability::CapabilitySpec;
use crate::runtime::capability::CapabilityTarget;
use crate::runtime::construct::ConstructionDescriptor;
use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::generic::GenericArguments;
use crate::runtime::descriptor::generic::TypeDefinitionDescriptor;
use crate::runtime::descriptor::impl_descriptor::ImplDescriptor;
use crate::runtime::descriptor::impl_descriptor::ImplKind;
use crate::runtime::descriptor::impl_descriptor::Impls;
use crate::runtime::descriptor::method::MethodCatalog;
use crate::runtime::descriptor::method::MethodDescriptor;
use crate::runtime::descriptor::trait_descriptor::TraitDefinitionDescriptor;
use crate::runtime::descriptor::trait_descriptor::TraitDescriptor;
use crate::runtime::error::ContextError;
use crate::runtime::error::LookupError;
use crate::runtime::error::LookupQuery;
use crate::runtime::error::LookupSubject;
use crate::runtime::error::ReflectionError;
use crate::runtime::identity::TypeIdentity;
use crate::runtime::lookup::Lookup;

/// A context-bound descriptor for a concrete Rust type.
#[derive(Clone, Copy)]
pub struct TypeDescriptor<'ctx> {
    metadata: &'static TypeMetadata,
    context: &'ctx ReflectionContext,
}

impl TypeDescriptor<'static> {
    /// Creates a descriptor using the process-wide reflection context.
    pub fn of<T: Reflect + ?Sized>() -> Self {
        Self::new(T::__reflect_registration().metadata(), ReflectionContext::global())
    }
}

impl<'ctx> TypeDescriptor<'ctx> {
    /// Creates a descriptor associated with `context`.
    pub fn of_in<T: Reflect + ?Sized>(context: &'ctx ReflectionContext) -> Self {
        Self::new(T::__reflect_registration().metadata(), context)
    }
}

impl<'ctx> TypeDescriptor<'ctx> {
    pub(crate) fn new(metadata: &'static TypeMetadata, context: &'ctx ReflectionContext) -> Self {
        Self { metadata, context }
    }

    /// Returns the Rust `TypeId` represented by this descriptor.
    pub fn type_id(self) -> TypeId {
        self.metadata.type_id()
    }

    /// Returns the final path component from Rust's diagnostic type name.
    pub fn name(self) -> &'static str {
        self.short_name()
    }

    /// Returns the type name without its module path or generic arguments.
    pub fn short_name(self) -> &'static str {
        let type_name = self.rust_type_name();
        let path_name = type_name.rsplit("::").next().unwrap_or(type_name);
        let end = path_name.find('<').unwrap_or(path_name.len());
        &path_name[..end]
    }

    /// Returns Rust's diagnostic type name. This value is not a stable lookup
    /// key.
    pub fn rust_type_name(self) -> &'static str {
        self.metadata.rust_name()
    }

    pub(crate) fn type_identity(self) -> TypeIdentity {
        TypeIdentity {
            type_id: self.type_id(),
            rust_name: self.metadata.rust_name(),
        }
    }

    pub(crate) fn shape_metadata(self) -> Option<&'static super::type_kind::TypeKindMetadata> {
        self.metadata
            .shape()
            .or_else(|| self.context.shape_for(self.type_id()).ok().flatten())
    }

    pub(crate) fn construction_shape_metadata(self) -> Option<crate::runtime::construct::ConstructionShape> {
        self.metadata.construction_shape()
    }

    /// Returns the reflection context associated with this descriptor.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }

    /// Returns whether this exact Rust type is registered in the bound context.
    ///
    /// Returns `Err` if the context cannot read its type registry; the error
    /// retains the registry failure details.
    pub fn is_registered(self) -> Result<bool, ContextError> {
        self.context.contains_type_id(self.type_id())
    }

    /// Returns the concrete type and const arguments for this compiled type
    /// application.
    ///
    /// Lifetime arguments are omitted because Rust erases them from runtime
    /// type identity. Types without source-level generic arguments return
    /// an empty collection.
    pub fn generic_arguments(self) -> GenericArguments<'ctx> {
        GenericArguments::new(self.metadata.generic_arguments(), self.context)
    }

    /// Finds this type's source-level nominal definition when one is available.
    ///
    /// Returns `Some` for types with retained nominal metadata and `None` for
    /// opaque or manually registered types without a source declaration.
    pub fn find_definition(self) -> Option<TypeDefinitionDescriptor<'ctx>> {
        self.metadata
            .type_definition()
            .map(|definition| TypeDefinitionDescriptor::new(definition, self.context))
    }

    /// Requires this type to have a source-level nominal definition.
    ///
    /// Returns `Err` with a missing-definition lookup error when the type has
    /// no retained nominal declaration. The error includes this type's
    /// diagnostic Rust name as its lookup query.
    pub fn definition(self) -> Result<TypeDefinitionDescriptor<'ctx>, ReflectionError> {
        self.find_definition().ok_or_else(|| {
            LookupError::Missing {
                subject: LookupSubject::TypeDefinition,
                query: LookupQuery::Name(self.rust_type_name().into()),
            }
            .into()
        })
    }

    /// Returns the immutable capability view registered for this type.
    pub fn capabilities(self) -> Result<Capabilities<'ctx>, CapabilityError> {
        let identity = self.type_identity();
        let target = CapabilityTarget::Type(self);
        self.context
            .capabilities_for_type(identity.type_id, identity.rust_name)
            .map(|capabilities| match capabilities {
                Some(capabilities) => Capabilities::new(capabilities, target, self.context()),
                None => Capabilities::empty(target, self.context()),
            })
            .map_err(CapabilityError::Context)
    }

    /// Finds a capability using its statically declared adapter contract.
    pub fn find_capability<C: CapabilitySpec>(self) -> Result<Option<Capability<'ctx, C>>, CapabilityError> {
        self.capabilities()?.find::<C>()
    }

    /// Returns whether this type has a capability matching the full adapter
    /// contract.
    ///
    /// Returns an error when the capability ID exists with a different adapter
    /// type.
    pub fn has_capability<C: CapabilitySpec>(self) -> Result<bool, CapabilityError> {
        self.find_capability::<C>().map(|capability| capability.is_some())
    }

    /// Requires a capability using its statically declared adapter contract.
    pub fn capability<C: CapabilitySpec>(self) -> Result<Capability<'ctx, C>, CapabilityError> {
        self.capabilities()?.capability::<C>()
    }

    pub(crate) fn construction_plans(self) -> Result<Box<[ConstructionDescriptor<'ctx>]>, ContextError> {
        self.context.construction_plans(self.type_id())
    }

    /// Looks up methods by name while preserving ambiguity information.
    pub fn lookup_method(self, name: &str) -> Result<Lookup<MethodDescriptor<'ctx>>, ContextError> {
        MethodCatalog::from_shared(self, self.context.methods_for(self.type_id())?).lookup_method(name)
    }

    /// Requires a unique method with the requested name.
    pub fn method(self, name: &str) -> Result<MethodDescriptor<'ctx>, LookupError> {
        self.lookup_method(name).map_err(LookupError::Context)?.into_unique(
            crate::runtime::error::LookupSubject::Method,
            crate::runtime::error::LookupQuery::Name(name.into()),
        )
    }

    /// Returns all registered implementation applications for this concrete
    /// type.
    pub fn impls(self) -> Result<Impls<'ctx>, ContextError> {
        self.context
            .impl_applications_for_target(self.type_id())
            .map(|values| Impls::new(values, self.context))
    }

    /// Returns the inherent implementation applications for this concrete type.
    pub fn inherent_impls(self) -> Result<Impls<'ctx>, ContextError> {
        let values = self.impls()?;
        Ok(Impls::new(
            values
                .iter()
                .filter(|implementation| implementation.kind() == ImplKind::Inherent)
                .collect(),
            self.context,
        ))
    }

    /// Returns the trait implementation applications for this concrete type.
    pub fn trait_impls(self) -> Result<Impls<'ctx>, ContextError> {
        let values = self.impls()?;
        Ok(Impls::new(
            values
                .iter()
                .filter(|implementation| implementation.kind() == ImplKind::Trait)
                .collect(),
            self.context,
        ))
    }

    /// Finds the implementation for one exact trait application.
    pub fn find_trait_impl(self, trait_: TraitDescriptor<'ctx>) -> Result<Option<ImplDescriptor<'ctx>>, ContextError> {
        self.trait_impls()?.find_trait_application(trait_)
    }

    /// Requires the implementation for one exact trait application.
    pub fn trait_impl(self, trait_: TraitDescriptor<'ctx>) -> Result<ImplDescriptor<'ctx>, LookupError> {
        self.find_trait_impl(trait_)
            .map_err(LookupError::Context)?
            .ok_or_else(|| LookupError::Missing {
                subject: LookupSubject::Impl,
                query: LookupQuery::Name(trait_.path().into()),
            })
    }

    /// Looks up trait implementations by declaration, preserving ambiguity.
    pub fn lookup_trait_impl_by_definition(
        self,
        trait_: TraitDefinitionDescriptor<'ctx>,
    ) -> Result<crate::runtime::lookup::Lookup<ImplDescriptor<'ctx>>, ContextError> {
        self.trait_impls()?.lookup_trait_definition(trait_)
    }

    /// Requires exactly one implementation application of a trait declaration.
    pub fn trait_impl_by_definition(
        self,
        trait_: TraitDefinitionDescriptor<'ctx>,
    ) -> Result<ImplDescriptor<'ctx>, LookupError> {
        self.lookup_trait_impl_by_definition(trait_)
            .map_err(LookupError::Context)?
            .into_unique(LookupSubject::Impl, LookupQuery::Name(trait_.path().into()))
    }
}
