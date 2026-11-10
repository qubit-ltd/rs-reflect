// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::InvocationAdapters;
use super::NO_ADAPTERS;
use super::ParameterMetadata;
use super::imports::Any;
use super::imports::ConstExpression;
use super::imports::ExpressionData;
use super::imports::GenericArgumentIdentity;
use super::imports::GenericArgumentMetadata;
use super::imports::GenericArgumentSelector;
use super::imports::GenericParameterMetadata;
use super::imports::GenericParameterMetadataKind;
use super::imports::InvocationAvailability;
use super::imports::InvocationUnavailableReason;
use super::imports::TypeId;

include!("specialization/method_specialization_argument.rs");
include!("specialization/method_specialization_argument_value.rs");
include!("specialization/const_integer_kind.rs");
include!("specialization/method_specialization_spec.rs");
include!("specialization/resolved_method_specialization.rs");
include!("specialization/resolved_method_arguments.rs");
include!("specialization/method_arguments.rs");
