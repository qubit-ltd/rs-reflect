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

use qubit_reflect::descriptor::ImplDefinitionDescriptor;
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

#[reflect_impl]
impl<T> crate::registry_generic_impl_trait_link_tests::path_traits::CrateQualifiedTrait for CrateQualifiedTarget<T> {
    /// Returns the crate-qualified fixture value.
    fn crate_qualified_value(&self) -> usize {
        1
    }
}

#[reflect_impl]
impl<T> path_traits::RelativeQualifiedTrait for RelativeQualifiedTarget<T> {
    /// Returns the relative-qualified fixture value.
    fn relative_qualified_value(&self) -> usize {
        2
    }
}

#[reflect_impl]
impl<T> self::SelfQualifiedTrait for SelfQualifiedTarget<T> {
    /// Returns the self-qualified fixture value.
    fn self_qualified_value(&self) -> usize {
        3
    }
}

use self::path_traits::ImportedAliasTrait as ReflectedAlias;

#[reflect_impl]
impl<T> ReflectedAlias for ImportedAliasTarget<T> {
    /// Returns the imported-alias fixture value.
    fn imported_alias_value(&self) -> usize {
        4
    }
}

mod nested_impl {
    use std::marker::PhantomData;

    use qubit_reflect::reflect_impl;

    pub(super) struct SuperQualifiedTarget<T>(PhantomData<T>);

    #[reflect_impl]
    impl<T> super::path_traits::SuperQualifiedTrait for SuperQualifiedTarget<T> {
        /// Returns the super-qualified fixture value.
        fn super_qualified_value(&self) -> usize {
            5
        }
    }
}

/// Finds the unique generic impl definition whose source trait path is `path`.
fn definition_by_trait_path(
    registry: &ReflectRegistry,
    path: &str,
) -> &'static ImplDefinitionDescriptor {
    let mut matches = registry
        .impl_definitions()
        .iter()
        .copied()
        .filter(|definition| definition.implemented_trait_path() == Some(path));
    let definition = matches
        .next()
        .expect("the generic trait impl definition must be registered");
    assert!(
        matches.next().is_none(),
        "the source trait path must identify one impl definition",
    );
    definition
}

/// Verifies legal Rust trait paths resolve to the reflected declaration marker.
#[test]
fn test_generic_impl_definition_resolves_qualified_and_aliased_trait_paths() {
    let registry = ReflectRegistry::initialize().expect(
        "qualified generic trait impl definitions must resolve uniquely",
    );
    let cases = [
        (
            "crate::registry_generic_impl_trait_link_tests::path_traits::CrateQualifiedTrait",
            "CrateQualifiedTrait",
        ),
        (
            "path_traits::RelativeQualifiedTrait",
            "RelativeQualifiedTrait",
        ),
        ("self::SelfQualifiedTrait", "SelfQualifiedTrait"),
        (
            "super::path_traits::SuperQualifiedTrait",
            "SuperQualifiedTrait",
        ),
        ("ReflectedAlias", "ImportedAliasTrait"),
    ];

    for (source_path, expected_trait) in cases {
        let definition = definition_by_trait_path(registry, source_path);
        assert_eq!(
            definition
                .implemented_trait()
                .expect("the registry must link the reflected trait marker")
                .rust_name(),
            expected_trait,
        );
    }
}
