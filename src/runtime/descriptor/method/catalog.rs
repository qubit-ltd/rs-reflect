// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::ContextEffectiveMethod;
use super::EffectiveMethod;
use super::MethodDescriptor;
#[cfg(test)]
use super::MethodNamespaceMetadata;
use super::imports::ContextError;
use super::imports::Lookup;
use super::imports::LookupError;
use super::imports::LookupQuery;
use super::imports::LookupSubject;
use super::imports::TypeDescriptor;
use super::imports::TypeId;

include!("catalog/method_catalog.rs");
include!("catalog/method_catalog_methods.rs");
include!("catalog/method_catalog_method.rs");
include!("catalog/method_catalog_method_iter.rs");
include!("catalog/method_catalog_registration.rs");
