//! Context-bound implementation declarations and concrete applications.

mod descriptor;
mod metadata;

pub(crate) use descriptor::{
    AssociatedConstBindingDescriptor, AssociatedConstBindings, AssociatedConstValueMode,
    AssociatedTypeBindingDescriptor, AssociatedTypeBindings, ImplDefinitionDescriptor,
    ImplDefinitions, ImplDescriptor, Impls, Methods,
};
pub use metadata::{
    AssociatedConstBindingMetadata, AssociatedItemSource, AssociatedTypeBindingMetadata,
    ImplApplicationMetadata, ImplDefinitionMetadata, ImplKind, ImplQualifiers,
};
pub(crate) use metadata::{AssociatedConstValueAdapter, ImplDefinitionId};
