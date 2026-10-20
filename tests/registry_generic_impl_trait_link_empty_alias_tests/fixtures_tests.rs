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

use EmptyTrait as EmptyAlias;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;

#[Reflect]
trait EmptyTrait {
    // empty
}

struct EmptyTarget<T>(PhantomData<T>);

#[ReflectImpl(reflected_trait = EmptyTrait)]
impl<T> EmptyAlias for EmptyTarget<T> {}

/// Verifies an empty trait alias still retains its compiler-resolved identity.
#[test]
fn test_generic_impl_definition_retains_fact_free_trait_identity() {
    ReflectionContext::builder()
        .include_linked()
        .build()
        .expect("an empty trait impl retains its written trait identity");
}
