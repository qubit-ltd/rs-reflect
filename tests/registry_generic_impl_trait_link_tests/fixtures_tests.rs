// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Regression tests for linking generic impl definitions to reflected traits.

#![allow(dead_code)]

use std::marker::PhantomData;

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;

use self::path_traits::ImportedAliasTrait as ReflectedAlias;

pub(crate) mod path_traits {
    use qubit_reflect::Reflect;

    #[Reflect]
    pub(crate) trait CrateQualifiedTrait {
        /// Returns the crate-qualified fixture value.
        fn crate_qualified_value(&self) -> usize;
    }

    #[Reflect]
    pub(crate) trait RelativeQualifiedTrait {
        /// Returns the relative-qualified fixture value.
        fn relative_qualified_value(&self) -> usize;
    }

    #[Reflect]
    pub(crate) trait SuperQualifiedTrait {
        /// Returns the super-qualified fixture value.
        fn super_qualified_value(&self) -> usize;
    }

    #[Reflect]
    pub(crate) trait ImportedAliasTrait {
        /// Returns the imported-alias fixture value.
        fn imported_alias_value(&self) -> usize;
    }
}

#[Reflect]
trait SelfQualifiedTrait {
    /// Returns the self-qualified fixture value.
    fn self_qualified_value(&self) -> usize;
}

struct CrateQualifiedTarget<T>(PhantomData<T>);
struct RelativeQualifiedTarget<T>(PhantomData<T>);
struct SelfQualifiedTarget<T>(PhantomData<T>);
struct ImportedAliasTarget<T>(PhantomData<T>);

#[ReflectImpl]
impl<T> crate::registry_generic_impl_trait_link_tests::path_traits::CrateQualifiedTrait for CrateQualifiedTarget<T> {
    /// Returns the crate-qualified fixture value.
    fn crate_qualified_value(&self) -> usize {
        1
    }
}

#[ReflectImpl]
impl<T> path_traits::RelativeQualifiedTrait for RelativeQualifiedTarget<T> {
    /// Returns the relative-qualified fixture value.
    fn relative_qualified_value(&self) -> usize {
        2
    }
}

#[ReflectImpl]
impl<T> self::SelfQualifiedTrait for SelfQualifiedTarget<T> {
    /// Returns the self-qualified fixture value.
    fn self_qualified_value(&self) -> usize {
        3
    }
}

#[ReflectImpl]
impl<T> ReflectedAlias for ImportedAliasTarget<T> {
    /// Returns the imported-alias fixture value.
    fn imported_alias_value(&self) -> usize {
        4
    }
}

mod nested_impl {
    use std::marker::PhantomData;

    use qubit_reflect::ReflectImpl;

    use super::path_traits::__qubit_reflect_trait_definition_SuperQualifiedTrait;

    pub(super) struct SuperQualifiedTarget<T>(PhantomData<T>);

    #[ReflectImpl]
    impl<T> super::path_traits::SuperQualifiedTrait for SuperQualifiedTarget<T> {
        /// Returns the super-qualified fixture value.
        fn super_qualified_value(&self) -> usize {
            5
        }
    }
}

/// Verifies explicit providers resolve every generic trait implementation.
#[test]
fn test_generic_impl_definition_resolves_explicit_trait_providers() {
    ReflectionContext::builder()
        .include_linked()
        .build()
        .expect("the generated generic impl registrations retain their resolved trait paths");
}
