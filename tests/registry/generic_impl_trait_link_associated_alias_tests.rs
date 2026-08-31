//! Regression tests for alias linking with associated-item facts.

#![allow(dead_code)]

use std::marker::PhantomData;

use qubit_reflect::reflect;
use qubit_reflect::reflect_impl;
use qubit_reflect::registry::ReflectRegistry;

#[reflect]
trait AssociatedAliasTrait {
    /// The value returned by this trait.
    type Output;

    /// The fixture limit.
    const LIMIT: usize;

    /// Returns a fixture value.
    fn associated_value(&self) -> Self::Output;
}

#[reflect]
trait AssociatedAliasDecoy {
    /// The value returned by this trait.
    type Output;

    /// A differently named fixture constant.
    const OTHER_LIMIT: usize;

    /// Returns a fixture value with the same method signature.
    fn associated_value(&self) -> Self::Output;
}

use AssociatedAliasTrait as AssociatedAlias;

struct AssociatedTarget<T>(PhantomData<T>);

#[reflect_impl]
impl<T> AssociatedAlias for AssociatedTarget<T> {
    type Output = usize;

    const LIMIT: usize = 1;

    /// Returns a fixture value.
    fn associated_value(&self) -> Self::Output {
        Self::LIMIT
    }
}

/// Verifies associated item facts participate in unique alias resolution.
#[test]
fn test_generic_impl_definition_resolves_associated_item_alias() {
    let registry = ReflectRegistry::initialize().expect("associated item facts must support unique alias resolution");
    let definition = registry
        .impl_definitions()
        .iter()
        .find(|definition| definition.implemented_trait_path() == Some("AssociatedAlias"))
        .expect("the generic impl definition must be registered");
    assert_eq!(
        definition
            .implemented_trait()
            .expect("the alias must resolve to its reflected marker")
            .rust_name(),
        "AssociatedAliasTrait",
    );
}
