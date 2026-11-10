// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::GenericDefinitionMetadata;
use super::InvocationAdapters;
use super::MethodQualifiers;
use super::MethodSpecializationSpec;
use super::MethodVisibility;
use super::ParameterMetadata;
use super::ReceiverDescriptor;
use super::ResolvedMethodSpecialization;
use super::ReturnKind;
use super::ReturnMetadata;
use super::imports::InvocationUnavailableReason;
use super::imports::TypeDescriptor;

include!("metadata/method_namespace.rs");
include!("metadata/method_qualified_name.rs");
include!("metadata/method_implementation_source.rs");
include!("metadata/method_availability.rs");
include!("metadata/method_metadata.rs");
include!("metadata/method_namespace_metadata.rs");
include!("metadata/method_metadata_spec.rs");
include!("metadata/effective_method.rs");
include!("metadata/context_effective_method.rs");
