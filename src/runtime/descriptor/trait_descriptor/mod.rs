//! Context-bound trait declarations and applications.

mod descriptor;
mod metadata;

pub(crate) use descriptor::{
    AssociatedConstDefinitionDescriptor, AssociatedConstDefinitions,
    AssociatedTypeDefinitionDescriptor, AssociatedTypeDefinitions, MethodDefinitionDescriptor,
    MethodDefinitionOwner, MethodDefinitions, TraitDefinitionDescriptor, TraitDefinitions,
    TraitDescriptor, TraitExpressionDescriptor, TraitExpressions, Traits,
};
pub(crate) use metadata::{
    AssociatedConstDefinitionMetadata, AssociatedTypeDefinitionMetadata, SupertraitCompleteness,
    TraitApplicationMetadata, TraitArgumentIdentity, TraitCompleteness, TraitDefinitionId,
    TraitDefinitionMetadata, TraitExpressionMetadata, Visibility,
};
