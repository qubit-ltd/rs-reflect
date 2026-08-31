// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Immutable type and member descriptor APIs.

mod field_descriptor;
mod generic_descriptor;
mod impl_descriptor;
mod internal;
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
pub use impl_descriptor::AssociatedConstBindingDescriptor;
pub use impl_descriptor::AssociatedConstImplementationSource;
pub use impl_descriptor::AssociatedConstReadUnavailableReason;
pub use impl_descriptor::AssociatedConstReader;
pub use impl_descriptor::AssociatedTypeBindingDescriptor;
pub use impl_descriptor::ImplAssociatedConstDescriptor;
pub use impl_descriptor::ImplAssociatedTypeDescriptor;
pub use impl_descriptor::ImplDefinitionDescriptor;
pub use impl_descriptor::ImplDescriptor;
pub use impl_descriptor::ImplDescriptorBuildError;
pub use impl_descriptor::ImplKind;
pub use impl_descriptor::MethodLookup;
pub use impl_descriptor::MethodQualifier;
pub use method_descriptor::CatchingAvailability;
pub use method_descriptor::InvocationAdapter;
pub use method_descriptor::InvocationUnavailableReason;
pub use method_descriptor::MethodDeclarationOwner;
pub use method_descriptor::MethodDescriptor;
pub use method_descriptor::MethodDescriptorBuilder;
pub use method_descriptor::MethodImplementationSource;
pub use method_descriptor::MethodInstanceBuildError;
pub use method_descriptor::MethodInstanceDescriptor;
pub use method_descriptor::MethodQualifiers;
pub use method_descriptor::MethodVisibility;
pub use method_descriptor::ParameterDescriptor;
pub use method_descriptor::ParameterPassingMode;
pub use method_descriptor::ParameterPatternDescriptor;
pub use method_descriptor::ReceiverDescriptor;
pub use method_descriptor::ReturnDescriptor;
pub use method_descriptor::ReturnKind;
pub use trait_descriptor::AppliedTraitId;
pub use trait_descriptor::AssociatedConstDescriptor;
pub use trait_descriptor::AssociatedTypeDescriptor;
pub use trait_descriptor::SupertraitClosure;
pub use trait_descriptor::TraitCompleteness;
pub use trait_descriptor::TraitDefinitionDescriptor;
pub use trait_descriptor::TraitDescriptor;
pub use trait_descriptor::TraitDescriptorBuildError;
pub use trait_descriptor::TraitDescriptorBuilder;
pub use trait_descriptor::TraitDescriptorRef;
pub use trait_descriptor::TraitId;
pub use trait_descriptor::TraitImplPayload;
pub use trait_descriptor::cached_trait_object_descriptor;
pub use trait_descriptor::external_supertrait;
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
pub use typed_view::EnumRepr;
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
pub use variant_descriptor::DiscriminantOrigin;
pub use variant_descriptor::NumericDiscriminant;
pub use variant_descriptor::VariantDescriptor;
pub use variant_descriptor::VariantKind;
