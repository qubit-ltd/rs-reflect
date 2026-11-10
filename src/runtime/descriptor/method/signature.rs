// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::TypeExpression;
use super::imports::ExpressionData;
use super::imports::ReflectionContext;
use super::imports::TypeDescriptor;
use super::imports::TypeId;

include!("signature/method_visibility.rs");
include!("signature/receiver_kind.rs");
include!("signature/receiver_descriptor.rs");
include!("signature/parameter_pattern.rs");
include!("signature/passing_mode.rs");
include!("signature/parameter_metadata.rs");
include!("signature/parameter_descriptor.rs");
include!("signature/parameters.rs");
include!("signature/return_kind.rs");
include!("signature/return_metadata.rs");
include!("signature/return_descriptor.rs");
include!("signature/method_qualifiers.rs");
include!("signature/function_abi.rs");
