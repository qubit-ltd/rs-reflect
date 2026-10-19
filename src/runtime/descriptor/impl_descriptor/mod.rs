//! Context-bound implementation declarations and concrete applications.

mod descriptor;
mod metadata;

pub(crate) use descriptor::{
    AssociatedConstBindingDescriptor, AssociatedConstBindings, AssociatedConstValueMode,
    AssociatedTypeBindingDescriptor, AssociatedTypeBindings, ImplDefinitionDescriptor,
    ImplDefinitions, ImplDescriptor, Impls, Methods,
};
pub(crate) use metadata::{
    AssociatedConstBindingMetadata, AssociatedConstValueAdapter, AssociatedItemSource,
    AssociatedTypeBindingMetadata, ImplApplicationMetadata, ImplDefinitionId,
    ImplDefinitionMetadata, ImplKind, ImplQualifiers,
};
