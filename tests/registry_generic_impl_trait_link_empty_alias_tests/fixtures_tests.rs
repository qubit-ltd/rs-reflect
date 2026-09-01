// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Regression tests for fact-insufficient generic trait-alias linking.

#![allow(dead_code)]

use std::marker::PhantomData;

use qubit_reflect::error::RegistryErrorKind;
use qubit_reflect::reflect;
use qubit_reflect::reflect_impl;
use qubit_reflect::registry::ReflectRegistry;

#[reflect]
trait EmptyTrait {}

use EmptyTrait as EmptyAlias;

struct EmptyTarget<T>(PhantomData<T>);

#[reflect_impl]
impl<T> EmptyAlias for EmptyTarget<T> {}

/// Verifies an alias without declaration facts cannot become marker identity.
#[test]
fn test_generic_impl_definition_rejects_fact_free_alias() {
    let error = ReflectRegistry::initialize().expect_err("a fact-free alias must not infer a reflected marker");
    assert_eq!(error.kind(), RegistryErrorKind::ImplTraitResolution);
}
