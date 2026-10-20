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
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::ImplKind;

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

#[Reflect]
struct CrateQualifiedTarget<T: 'static>(PhantomData<T>);
struct RelativeQualifiedTarget<T>(PhantomData<T>);
struct SelfQualifiedTarget<T>(PhantomData<T>);
struct ImportedAliasTarget<T>(PhantomData<T>);

#[ReflectImpl(reflected_trait = crate::registry_generic_impl_trait_link_tests::path_traits::CrateQualifiedTrait)]
impl<T> crate::registry_generic_impl_trait_link_tests::path_traits::CrateQualifiedTrait for CrateQualifiedTarget<T> {
    /// Returns the crate-qualified fixture value.
    fn crate_qualified_value(&self) -> usize {
        1
    }
}

#[ReflectImpl(reflected_trait = path_traits::RelativeQualifiedTrait)]
impl<T> path_traits::RelativeQualifiedTrait for RelativeQualifiedTarget<T> {
    /// Returns the relative-qualified fixture value.
    fn relative_qualified_value(&self) -> usize {
        2
    }
}

#[ReflectImpl(reflected_trait = self::SelfQualifiedTrait)]
impl<T> self::SelfQualifiedTrait for SelfQualifiedTarget<T> {
    /// Returns the self-qualified fixture value.
    fn self_qualified_value(&self) -> usize {
        3
    }
}

#[ReflectImpl(reflected_trait = path_traits::ImportedAliasTrait)]
impl<T> ReflectedAlias for ImportedAliasTarget<T> {
    /// Returns the imported-alias fixture value.
    fn imported_alias_value(&self) -> usize {
        4
    }
}

mod nested_impl {
    use std::marker::PhantomData;

    use qubit_reflect::ReflectImpl;

    pub(super) struct SuperQualifiedTarget<T>(PhantomData<T>);

    #[ReflectImpl(reflected_trait = super::path_traits::SuperQualifiedTrait)]
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
    let context = ReflectionContext::builder()
        .include_linked()
        .build()
        .expect("the generated generic impl registrations retain their resolved trait paths");
    let alias_definition = context
        .trait_definitions_by_short_name("ImportedAliasTrait")
        .expect("the imported alias trait definition is registered")
        .iter()
        .next()
        .expect("the imported alias trait has one definition");
    let impl_definition = context
        .impl_definitions()
        .expect("generic impl source definitions are available")
        .iter()
        .find(|definition| {
            definition
                .methods()
                .iter()
                .any(|method| method.name() == "imported_alias_value")
        })
        .expect("the imported-alias generic impl definition is registered");
    let implemented_trait = impl_definition
        .implemented_trait()
        .expect("the generic impl retains its source trait expression");

    assert_eq!(implemented_trait.path(), "ReflectedAlias");
    assert_eq!(
        implemented_trait
            .find_definition()
            .expect("the explicit provider remains linked from the source definition")
            .id(),
        alias_definition.id(),
    );
}

/// Verifies a generic source definition is not treated as a concrete impl
/// application until its target type is included in the context.
#[test]
fn test_generic_impl_source_definition_does_not_fabricate_applications() {
    let definition_context = ReflectionContext::builder()
        .include_linked()
        .build()
        .expect("linked generic source definitions must be queryable");
    let definition = definition_context
        .impl_definitions()
        .expect("source impl definitions are available")
        .iter()
        .find(|definition| {
            definition.kind() == ImplKind::Trait
                && definition
                    .methods()
                    .iter()
                    .any(|method| method.name() == "crate_qualified_value")
        })
        .expect("the crate-qualified generic source definition is registered");
    assert_eq!(definition.generics().parameters().len(), 1);

    assert!(
        definition
            .instances()
            .expect("the generic definition has an application collection")
            .is_empty(),
        "a generic definition alone must not fabricate a concrete target application"
    );
    assert!(
        TypeDescriptor::of_in::<CrateQualifiedTarget<u8>>(&definition_context)
            .trait_impls()
            .expect("the concrete descriptor can query linked implementations")
            .is_empty(),
        "the concrete target descriptor must not expose an application that was not registered"
    );
}
