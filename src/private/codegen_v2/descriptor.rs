// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Exact descriptor surface consumed by codegen v2.

#[doc(hidden)]
pub use crate::__private::descriptor::AssociatedConstProbe;
#[doc(hidden)]
pub use crate::__private::descriptor::AssociatedConstProvider;
#[doc(hidden)]
pub use crate::__private::descriptor::ReflectArgumentProbe;
#[doc(hidden)]
pub use crate::__private::descriptor::ResolveAssociatedConstReader;
#[doc(hidden)]
pub use crate::__private::descriptor::ResolveReflectArgument;
#[doc(hidden)]
pub use crate::__private::descriptor::ResolveReflectTypeDescriptor;
#[doc(hidden)]
pub use crate::__private::descriptor::array;
#[doc(hidden)]
pub use crate::__private::descriptor::associated_const_reader;
#[doc(hidden)]
pub use crate::__private::descriptor::const_argument_diagnostic;
#[doc(hidden)]
pub use crate::__private::descriptor::const_argument_expression;
#[doc(hidden)]
pub use crate::__private::descriptor::const_argument_owned;
#[doc(hidden)]
pub use crate::__private::descriptor::enum_type;
#[doc(hidden)]
pub use crate::__private::descriptor::enum_type_with_repr;
#[doc(hidden)]
pub use crate::__private::descriptor::field;
#[doc(hidden)]
pub use crate::__private::descriptor::function;
#[doc(hidden)]
pub use crate::__private::descriptor::intern_type;
#[doc(hidden)]
pub use crate::__private::descriptor::lazy_field;
#[doc(hidden)]
pub use crate::__private::descriptor::lazy_type_ref;
#[doc(hidden)]
pub use crate::__private::descriptor::map;
#[doc(hidden)]
pub use crate::__private::descriptor::opaque_member;
#[doc(hidden)]
pub use crate::__private::descriptor::opaque_root;
#[doc(hidden)]
pub use crate::__private::descriptor::opaque_root_with_capabilities;
#[doc(hidden)]
pub use crate::__private::descriptor::optional;
#[doc(hidden)]
pub use crate::__private::descriptor::primitive;
#[doc(hidden)]
pub use crate::__private::descriptor::primitive_with_capabilities;
#[doc(hidden)]
pub use crate::__private::descriptor::raw_pointer;
#[doc(hidden)]
pub use crate::__private::descriptor::reference;
#[doc(hidden)]
pub use crate::__private::descriptor::sequence;
#[doc(hidden)]
pub use crate::__private::descriptor::set;
#[doc(hidden)]
pub use crate::__private::descriptor::slice;
#[doc(hidden)]
pub use crate::__private::descriptor::smart_pointer;
#[doc(hidden)]
pub use crate::__private::descriptor::struct_type;
#[doc(hidden)]
pub use crate::__private::descriptor::struct_type_with_construction;
#[doc(hidden)]
pub use crate::__private::descriptor::text;
#[doc(hidden)]
pub use crate::__private::descriptor::text_with_capabilities;
#[doc(hidden)]
pub use crate::__private::descriptor::trait_object;
#[doc(hidden)]
pub use crate::__private::descriptor::tuple;
#[doc(hidden)]
pub use crate::__private::descriptor::variant;
#[doc(hidden)]
pub use crate::__private::descriptor::with_capabilities;
#[doc(hidden)]
pub use crate::__private::descriptor::with_concrete_generic;
#[doc(hidden)]
pub use crate::__private::descriptor::with_type_definition;
#[doc(hidden)]
pub use crate::descriptor::AssociatedConstBindingDescriptor;
#[doc(hidden)]
pub use crate::descriptor::AssociatedConstDescriptor;
#[doc(hidden)]
pub use crate::descriptor::AssociatedConstImplementationSource;
#[doc(hidden)]
pub use crate::descriptor::AssociatedConstReader;
#[doc(hidden)]
pub use crate::descriptor::AssociatedTypeBindingDescriptor;
#[doc(hidden)]
pub use crate::descriptor::AssociatedTypeDescriptor;
#[doc(hidden)]
pub use crate::descriptor::ConcreteGenericDescriptor;
#[doc(hidden)]
pub use crate::descriptor::DiscriminantOrigin;
#[doc(hidden)]
pub use crate::descriptor::EnumRepr;
#[doc(hidden)]
pub use crate::descriptor::FieldDefinitionDescriptor;
#[doc(hidden)]
pub use crate::descriptor::ImplAssociatedConstDescriptor;
#[doc(hidden)]
pub use crate::descriptor::ImplAssociatedTypeDescriptor;
#[doc(hidden)]
pub use crate::descriptor::ImplDefinitionDescriptor;
#[doc(hidden)]
pub use crate::descriptor::ImplDescriptor;
#[doc(hidden)]
pub use crate::descriptor::ImplKind;
#[doc(hidden)]
pub use crate::descriptor::InvocationAdapter;
#[doc(hidden)]
pub use crate::descriptor::InvocationUnavailableReason;
#[doc(hidden)]
pub use crate::descriptor::MethodDeclarationOwner;
#[doc(hidden)]
pub use crate::descriptor::MethodDescriptor;
#[doc(hidden)]
pub use crate::descriptor::MethodImplementationSource;
#[doc(hidden)]
pub use crate::descriptor::MethodInstanceDescriptor;
#[doc(hidden)]
pub use crate::descriptor::MethodQualifiers;
#[doc(hidden)]
pub use crate::descriptor::MethodVisibility;
#[doc(hidden)]
pub use crate::descriptor::NumericDiscriminant;
#[doc(hidden)]
pub use crate::descriptor::ParameterDescriptor;
#[doc(hidden)]
pub use crate::descriptor::ParameterPassingMode;
#[doc(hidden)]
pub use crate::descriptor::ParameterPatternDescriptor;
#[doc(hidden)]
pub use crate::descriptor::ReceiverDescriptor;
#[doc(hidden)]
pub use crate::descriptor::Reflect;
#[doc(hidden)]
pub use crate::descriptor::ReturnDescriptor;
#[doc(hidden)]
pub use crate::descriptor::ReturnKind;
#[doc(hidden)]
pub use crate::descriptor::StructKind;
#[doc(hidden)]
pub use crate::descriptor::TraitCompleteness;
#[doc(hidden)]
pub use crate::descriptor::TraitDefinitionDescriptor;
#[doc(hidden)]
pub use crate::descriptor::TraitDescriptor;
#[doc(hidden)]
pub use crate::descriptor::TraitId;
#[doc(hidden)]
pub use crate::descriptor::TraitImplPayload;
#[doc(hidden)]
pub use crate::descriptor::TypeDefinitionDescriptor;
#[doc(hidden)]
pub use crate::descriptor::TypeDefinitionId;
#[doc(hidden)]
pub use crate::descriptor::TypeDescriptor;
#[doc(hidden)]
pub use crate::descriptor::TypeDescriptorResolver;
#[doc(hidden)]
pub use crate::descriptor::TypeRef;
#[doc(hidden)]
pub use crate::descriptor::VariantDefinitionDescriptor;
#[doc(hidden)]
pub use crate::descriptor::VariantKind;
#[doc(hidden)]
pub use crate::descriptor::cached_trait_object_descriptor;
#[doc(hidden)]
pub use crate::descriptor::external_supertrait;
