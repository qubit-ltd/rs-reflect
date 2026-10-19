pub(crate) mod collections;
pub(crate) mod field;
pub(crate) mod generic;
pub(crate) mod impl_descriptor;
pub(crate) mod metadata;
pub(crate) mod method;
pub(crate) mod reflect;
pub(crate) mod trait_descriptor;
pub(crate) mod type_descriptor;
pub(crate) mod type_kind;
pub(crate) mod variant;

pub(crate) use field::{FieldAccessError, FieldDescriptor, FieldSetError};
pub use generic::{
    GenericArgumentMetadata, GenericDefinitionMetadata, GenericParameterMetadata,
    GenericParameterMetadataKind, StructKind, TypeDefinitionKind, TypeDefinitionMetadata,
};
pub(crate) use method::{MethodCatalog, MethodCatalogRegistration, MethodDescriptor};
pub(crate) use reflect::{Reflect, TypeRegistration};
pub(crate) use type_descriptor::TypeDescriptor;
pub(crate) use variant::VariantDescriptor;
