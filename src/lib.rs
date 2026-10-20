// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Macro-based structural and executable reflection for Rust.
//!
//! The crate exposes immutable descriptors plus checked field access, method
//! invocation, construction, capability, and registry APIs. Reflection is
//! opt-in through [`macro@Reflect`] and [`ReflectImpl`] attributes when the
//! `derive` feature is enabled.
//!
//! # Quick start
//!
//! ```
//! use qubit_reflect::TypeDescriptor;
//!
//! let descriptor = TypeDescriptor::of::<Vec<String>>();
//! let sequence = descriptor
//!     .as_sequence()
//!     .expect("Vec exposes a sequence view");
//! assert_eq!(sequence.descriptor().type_id(), descriptor.type_id());
//!
//! let value = vec![String::from("Ada")];
//! assert_eq!(value[0], "Ada");
//! ```
//!
//! To reflect an application record and update a named field, enable the
//! default `derive` feature:
//!
//! ```rust
//! # #![allow(proc_macro_derive_resolution_fallback)]
//! # #[cfg(feature = "derive")]
//! # fn main() {
//! use qubit_reflect::{Reflect, TypeDescriptor};
//!
//! mod example {
//!     use qubit_reflect::Reflect;
//!     #[Reflect]
//!     pub struct Customer { pub email: String }
//! }
//!
//! let mut customer = example::Customer { email: String::from("old@example.com") };
//! let email = TypeDescriptor::of::<example::Customer>()
//!     .field("email")
//!     .expect("derived field");
//! email
//!     .set(&mut customer, String::from("new@example.com"))
//!     .expect("exactly typed replacement");
//! assert_eq!(customer.email, "new@example.com");
//! # }
//! # #[cfg(not(feature = "derive"))]
//! # fn main() {}
//! ```
//!
//! # Guides
//!
//! - [English user guide](https://github.com/qubit-ltd/rs-reflect/blob/main/doc/user_guide.md)
//! - [中文用户指南](https://github.com/qubit-ltd/rs-reflect/blob/main/doc/user_guide.zh_CN.md)
#![deny(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(not(test))]
pub use crate as qubit_reflect;
#[cfg(test)]
extern crate self as qubit_reflect;

/// Context construction, registration, and discovery.
pub mod context {
    pub use crate::__private::codegen_v4::registration::MethodBinding;
    pub use crate::runtime::context::TraitDefinitions;
    pub use crate::runtime::context::Traits;
    pub use crate::runtime::context::TypeDefinitions;
    pub use crate::runtime::context::Types;
    pub use crate::runtime::context::handle::ReflectionContext;
    pub use crate::runtime::context::handle::ReflectionContextId;
    pub use crate::runtime::context::handle::RegisteredTraitDescriptor;
    pub use crate::runtime::context::handle::TraitObjectDescriptor;
    pub use crate::runtime::context::handle::builder::ReflectionContextBuilder;
    pub use crate::runtime::context::handle::registration::Registration;
    pub use crate::runtime::context::handle::registration::TraitObjectRegistration;
    pub use crate::runtime::context::handle::registration::TypeDefinitionRegistration;
}
/// Context-bound type, member, trait, and implementation descriptors.
pub mod descriptor {
    pub use crate::runtime::descriptor::collections::Fields;
    pub use crate::runtime::descriptor::collections::Variants;
    pub use crate::runtime::descriptor::field::FieldAccess;
    pub use crate::runtime::descriptor::field::FieldAccessError;
    pub use crate::runtime::descriptor::field::FieldAccessUnavailableReason;
    pub use crate::runtime::descriptor::field::FieldDescriptor;
    pub use crate::runtime::descriptor::field::FieldSetError;
    pub use crate::runtime::descriptor::field::FieldSetFailure;
    pub use crate::runtime::descriptor::field::Visibility;
    pub use crate::runtime::descriptor::generic::ConstArgumentDescriptor;
    pub use crate::runtime::descriptor::generic::GenericArgumentDescriptor;
    pub use crate::runtime::descriptor::generic::GenericArgumentMetadata;
    pub use crate::runtime::descriptor::generic::GenericArguments;
    pub use crate::runtime::descriptor::generic::GenericDefinitionDescriptor;
    pub use crate::runtime::descriptor::generic::GenericDefinitionMetadata;
    pub use crate::runtime::descriptor::generic::GenericParameterDescriptor;
    pub use crate::runtime::descriptor::generic::GenericParameterKind;
    pub use crate::runtime::descriptor::generic::GenericParameterMetadata;
    pub use crate::runtime::descriptor::generic::GenericParameterMetadataKind;
    pub use crate::runtime::descriptor::generic::StructKind;
    pub use crate::runtime::descriptor::generic::TypeArgumentDescriptor;
    pub use crate::runtime::descriptor::generic::TypeDefinitionDescriptor;
    pub use crate::runtime::descriptor::generic::TypeDefinitionId;
    pub use crate::runtime::descriptor::generic::TypeDefinitionKind;
    pub use crate::runtime::descriptor::generic::TypeDefinitionMetadata;
    pub use crate::runtime::descriptor::impl_descriptor::AssociatedConstBindingDescriptor;
    pub use crate::runtime::descriptor::impl_descriptor::AssociatedConstBindings;
    pub use crate::runtime::descriptor::impl_descriptor::AssociatedItemSource;
    pub use crate::runtime::descriptor::impl_descriptor::AssociatedTypeBindingDescriptor;
    pub use crate::runtime::descriptor::impl_descriptor::AssociatedTypeBindings;
    pub use crate::runtime::descriptor::impl_descriptor::ImplAssociatedConstDefinitions;
    pub use crate::runtime::descriptor::impl_descriptor::ImplAssociatedTypeDefinitions;
    pub use crate::runtime::descriptor::impl_descriptor::ImplDefinitionDescriptor;
    pub use crate::runtime::descriptor::impl_descriptor::ImplDefinitionId;
    pub use crate::runtime::descriptor::impl_descriptor::ImplDefinitions;
    pub use crate::runtime::descriptor::impl_descriptor::ImplDescriptor;
    pub use crate::runtime::descriptor::impl_descriptor::ImplKind;
    pub use crate::runtime::descriptor::impl_descriptor::ImplQualifiers;
    pub use crate::runtime::descriptor::impl_descriptor::Impls;
    pub use crate::runtime::descriptor::impl_descriptor::Methods;
    pub use crate::runtime::descriptor::method::EffectiveMethod;
    pub use crate::runtime::descriptor::method::FunctionAbi;
    pub use crate::runtime::descriptor::method::InvocationAdapters;
    pub use crate::runtime::descriptor::method::InvocationAdaptersBuilder;
    pub use crate::runtime::descriptor::method::MethodArguments;
    pub use crate::runtime::descriptor::method::MethodAvailability;
    pub use crate::runtime::descriptor::method::MethodDescriptor;
    pub use crate::runtime::descriptor::method::MethodImplementationSource;
    pub use crate::runtime::descriptor::method::MethodNamespace;
    pub use crate::runtime::descriptor::method::MethodNamespaceMetadata;
    pub use crate::runtime::descriptor::method::MethodQualifiedName;
    pub use crate::runtime::descriptor::method::MethodQualifiers;
    pub use crate::runtime::descriptor::method::MethodSpecializationArgument;
    pub use crate::runtime::descriptor::method::MethodSpecializationDescriptor;
    pub use crate::runtime::descriptor::method::MethodSpecializationSpec;
    pub use crate::runtime::descriptor::method::MethodSpecializations;
    pub use crate::runtime::descriptor::method::ParameterDescriptor;
    pub use crate::runtime::descriptor::method::ParameterPattern;
    pub use crate::runtime::descriptor::method::Parameters;
    pub use crate::runtime::descriptor::method::PassingMode;
    pub use crate::runtime::descriptor::method::ReceiverDescriptor;
    pub use crate::runtime::descriptor::method::ReceiverKind;
    pub use crate::runtime::descriptor::method::ReturnDescriptor;
    pub use crate::runtime::descriptor::method::ReturnKind;
    pub use crate::runtime::descriptor::reflect::Reflect;
    pub use crate::runtime::descriptor::reflect::TypeRegistration;
    pub use crate::runtime::descriptor::trait_descriptor::AssociatedConstDefinitionDescriptor;
    pub use crate::runtime::descriptor::trait_descriptor::AssociatedConstDefinitions;
    pub use crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitionDescriptor;
    pub use crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitions;
    pub use crate::runtime::descriptor::trait_descriptor::MethodDefinitionDescriptor;
    pub use crate::runtime::descriptor::trait_descriptor::MethodDefinitionOwner;
    pub use crate::runtime::descriptor::trait_descriptor::MethodDefinitions;
    pub use crate::runtime::descriptor::trait_descriptor::ReceiverDefinitionDescriptor;
    pub use crate::runtime::descriptor::trait_descriptor::SupertraitCompleteness;
    pub use crate::runtime::descriptor::trait_descriptor::TraitCompleteness;
    pub use crate::runtime::descriptor::trait_descriptor::TraitDefinitionDescriptor;
    pub use crate::runtime::descriptor::trait_descriptor::TraitDefinitionId;
    pub use crate::runtime::descriptor::trait_descriptor::TraitDescriptor;
    pub use crate::runtime::descriptor::trait_descriptor::TraitExpressionDescriptor;
    pub use crate::runtime::descriptor::trait_descriptor::TraitExpressions;
    pub use crate::runtime::descriptor::trait_descriptor::Traits;
    pub use crate::runtime::descriptor::trait_descriptor::Visibility as TraitVisibility;
    pub use crate::runtime::descriptor::type_descriptor::TypeDescriptor;
    pub use crate::runtime::descriptor::type_kind::ArrayType;
    pub use crate::runtime::descriptor::type_kind::EnumType;
    pub use crate::runtime::descriptor::type_kind::FunctionType;
    pub use crate::runtime::descriptor::type_kind::MapType;
    pub use crate::runtime::descriptor::type_kind::OpaqueType;
    pub use crate::runtime::descriptor::type_kind::OptionalType;
    pub use crate::runtime::descriptor::type_kind::PrimitiveType;
    pub use crate::runtime::descriptor::type_kind::RawPointerType;
    pub use crate::runtime::descriptor::type_kind::ReferenceType;
    pub use crate::runtime::descriptor::type_kind::SequenceType;
    pub use crate::runtime::descriptor::type_kind::SetType;
    pub use crate::runtime::descriptor::type_kind::SliceType;
    pub use crate::runtime::descriptor::type_kind::SmartPointerType;
    pub use crate::runtime::descriptor::type_kind::StructType;
    pub use crate::runtime::descriptor::type_kind::TextType;
    pub use crate::runtime::descriptor::type_kind::TraitObjectType;
    pub use crate::runtime::descriptor::type_kind::TupleType;
    pub use crate::runtime::descriptor::type_kind::TypeKind;
    pub use crate::runtime::descriptor::variant::DiscriminantOrigin;
    pub use crate::runtime::descriptor::variant::NumericDiscriminant;
    pub use crate::runtime::descriptor::variant::VariantDescriptor;
    pub use crate::runtime::descriptor::variant::VariantDiscriminant;
    pub use crate::runtime::descriptor::variant::VariantIdentity;
    pub use crate::runtime::descriptor::variant::VariantShape;
    pub use crate::runtime::expression::AssociatedTypeConstraintDescriptor;
    pub use crate::runtime::expression::AssociatedTypeConstraintKind;
    pub use crate::runtime::expression::AssociatedTypeConstraints;
    pub use crate::runtime::expression::AssociatedTypeExpression;
    pub use crate::runtime::expression::ConstExpression;
    pub use crate::runtime::expression::FunctionPointerExpression;
    pub use crate::runtime::expression::GenericArgumentError;
    pub use crate::runtime::expression::GenericExpressionArgumentKind;
    pub use crate::runtime::expression::GenericExpressionArgumentView;
    pub use crate::runtime::expression::GenericExpressionArguments;
    pub use crate::runtime::expression::LifetimeExpression;
    pub use crate::runtime::expression::NamedTypeExpression;
    pub use crate::runtime::expression::OpaqueTypeExpression;
    pub use crate::runtime::expression::PredicateDescriptor;
    pub use crate::runtime::expression::PredicateKind;
    pub use crate::runtime::expression::Predicates;
    pub use crate::runtime::expression::TraitBoundModifier;
    pub use crate::runtime::expression::TraitExpression;
    pub use crate::runtime::expression::TraitObjectExpression;
    pub use crate::runtime::expression::TypeExpression;
    pub use crate::runtime::expression::TypeExpressionError;
    pub use crate::runtime::expression::TypeExpressionKind;
    pub use crate::runtime::invoke::BorrowOrigin;
    pub use crate::runtime::invoke::CaughtInvocationFailure;
    pub use crate::runtime::invoke::InvocationAvailability;
    pub use crate::runtime::invoke::InvocationError;
    pub use crate::runtime::invoke::InvocationErrorKind;
    pub use crate::runtime::invoke::InvocationFailure;
    pub use crate::runtime::invoke::InvocationPanic;
    pub use crate::runtime::invoke::InvocationRecovery;
    pub use crate::runtime::invoke::InvocationUnavailableReason;
    pub use crate::runtime::invoke::OutputMismatch;
    pub use crate::runtime::invoke::ReflectedFuture;
}
/// Dynamic values and their storage modes.
pub mod value {
    pub use crate::runtime::value::DowncastFailure;
    pub use crate::runtime::value::DynamicMut;
    pub use crate::runtime::value::DynamicOwned;
    pub use crate::runtime::value::DynamicRef;
    pub use crate::runtime::value::DynamicValue;
    pub use crate::runtime::value::DynamicValueKind;
    pub use crate::runtime::value::Local;
    pub use crate::runtime::value::ThreadSafe;
    pub use crate::runtime::value::TypeMismatch;
    pub use crate::runtime::value::ValueFor;
    pub use crate::runtime::value::ValueForMut;
    pub use crate::runtime::value::ValueForRef;
    pub use crate::runtime::value::ValueMode;
}
/// Checked construction and update APIs.
pub mod construct {
    pub use crate::construct_vnext::ConstructField;
    pub use crate::construct_vnext::ConstructionAvailability;
    pub use crate::construct_vnext::ConstructionDescriptor;
    pub use crate::construct_vnext::ConstructionError;
    pub use crate::construct_vnext::ConstructionFailure;
    pub use crate::construct_vnext::ConstructionFieldPolicy;
    pub use crate::construct_vnext::ConstructionFields;
    pub use crate::construct_vnext::ConstructionInput;
    pub use crate::construct_vnext::ConstructionInputMode;
    pub use crate::construct_vnext::ConstructionMode;
    pub use crate::construct_vnext::ConstructionPlan;
    pub use crate::construct_vnext::ConstructionProviders;
    pub use crate::construct_vnext::ConstructionRecovery;
    pub use crate::construct_vnext::ConstructionShape;
    pub use crate::construct_vnext::ConstructionTarget;
    pub use crate::construct_vnext::ConstructionUnavailableReason;
    pub use crate::construct_vnext::NamedFields;
    pub use crate::construct_vnext::PositionalFields;
    pub use crate::construct_vnext::RecoveredConstructionInput;
    pub use crate::construct_vnext::RuntimeConstructionInput;
    pub use crate::construct_vnext::UnitFields;
    pub use crate::construct_vnext::UpdateError;
    pub use crate::construct_vnext::UpdateFailure;
    pub use crate::construct_vnext::UpdateFieldPolicy;
    pub use crate::construct_vnext::UpdateInput;
    pub use crate::construct_vnext::UpdateRecovery;
    pub use crate::construct_vnext::UpdateUnavailableReason;
}
/// Method invocation and recovery.
pub mod invoke {
    pub use crate::runtime::invoke::CaughtInvocationFailure;
    pub use crate::runtime::invoke::FunctionInvocationAdapter;
    pub use crate::runtime::invoke::Invocation;
    pub use crate::runtime::invoke::InvocationAdapter;
    pub use crate::runtime::invoke::InvocationAvailability;
    pub use crate::runtime::invoke::InvocationError;
    pub use crate::runtime::invoke::InvocationErrorKind;
    pub use crate::runtime::invoke::InvocationFailure;
    pub use crate::runtime::invoke::InvocationOutput;
    pub use crate::runtime::invoke::InvocationPanic;
    pub use crate::runtime::invoke::InvocationReceiver;
    pub use crate::runtime::invoke::InvocationRecovery;
    pub use crate::runtime::invoke::InvocationUnavailableReason;
    pub use crate::runtime::invoke::InvocationValue;
    pub use crate::runtime::invoke::OutputMismatch;
    pub use crate::runtime::invoke::PinnedFunctionInvocationAdapter;
    pub use crate::runtime::invoke::PinnedInvocation;
    pub use crate::runtime::invoke::PinnedInvocationFailure;
    pub use crate::runtime::invoke::PinnedReceiver;
    pub use crate::runtime::invoke::ReflectedFuture;
    pub use crate::runtime::invoke::ValidatedInvocation;
    pub use crate::runtime::invoke::ValidatedPinnedInvocation;
}
/// Typed capability registration and access.
pub mod capability {
    pub use crate::runtime::capability::Capabilities;
    pub use crate::runtime::capability::Capability;
    pub use crate::runtime::capability::CapabilityContractError;
    pub use crate::runtime::capability::CapabilityDescriptor;
    pub use crate::runtime::capability::CapabilityError;
    pub use crate::runtime::capability::CapabilityInvocationError;
    pub use crate::runtime::capability::CapabilityOrigin;
    pub use crate::runtime::capability::CapabilityRegistration;
    pub use crate::runtime::capability::CapabilityRegistrationOrigin;
    pub use crate::runtime::capability::CapabilitySpec;
    pub use crate::runtime::capability::CapabilityTarget;
    pub use crate::runtime::capability::CapabilityTargetId;
    pub use crate::runtime::capability::CapabilityValueMode;
    pub use crate::runtime::capability::CloneAdapter;
    pub use crate::runtime::capability::CloneCapability;
    pub use crate::runtime::capability::ContextRegistrationOrigin;
    pub use crate::runtime::capability::DefaultAdapter;
    pub use crate::runtime::capability::DefaultCapability;
    pub use crate::runtime::capability::SendCapability;
    pub use crate::runtime::capability::SyncCapability;
}
/// High-level static registration fragments.
pub mod registration {
    pub use crate::__private::codegen_v4::registration::MethodBinding;
    pub use crate::runtime::capability::CapabilityRegistration;
    pub use crate::runtime::capability::CapabilityRegistrationOrigin;
    pub use crate::runtime::context::handle::registration::Registration;
    pub use crate::runtime::context::handle::registration::TraitObjectRegistration;
    pub use crate::runtime::context::handle::registration::TypeDefinitionRegistration;
}
/// Structured reflection errors.
pub mod error {
    pub(crate) use crate::old_id_error::IdError;
    pub use crate::runtime::error::ContextError;
    pub use crate::runtime::error::ContextIssue;
    pub use crate::runtime::error::LookupError;
    pub use crate::runtime::error::LookupQuery;
    pub use crate::runtime::error::LookupSubject;
    pub use crate::runtime::error::RecoverableFailure;
    pub use crate::runtime::error::RecoverableFailureParts;
    pub use crate::runtime::error::ReflectionError;
    pub use crate::runtime::error::TypeMismatch;
    pub use crate::runtime::error::UnsupportedOperationError;
    pub use crate::runtime::identity::GenericArgumentIdentity;
    pub use crate::runtime::identity::IntoLookupCandidate;
    pub use crate::runtime::identity::ItemIdentity;
    pub use crate::runtime::identity::ItemKind;
    pub use crate::runtime::identity::LookupCandidate;
    pub use crate::runtime::identity::RegistrationOrigin;
    pub use crate::runtime::identity::StableItemId;
    pub use crate::runtime::identity::TypeIdentity;
}
/// Three-state descriptor lookup results.
pub mod lookup {
    pub use crate::runtime::lookup::Lookup;
}

/// Register a built-in capability for a reflected type.
#[cfg(feature = "derive")]
pub use __private::codegen_v4::capability::register_capability;
pub use construct::ConstructionInput;
pub use construct::UpdateInput;
pub use context::ReflectionContext;
pub use descriptor::FieldDescriptor;
pub use descriptor::ImplDefinitionDescriptor;
pub use descriptor::ImplDefinitions;
pub use descriptor::MethodArguments;
pub use descriptor::MethodDescriptor;
pub use descriptor::MethodNamespace;
pub use descriptor::MethodQualifiedName;
pub use descriptor::MethodSpecializationDescriptor;
pub use descriptor::TypeDescriptor;
pub use descriptor::VariantDescriptor;
pub use error::ReflectionError;
pub use invoke::Invocation;
pub use invoke::InvocationOutput;
/// The `Reflect` procedural attribute for reflected Rust type and trait
/// declarations.
#[cfg(feature = "derive")]
pub use qubit_reflect_derive::Reflect;
/// The `ReflectImpl` procedural attribute for reflected Rust impl blocks.
#[cfg(feature = "derive")]
pub use qubit_reflect_derive::ReflectImpl;
/// Build a compile-time method-instance binding for runtime reflection.
#[cfg(feature = "derive")]
pub use qubit_reflect_derive::bind_method;
/// Define an identity handle for an external trait.
#[cfg(feature = "derive")]
pub use qubit_reflect_derive::reflect_external_trait;
/// Register one monomorphized generic method in the linked method catalog.
#[cfg(feature = "derive")]
pub use qubit_reflect_derive::register_method_binding;
/// Register a compiler-checked trait-object type.
#[cfg(feature = "derive")]
pub use qubit_reflect_derive::register_trait_object;
// Dynamic-value facade.
/// Internal dependencies needed by code generated by reflection macros.
#[doc(hidden)]
#[path = "private/mod.rs"]
pub mod __private;

mod construct_vnext;
pub(crate) mod runtime;

#[path = "identity/capability_id.rs"]
mod old_capability_id;
#[path = "error/id_error.rs"]
mod old_id_error;
mod identity {
    pub(crate) use crate::old_capability_id::CapabilityId;
}
