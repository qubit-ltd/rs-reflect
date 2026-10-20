// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Tests explicitly typed supertrait providers and their retained closure.

#![allow(dead_code)]

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::TypeDescriptor;

#[Reflect]
trait Root {}

#[Reflect(supertrait_binding(0 => Root))]
trait Middle: Root {}

#[Reflect(supertrait_binding(0 => Middle))]
trait Leaf: Middle {}

#[Reflect(supertrait_binding(0 => Root))]
trait Left: Root {}

#[Reflect(supertrait_binding(0 => Root))]
trait Right: Root {}

#[Reflect(
    supertrait_binding(0 => Left),
    supertrait_binding(1 => Right),
)]
trait Diamond: Left + Right {}

use self::Root as AliasRoot;

#[Reflect(supertrait_binding(0 => Root))]
trait AliasChild: AliasRoot {}

#[Reflect(supertrait_binding(0 => Root))]
trait GenericRootChild<T>: Root {}

#[Reflect(supertrait_binding(0 => Root))]
trait IrrelevantLifetimeGenericChild<'a, T, const N: usize>: Root {}

#[Reflect]
trait HrtbParent<T> {}

#[Reflect(supertrait_binding(0 => HrtbParent))]
trait HrtbChild: for<'a> HrtbParent<&'a str> {}

#[Reflect]
trait FixedArgumentParent<T> {}

#[Reflect]
trait FixedArgumentChild: FixedArgumentParent<u8> {}

#[Reflect(supertrait_binding(0 => FixedArgumentParent))]
trait FixedArgumentBoundChild: FixedArgumentParent<u8> {}

#[Reflect]
trait DefaultArgumentParent<T: qubit_reflect::descriptor::Reflect> {}

#[Reflect(supertrait_binding(0 => DefaultArgumentParent))]
trait DefaultArgumentChild<T: qubit_reflect::descriptor::Reflect = u8>: DefaultArgumentParent<T> {}

#[Reflect]
trait ProjectionSource {
    type Output;
}

#[Reflect]
trait ProjectionParent<T> {}

#[Reflect(supertrait_binding(0 => ProjectionParent))]
trait ProjectionChild<T: ProjectionSource>: ProjectionParent<<T as ProjectionSource>::Output> {}

#[Reflect]
struct ChainTarget;

#[ReflectImpl(reflected_trait = Leaf)]
impl Leaf for ChainTarget {}
impl Middle for ChainTarget {}
impl Root for ChainTarget {}

#[Reflect]
struct DiamondTarget;

#[ReflectImpl(reflected_trait = Diamond)]
impl Diamond for DiamondTarget {}
impl Left for DiamondTarget {}
impl Right for DiamondTarget {}
impl Root for DiamondTarget {}

#[Reflect]
struct AliasTarget;

#[ReflectImpl(reflected_trait = AliasChild)]
impl AliasChild for AliasTarget {}
impl Root for AliasTarget {}

#[Reflect]
struct HrtbTarget;

// Keep the explicit binder: this implementation must satisfy the `for<'a>`
// supertrait.
#[allow(clippy::needless_lifetimes)]
impl<'a> HrtbParent<&'a str> for HrtbTarget {}

#[ReflectImpl(reflected_trait = HrtbChild)]
impl HrtbChild for HrtbTarget {}

#[Reflect]
struct FixedArgumentTarget;

impl FixedArgumentParent<u8> for FixedArgumentTarget {}

#[ReflectImpl(reflected_trait = FixedArgumentChild)]
impl FixedArgumentChild for FixedArgumentTarget {}

#[Reflect]
struct FixedArgumentBoundTarget;

impl FixedArgumentParent<u8> for FixedArgumentBoundTarget {}

#[Reflect]
struct DefaultArgumentTarget;

#[ReflectImpl(reflected_trait = DefaultArgumentChild)]
impl DefaultArgumentChild for DefaultArgumentTarget {}
impl DefaultArgumentParent<u8> for DefaultArgumentTarget {}

#[Reflect]
struct ProjectionTarget;

impl ProjectionSource for ProjectionTarget {
    type Output = u8;
}
impl ProjectionParent<u8> for ProjectionTarget {}

#[ReflectImpl(reflected_trait = ProjectionChild::<ProjectionTarget>)]
impl ProjectionChild<ProjectionTarget> for ProjectionTarget {}

#[ReflectImpl(reflected_trait = FixedArgumentBoundChild)]
impl FixedArgumentBoundChild for FixedArgumentBoundTarget {}

#[Reflect]
struct GenericRootTarget;

#[ReflectImpl(reflected_trait = GenericRootChild::<u8>)]
impl GenericRootChild<u8> for GenericRootTarget {}
impl Root for GenericRootTarget {}

#[Reflect]
struct IrrelevantLifetimeGenericTarget;

#[ReflectImpl(reflected_trait = IrrelevantLifetimeGenericChild)]
impl IrrelevantLifetimeGenericChild<'static, String, 7> for IrrelevantLifetimeGenericTarget {}
impl Root for IrrelevantLifetimeGenericTarget {}

#[test]
fn test_typed_supertrait_bindings_retain_three_level_closure() {
    let application = TypeDescriptor::of::<ChainTarget>()
        .trait_impls()
        .expect("the target's impl metadata registers")
        .iter()
        .find_map(|implementation| implementation.implemented_trait())
        .expect("the leaf application is retained");

    let direct: Vec<_> = application
        .direct_supertraits()
        .iter()
        .map(|trait_| trait_.name())
        .collect();
    let all: Vec<_> = application
        .all_supertraits()
        .iter()
        .map(|trait_| trait_.name())
        .collect();
    assert_eq!(direct, ["Middle"]);
    assert_eq!(all, ["Middle", "Root"]);
    assert_eq!(format!("{:?}", application.supertrait_completeness()), "Complete");
    assert!(
        application
            .all_supertraits()
            .iter()
            .all(|trait_| trait_.find_definition().is_some())
    );
}

#[test]
fn test_typed_supertrait_closure_deduplicates_diamond_identity() {
    let application = TypeDescriptor::of::<DiamondTarget>()
        .trait_impls()
        .expect("the target's impl metadata registers")
        .iter()
        .find_map(|implementation| implementation.implemented_trait())
        .expect("the diamond application is retained");

    let direct: Vec<_> = application
        .direct_supertraits()
        .iter()
        .map(|trait_| trait_.name())
        .collect();
    let all: Vec<_> = application
        .all_supertraits()
        .iter()
        .map(|trait_| trait_.name())
        .collect();
    assert_eq!(direct, ["Left", "Right"]);
    assert_eq!(all, ["Left", "Root", "Right"]);
    assert_eq!(all.iter().filter(|name| **name == "Root").count(), 1);
    assert_eq!(format!("{:?}", application.supertrait_completeness()), "Complete");
}

#[test]
fn test_supertrait_binding_resolves_alias_to_typed_definition_and_keeps_source_path() {
    let application = TypeDescriptor::of::<AliasTarget>()
        .trait_impls()
        .expect("the target's impl metadata registers")
        .iter()
        .find_map(|implementation| implementation.implemented_trait())
        .expect("the alias child application is retained");
    let parent = application
        .direct_supertraits()
        .iter()
        .next()
        .expect("the bound parent application is retained");

    assert_eq!(parent.path(), "AliasRoot");
    assert_eq!(parent.find_definition().unwrap().name(), "Root");
    assert_eq!(format!("{:?}", application.supertrait_completeness()), "Complete");
}

#[test]
fn test_higher_ranked_supertrait_binding_keeps_closure_incomplete() {
    let application = TypeDescriptor::of::<HrtbTarget>()
        .trait_impls()
        .expect("the target's impl metadata registers")
        .iter()
        .find_map(|implementation| implementation.implemented_trait())
        .expect("the HRTB child application is retained");
    let parent = application
        .direct_supertraits()
        .iter()
        .next()
        .expect("the source supertrait is retained");

    assert_eq!(parent.name(), "HrtbParent");
    assert_eq!(parent.path(), "HrtbParent");
    assert_eq!(parent.find_definition().unwrap().name(), "HrtbParent");
    assert_eq!(parent.arguments().len(), 1);
    let argument = parent
        .arguments()
        .get_at(0)
        .expect("the higher-ranked type argument is retained")
        .type_argument()
        .expect("the parent argument is a type");
    assert!(argument.concrete_type().is_err());
    assert!(argument.expression().display().to_string().contains("str"));
    assert_eq!(format!("{:?}", application.supertrait_completeness()), "Incomplete");
}

#[test]
fn test_irrelevant_child_lifetime_does_not_block_typed_supertrait_binding() {
    let application = TypeDescriptor::of::<IrrelevantLifetimeGenericTarget>()
        .trait_impls()
        .expect("the target's impl metadata registers")
        .iter()
        .find_map(|implementation| implementation.implemented_trait())
        .expect("the generic child application is retained");
    let parent = application
        .direct_supertraits()
        .iter()
        .next()
        .expect("the typed root edge is retained");

    assert_eq!(parent.find_definition().unwrap().name(), "Root");
    assert_eq!(format!("{:?}", application.supertrait_completeness()), "Complete");
}

#[test]
fn test_generic_supertrait_without_provider_keeps_concrete_argument_and_incomplete_status() {
    let application = TypeDescriptor::of::<FixedArgumentTarget>()
        .trait_impls()
        .expect("the target's impl metadata registers")
        .iter()
        .find_map(|implementation| implementation.implemented_trait())
        .expect("the fixed argument child application is retained");
    let parent = application
        .direct_supertraits()
        .iter()
        .next()
        .expect("the source supertrait is retained");
    let concrete = parent
        .arguments()
        .get_at(0)
        .expect("the generic argument remains available")
        .type_argument()
        .expect("the argument is a type")
        .concrete_type()
        .expect("the concrete type identity is retained");

    assert!(parent.find_definition().is_none());
    assert_eq!(concrete.type_id(), TypeDescriptor::of::<u8>().type_id());
    assert_eq!(format!("{:?}", application.supertrait_completeness()), "Incomplete");
}

#[test]
fn test_bound_generic_supertrait_application_stays_incomplete() {
    let application = TypeDescriptor::of::<FixedArgumentBoundTarget>()
        .trait_impls()
        .expect("the target's impl metadata registers")
        .iter()
        .find_map(|implementation| implementation.implemented_trait())
        .expect("the fixed argument child application is retained");
    let parent = application
        .direct_supertraits()
        .iter()
        .next()
        .expect("the explicitly bound parent application is retained");
    let concrete = parent
        .arguments()
        .get_at(0)
        .expect("the generic argument remains available")
        .type_argument()
        .expect("the argument is a type")
        .concrete_type()
        .expect("the concrete type identity is retained");

    assert_eq!(parent.find_definition().unwrap().name(), "FixedArgumentParent");
    assert_eq!(concrete.type_id(), TypeDescriptor::of::<u8>().type_id());
    assert_eq!(format!("{:?}", application.supertrait_completeness()), "Incomplete");
}

#[test]
fn test_defaulted_child_argument_is_resolved_for_supertrait_metadata() {
    let application = TypeDescriptor::of::<DefaultArgumentTarget>()
        .trait_impls()
        .expect("the target's impl metadata registers")
        .iter()
        .find_map(|implementation| implementation.implemented_trait())
        .expect("the defaulted child application is retained");
    let parent = application
        .direct_supertraits()
        .iter()
        .next()
        .expect("the source supertrait is retained");
    let concrete = parent
        .arguments()
        .get_at(0)
        .expect("the defaulted generic argument remains available")
        .type_argument()
        .expect("the argument is a type")
        .concrete_type()
        .expect("the compiler-resolved default is concrete");

    assert_eq!(concrete.type_id(), TypeDescriptor::of::<u8>().type_id());
    assert_eq!(parent.find_definition().unwrap().name(), "DefaultArgumentParent");
    assert_eq!(format!("{:?}", application.supertrait_completeness()), "Incomplete");
}

#[test]
fn test_projected_supertrait_argument_retains_source_without_concrete_identity() {
    let application = TypeDescriptor::of::<ProjectionTarget>()
        .trait_impls()
        .expect("the target's impl metadata registers")
        .iter()
        .find_map(|implementation| implementation.implemented_trait())
        .expect("the projection child application is retained");
    let parent = application
        .direct_supertraits()
        .iter()
        .next()
        .expect("the explicitly bound projected parent is retained");
    let argument = parent
        .arguments()
        .get_at(0)
        .expect("the projected argument is retained")
        .type_argument()
        .expect("the parent argument is a type");

    assert_eq!(parent.find_definition().unwrap().name(), "ProjectionParent");
    assert!(argument.find_concrete_type().is_none());
    assert!(argument.expression().display().to_string().contains("ProjectionSource"));
    assert_eq!(format!("{:?}", application.supertrait_completeness()), "Incomplete");
}

#[test]
fn test_unrelated_child_generic_keeps_typed_direct_and_recursive_supertrait() {
    let application = TypeDescriptor::of::<GenericRootTarget>()
        .trait_impls()
        .expect("the target's impl metadata registers")
        .iter()
        .find_map(|implementation| implementation.implemented_trait())
        .expect("the generic child application is retained");

    let direct: Vec<_> = application
        .direct_supertraits()
        .iter()
        .map(|trait_| trait_.name())
        .collect();
    let all: Vec<_> = application
        .all_supertraits()
        .iter()
        .map(|trait_| trait_.name())
        .collect();
    assert_eq!(direct, ["Root"]);
    assert_eq!(all, ["Root"]);
    assert_eq!(
        application
            .direct_supertraits()
            .iter()
            .next()
            .expect("the typed root edge is retained")
            .find_definition()
            .expect("the root definition is linked")
            .name(),
        "Root"
    );
    assert_eq!(format!("{:?}", application.supertrait_completeness()), "Complete");
}
