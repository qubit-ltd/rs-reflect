// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::EffectiveMethod;
use super::InvocationAdapters;
use super::MethodArguments;
use super::MethodAvailability;
use super::MethodImplementationSource;
use super::MethodNamespace;
use super::MethodNamespaceMetadata;
use super::MethodQualifiedName;
use super::MethodQualifiers;
use super::MethodVisibility;
use super::NO_ADAPTERS;
use super::ParameterDescriptor;
use super::ParameterMetadata;
use super::ParameterPattern;
use super::Parameters;
use super::ReceiverDescriptor;
use super::ResolvedMethodSpecialization;
use super::ReturnDescriptor;
use super::imports::CaughtInvocationFailure;
use super::imports::GenericDefinitionDescriptor;
use super::imports::Invocation;
use super::imports::InvocationAttempt;
use super::imports::InvocationAvailability;
use super::imports::InvocationError;
use super::imports::InvocationErrorKind;
use super::imports::InvocationFailure;
use super::imports::InvocationOutput;
use super::imports::InvocationPanic;
use super::imports::ItemIdentity;
use super::imports::ItemKind;
use super::imports::LookupCandidate;
use super::imports::LookupError;
use super::imports::LookupQuery;
use super::imports::LookupSubject;
use super::imports::ModeAdapter;
use super::imports::PinnedInvocation;
use super::imports::PinnedInvocationFailure;
use super::imports::ReflectionContext;
use super::imports::StableItemId;
use super::imports::TypeDescriptor;
use super::imports::TypeId;
use super::imports::ValueMode;

include!("method_descriptor/method_descriptor.rs");
include!("method_descriptor/method_specializations.rs");
include!("method_descriptor/method_specialization_descriptor.rs");
