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

use qubit_reflect::reflect;
use qubit_reflect::reflect_impl;
use qubit_reflect::registry::ReflectRegistry;

pub(crate) mod path_traits {
    use qubit_reflect::reflect;

    #[reflect]
    pub(crate) trait CrateQualifiedTrait {
        /// Returns the crate-qualified fixture value.
        fn crate_qualified_value(&self) -> usize;
    }

    #[reflect]
    pub(crate) trait RelativeQualifiedTrait {
        /// Returns the relative-qualified fixture value.
        fn relative_qualified_value(&self) -> usize;
    }

    #[reflect]
    pub(crate) trait SuperQualifiedTrait {
        /// Returns the super-qualified fixture value.
        fn super_qualified_value(&self) -> usize;
    }

    #[reflect]
    pub(crate) trait ImportedAliasTrait {
        /// Returns the imported-alias fixture value.
        fn imported_alias_value(&self) -> usize;
    }
}

#[reflect]
trait SelfQualifiedTrait {
    /// Returns the self-qualified fixture value.
    fn self_qualified_value(&self) -> usize;
}

struct CrateQualifiedTarget<T>(PhantomData<T>);
struct RelativeQualifiedTarget<T>(PhantomData<T>);
struct SelfQualifiedTarget<T>(PhantomData<T>);
struct ImportedAliasTarget<T>(PhantomData<T>);

use self::path_traits::__qubit_reflect_trait_definition_CrateQualifiedTrait;
use self::path_traits::__qubit_reflect_trait_definition_ImportedAliasTrait;
use self::path_traits::__qubit_reflect_trait_definition_RelativeQualifiedTrait;

#[reflect_impl(definition_provider_v2 = __qubit_reflect_trait_definition_CrateQualifiedTrait)]
impl<T> crate::registry_generic_impl_trait_link_tests::path_traits::CrateQualifiedTrait for CrateQualifiedTarget<T> {
    /// Returns the crate-qualified fixture value.
    fn crate_qualified_value(&self) -> usize {
        1
    }
}

#[reflect_impl(definition_provider_v2 = __qubit_reflect_trait_definition_RelativeQualifiedTrait)]
impl<T> path_traits::RelativeQualifiedTrait for RelativeQualifiedTarget<T> {
    /// Returns the relative-qualified fixture value.
    fn relative_qualified_value(&self) -> usize {
        2
    }
}

#[reflect_impl(definition_provider_v2 = __qubit_reflect_trait_definition_SelfQualifiedTrait)]
impl<T> self::SelfQualifiedTrait for SelfQualifiedTarget<T> {
    /// Returns the self-qualified fixture value.
    fn self_qualified_value(&self) -> usize {
        3
    }
}

use self::path_traits::ImportedAliasTrait as ReflectedAlias;

#[reflect_impl(definition_provider_v2 = __qubit_reflect_trait_definition_ImportedAliasTrait)]
impl<T> ReflectedAlias for ImportedAliasTarget<T> {
    /// Returns the imported-alias fixture value.
    fn imported_alias_value(&self) -> usize {
        4
    }
}

mod nested_impl {
    use std::marker::PhantomData;

    use qubit_reflect::reflect_impl;

    use super::path_traits::__qubit_reflect_trait_definition_SuperQualifiedTrait;

    pub(super) struct SuperQualifiedTarget<T>(PhantomData<T>);

    #[reflect_impl(definition_provider_v2 = __qubit_reflect_trait_definition_SuperQualifiedTrait)]
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
    let registry =
        ReflectRegistry::initialize().expect("qualified generic trait impl definitions must resolve uniquely");
    let mut resolved = registry
        .impl_definitions()
        .iter()
        .filter_map(|definition| definition.implemented_trait_in(registry))
        .map(|trait_definition| trait_definition.rust_name())
        .filter(|name| {
            matches!(
                *name,
                "CrateQualifiedTrait"
                    | "RelativeQualifiedTrait"
                    | "SelfQualifiedTrait"
                    | "SuperQualifiedTrait"
                    | "ImportedAliasTrait"
            )
        })
        .collect::<Vec<_>>();
    resolved.sort_unstable();
    assert_eq!(
        resolved,
        [
            "CrateQualifiedTrait",
            "ImportedAliasTrait",
            "RelativeQualifiedTrait",
            "SelfQualifiedTrait",
            "SuperQualifiedTrait"
        ]
    );
}
