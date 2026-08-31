// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Regression tests for conservative generic trait-alias linking.

#![allow(dead_code)]

use std::marker::PhantomData;

use qubit_reflect::error::RegistryErrorKind;
use qubit_reflect::reflect;
use qubit_reflect::reflect_impl;
use qubit_reflect::registry::ReflectRegistry;

#[reflect]
trait FirstMatchingTrait {
    /// Returns the shared fixture value.
    fn shared_value(&self) -> usize;
}

#[reflect]
trait SecondMatchingTrait {
    /// Returns the shared fixture value.
    fn shared_value(&self) -> usize;
}

use FirstMatchingTrait as AmbiguousAlias;

struct AmbiguousTarget<T>(PhantomData<T>);

#[reflect_impl]
impl<T> AmbiguousAlias for AmbiguousTarget<T> {
    /// Returns the shared fixture value.
    fn shared_value(&self) -> usize {
        1
    }
}

/// Verifies compatible declaration facts never select an arbitrary marker.
#[test]
fn test_generic_impl_definition_rejects_ambiguous_alias_facts() {
    let error = ReflectRegistry::initialize().expect_err("identical trait declaration facts must remain ambiguous");
    assert_eq!(error.kind(), RegistryErrorKind::ImplTraitResolution);
}
