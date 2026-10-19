//! Context-bound implementation declarations and concrete applications.

mod descriptor;
mod metadata;

pub use descriptor::{ImplDefinitionDescriptor, ImplDescriptor};
pub(crate) use descriptor::{
    AssociatedConstBindingDescriptor, AssociatedConstBindings, AssociatedConstValueMode,
    AssociatedTypeBindingDescriptor, AssociatedTypeBindings, ImplDefinitions, Impls, Methods,
};
pub use metadata::{
    AssociatedConstBindingMetadata, AssociatedItemSource, AssociatedTypeBindingMetadata,
    ImplApplicationMetadata, ImplDefinitionMetadata, ImplKind, ImplQualifiers,
};
pub(crate) use metadata::{AssociatedConstValueAdapter, ImplDefinitionId};
