// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Method declarations, effective implementations, and callable
//! specializations.

pub(crate) use crate::runtime::descriptor::generic::GenericDefinitionMetadata;
pub use crate::runtime::expression::TypeExpression;

mod imports {
    pub(super) use std::any::Any;
    pub(super) use std::any::TypeId;

    pub(super) use crate::runtime::context::ReflectionContext;
    pub(super) use crate::runtime::descriptor::TypeDescriptor;
    pub(super) use crate::runtime::descriptor::generic::GenericArgumentMetadata;
    pub(super) use crate::runtime::descriptor::generic::GenericDefinitionDescriptor;
    pub(super) use crate::runtime::descriptor::generic::GenericParameterMetadata;
    pub(super) use crate::runtime::descriptor::generic::GenericParameterMetadataKind;
    pub(super) use crate::runtime::error::ContextError;
    pub(super) use crate::runtime::error::LookupError;
    pub(super) use crate::runtime::error::LookupQuery;
    pub(super) use crate::runtime::error::LookupSubject;
    pub(super) use crate::runtime::expression::ConstExpression;
    pub(super) use crate::runtime::expression::ExpressionData;
    pub(super) use crate::runtime::identity::GenericArgumentIdentity;
    pub(super) use crate::runtime::identity::GenericArgumentSelector;
    pub(super) use crate::runtime::identity::ItemIdentity;
    pub(super) use crate::runtime::identity::ItemKind;
    pub(super) use crate::runtime::identity::LookupCandidate;
    pub(super) use crate::runtime::identity::StableItemId;
    pub(super) use crate::runtime::invoke::CaughtInvocationFailure;
    pub(super) use crate::runtime::invoke::Invocation;
    pub(super) use crate::runtime::invoke::InvocationAdapter;
    pub(super) use crate::runtime::invoke::InvocationAttempt;
    pub(super) use crate::runtime::invoke::InvocationAvailability;
    pub(super) use crate::runtime::invoke::InvocationError;
    pub(super) use crate::runtime::invoke::InvocationErrorKind;
    pub(super) use crate::runtime::invoke::InvocationFailure;
    pub(super) use crate::runtime::invoke::InvocationOutput;
    pub(super) use crate::runtime::invoke::InvocationPanic;
    pub(super) use crate::runtime::invoke::InvocationUnavailableReason;
    pub(super) use crate::runtime::invoke::Local;
    pub(super) use crate::runtime::invoke::ModeAdapter;
    pub(super) use crate::runtime::invoke::PinnedInvocation;
    pub(super) use crate::runtime::invoke::PinnedInvocationAdapter;
    pub(super) use crate::runtime::invoke::PinnedInvocationFailure;
    pub(super) use crate::runtime::invoke::ThreadSafe;
    pub(super) use crate::runtime::invoke::ValueMode;
    pub(super) use crate::runtime::lookup::Lookup;
}

mod adapters;
mod catalog;
mod metadata;
mod method_descriptor;
mod signature;
mod specialization;

pub use adapters::InvocationAdapters;
pub use adapters::InvocationAdaptersBuilder;
use adapters::NO_ADAPTERS;
pub(crate) use catalog::MethodCatalog;
pub use catalog::MethodCatalogRegistration;
pub(crate) use metadata::ContextEffectiveMethod;
pub use metadata::EffectiveMethod;
pub use metadata::MethodAvailability;
pub use metadata::MethodImplementationSource;
pub use metadata::MethodMetadata;
pub use metadata::MethodMetadataSpec;
pub use metadata::MethodNamespace;
pub use metadata::MethodNamespaceMetadata;
pub use metadata::MethodQualifiedName;
pub use method_descriptor::MethodDescriptor;
pub use method_descriptor::MethodSpecializationDescriptor;
pub use method_descriptor::MethodSpecializations;
pub use signature::FunctionAbi;
pub use signature::MethodQualifiers;
pub use signature::MethodVisibility;
pub use signature::ParameterDescriptor;
pub use signature::ParameterMetadata;
pub use signature::ParameterPattern;
pub use signature::Parameters;
pub use signature::PassingMode;
pub use signature::ReceiverDescriptor;
pub use signature::ReceiverKind;
pub use signature::ReturnDescriptor;
pub use signature::ReturnKind;
pub use signature::ReturnMetadata;
#[cfg(test)]
use specialization::ConstIntegerKind;
pub use specialization::MethodArguments;
pub use specialization::MethodSpecializationArgument;
pub use specialization::MethodSpecializationSpec;
#[cfg(test)]
pub(crate) use specialization::ResolvedMethodArguments;
pub(crate) use specialization::ResolvedMethodSpecialization;
#[cfg(test)]
use specialization::integer_kind_for_expression;

#[cfg(test)]
mod tests;
