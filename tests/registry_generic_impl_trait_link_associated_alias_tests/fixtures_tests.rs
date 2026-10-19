// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Regression tests for alias linking with associated-item facts.

#![allow(dead_code)]

use std::marker::PhantomData;

use AssociatedAliasTrait as AssociatedAlias;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;

#[Reflect]
trait AssociatedAliasTrait {
    /// The value returned by this trait.
    type Output;

    /// The fixture limit.
    const LIMIT: usize;

    /// Returns a fixture value.
    fn associated_value(&self) -> Self::Output;
}

#[Reflect]
trait AssociatedAliasDecoy {
    /// The value returned by this trait.
    type Output;

    /// A differently named fixture constant.
    const OTHER_LIMIT: usize;

    /// Returns a fixture value with the same method signature.
    fn associated_value(&self) -> Self::Output;
}

struct AssociatedTarget<T>(PhantomData<T>);

#[ReflectImpl]
impl<T> AssociatedAlias for AssociatedTarget<T> {
    type Output = usize;

    const LIMIT: usize = 1;

    /// Returns a fixture value.
    fn associated_value(&self) -> Self::Output {
        Self::LIMIT
    }
}

/// Verifies associated item registrations retain their trait identity.
#[test]
fn test_generic_impl_definition_retains_associated_item_alias() {
    ReflectionContext::builder()
        .include_linked()
        .build()
        .expect("associated items and the written trait path form a valid registration");
}
