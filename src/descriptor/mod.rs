//! Immutable type and member descriptor APIs.

mod field_descriptor;
mod generic_descriptor;
mod impl_descriptor;
mod method_descriptor;
mod trait_descriptor;
mod type_descriptor;
mod type_kind;
mod type_ref;
mod typed_view;
mod variant_descriptor;

pub use field_descriptor::FieldDescriptor;
pub use field_descriptor::TypeDescriptorResolver;
pub use generic_descriptor::ConcreteGenericDescriptor;
pub use impl_descriptor::{
    AssociatedConstBindingDescriptor, AssociatedConstImplementationSource, AssociatedConstReader,
    AssociatedTypeBindingDescriptor, ImplDefinitionDescriptor, ImplDescriptor,
    ImplDescriptorBuildError, ImplKind, MethodLookup, MethodQualifier,
};
pub use method_descriptor::{
    InvocationAdapter, InvocationUnavailableReason, MethodDescriptor, MethodDescriptorBuilder,
    MethodDeclarationOwner, MethodImplementationSource, MethodInstanceBuildError,
    MethodInstanceDescriptor, MethodQualifiers, MethodVisibility, ParameterDescriptor,
    ParameterPassingMode, ParameterPatternDescriptor, ReceiverDescriptor, ReturnDescriptor,
    ReturnKind,
};
pub use trait_descriptor::{
    AppliedTraitId, AssociatedConstDescriptor, AssociatedTypeDescriptor, SupertraitClosure,
    TraitCompleteness, TraitDefinitionDescriptor, TraitDescriptor, TraitDescriptorBuildError,
    TraitDescriptorBuilder, TraitDescriptorRef, TraitId, TraitImplPayload, external_supertrait,
};
pub use type_descriptor::Reflect;
pub use type_descriptor::TypeDescriptor;
pub use type_kind::FunctionPointerKind;
pub use type_kind::Mutability;
pub use type_kind::PrimitiveKind;
pub use type_kind::ReferenceKind;
pub use type_kind::SmartPointerKind;
pub use type_kind::StructKind;
pub use type_kind::TextKind;
pub use type_kind::TypeKind;
pub use type_ref::OpaqueTypeDescriptor;
pub use type_ref::TypeRef;
pub use typed_view::ArrayTypeDescriptor;
pub use typed_view::EnumTypeDescriptor;
pub use typed_view::FunctionTypeDescriptor;
pub use typed_view::MapKind;
pub use typed_view::MapTypeDescriptor;
pub use typed_view::OpaqueTypeView;
pub use typed_view::OptionalTypeDescriptor;
pub use typed_view::PrimitiveTypeDescriptor;
pub use typed_view::RawPointerTypeDescriptor;
pub use typed_view::ReferenceTypeDescriptor;
pub use typed_view::SequenceKind;
pub use typed_view::SequenceTypeDescriptor;
pub use typed_view::SetKind;
pub use typed_view::SetTypeDescriptor;
pub use typed_view::SliceTypeDescriptor;
pub use typed_view::SmartPointerTypeDescriptor;
pub use typed_view::StructTypeDescriptor;
pub use typed_view::TextTypeDescriptor;
pub use typed_view::TraitObjectTypeDescriptor;
pub use typed_view::TupleTypeDescriptor;
pub use variant_descriptor::{DiscriminantOrigin, NumericDiscriminant, VariantDescriptor};
pub use variant_descriptor::VariantKind;
