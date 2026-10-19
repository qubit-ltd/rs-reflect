use std::sync::Arc;

use crate::runtime::descriptor::generic::TypeDefinitionDescriptor;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::identity::TypeIdentity;

use super::{Capabilities, Capability, CapabilityError, CapabilitySpec};

/// Typed capability queries for a reflected concrete type.
///
/// Import this trait to query capability adapters directly from a type descriptor.
pub trait TypeDescriptorCapabilityExt<'ctx> {
    /// Returns the immutable capability view registered for this type, if any.
    fn capabilities(self) -> Result<Arc<Capabilities>, CapabilityError>;

    /// Finds a capability using its statically declared adapter contract.
    fn find_capability<C: CapabilitySpec>(
        self,
    ) -> Result<Option<Capability<'ctx, C>>, CapabilityError>;

    /// Requires a capability using its statically declared adapter contract.
    fn capability<C: CapabilitySpec>(self) -> Result<Capability<'ctx, C>, CapabilityError>;
}

impl<'ctx> TypeDescriptorCapabilityExt<'ctx> for TypeDescriptor<'ctx> {
    fn capabilities(self) -> Result<Arc<Capabilities>, CapabilityError> {
        let identity = self.type_identity();
        let target = super::CapabilityTargetId::Type(TypeIdentity {
            type_id: identity.type_id,
            rust_name: identity.rust_name,
        });
        TypeDescriptor::capabilities(self)
            .map(|capabilities| capabilities.unwrap_or_else(|| Arc::new(Capabilities::empty(target))))
            .map_err(CapabilityError::Context)
    }

    fn find_capability<C: CapabilitySpec>(
        self,
    ) -> Result<Option<Capability<'ctx, C>>, CapabilityError> {
        TypeDescriptorCapabilityExt::capabilities(self)?.find::<C>()
    }

    fn capability<C: CapabilitySpec>(self) -> Result<Capability<'ctx, C>, CapabilityError> {
        TypeDescriptorCapabilityExt::capabilities(self)?.capability::<C>()
    }
}

/// Typed capability queries for a generic source type definition.
pub trait TypeDefinitionCapabilityExt<'ctx> {
    /// Returns the immutable capability view registered for this definition.
    fn capabilities(self) -> Result<Arc<Capabilities>, CapabilityError>;

    /// Finds a capability using its statically declared adapter contract.
    fn find_capability<C: CapabilitySpec>(
        self,
    ) -> Result<Option<Capability<'ctx, C>>, CapabilityError>;

    /// Requires a capability using its statically declared adapter contract.
    fn capability<C: CapabilitySpec>(self) -> Result<Capability<'ctx, C>, CapabilityError>;
}

impl<'ctx> TypeDefinitionCapabilityExt<'ctx> for TypeDefinitionDescriptor<'ctx> {
    fn capabilities(self) -> Result<Arc<Capabilities>, CapabilityError> {
        let id = self.id();
        let target = definition_target(id);
        self.context()
            .capabilities_for_definition(id)
            .map(|capabilities| capabilities.unwrap_or_else(|| Arc::new(Capabilities::empty(target))))
            .map_err(CapabilityError::Context)
    }

    fn find_capability<C: CapabilitySpec>(
        self,
    ) -> Result<Option<Capability<'ctx, C>>, CapabilityError> {
        TypeDefinitionCapabilityExt::capabilities(self)?.find::<C>()
    }

    fn capability<C: CapabilitySpec>(self) -> Result<Capability<'ctx, C>, CapabilityError> {
        TypeDefinitionCapabilityExt::capabilities(self)?.capability::<C>()
    }
}

fn definition_target(id: crate::runtime::descriptor::generic::TypeDefinitionId) -> super::CapabilityTargetId {
    let diagnostic = format!("{id:?}");
    super::CapabilityTargetId::Definition(crate::runtime::identity::ItemIdentity {
        kind: crate::runtime::identity::ItemKind::TypeDefinition,
        stable_id: crate::runtime::identity::StableItemId(diagnostic.clone().into()),
        display_name: diagnostic.into(),
    })
}
