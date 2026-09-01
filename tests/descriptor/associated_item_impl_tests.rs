// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Integration coverage for associated type and constant impl bindings.

use std::marker::PhantomData;

use qubit_reflect::Reflect;
use qubit_reflect::Reflect as FrameworkReflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::AssociatedConstImplementationSource;
use qubit_reflect::descriptor::AssociatedConstReadUnavailableReason;
use qubit_reflect::descriptor::ImplDescriptor;
use qubit_reflect::expression::LifetimeExpression;
use qubit_reflect::expression::PredicateDescriptor;
use qubit_reflect::expression::TypeExpression;
use qubit_reflect::reflect;
use qubit_reflect::reflect_impl;
use qubit_reflect::registry::ReflectRegistry;

#[derive(Reflect)]
struct AssociatedItemTarget;

#[reflect]
trait AssociatedContract {
    type Reflected: Reflect;
    type Unproven;

    const DEFAULTED: u32 = 17;
    const OVERRIDDEN: u32 = 19;
    const UNREADABLE: Self::Unproven;
}

#[reflect_impl]
impl AssociatedContract for AssociatedItemTarget {
    type Unproven = u8;
    type Reflected = u16;

    const UNREADABLE: Self::Unproven = 29;
    const OVERRIDDEN: u32 = 23;
}

trait Projection {
    type Output;
    const VALUE: Self::Output;
}

impl Projection for u8 {
    type Output = String;
    const VALUE: Self::Output = String::new();
}

#[derive(Reflect)]
struct GenericAssociatedItemTarget<T: 'static>(
    #[reflect(opaque)] PhantomData<T>,
);

#[reflect]
trait SymbolicAssociatedContract {
    type Projected;
}

#[reflect_impl(specialize(T = u8))]
impl<T> SymbolicAssociatedContract for GenericAssociatedItemTarget<T>
where
    T: Projection + 'static,
{
    type Projected = <T as Projection>::Output;
}

#[reflect]
trait SymbolicAssociatedConstContract<T: Projection + 'static> {
    const PROJECTED: <T as Projection>::Output = T::VALUE;
}

#[reflect_impl(specialize(T = u8))]
impl<T> SymbolicAssociatedConstContract<T> for GenericAssociatedItemTarget<T> where
    T: Projection + 'static
{
}

#[derive(Reflect)]
struct AliasedBoundTarget;

#[reflect]
trait AliasedBoundContract {
    type Reflected: crate::descriptor::associated_item_impl_tests::FrameworkReflect;
}

#[reflect_impl]
impl AliasedBoundContract for AliasedBoundTarget {
    type Reflected = u32;
}

#[derive(Reflect)]
struct ImplBoundTarget<T: 'static>(#[reflect(opaque)] PhantomData<T>);

#[reflect]
trait ImplBoundContract {
    type Reflected;
}

#[reflect_impl(specialize(T = u8))]
impl<T: FrameworkReflect> ImplBoundContract for ImplBoundTarget<T> {
    type Reflected = T;
}

fn increment(value: u8) -> u8 {
    value + 1
}

fn observe(_: &u8) {}

#[derive(Reflect)]
struct OwnedConstShapeTarget;

#[reflect]
trait OwnedConstShapeContract {
    const CALLBACK: fn(u8) -> u8 = increment;
    const BOXED_DYN: Option<Box<dyn std::fmt::Debug>> = None;
}

#[reflect_impl]
impl OwnedConstShapeContract for OwnedConstShapeTarget {}

#[derive(Reflect)]
struct HrtbConstShapeTarget;

#[reflect]
#[allow(clippy::type_complexity)]
trait HrtbConstShapeContract {
    const BOUND_FUNCTION: for<'a> fn(&'a u8) = observe;
    const ELIDED_FUNCTION: fn(&u8) = observe;
    const BOUND_DYN: Option<Box<dyn for<'a> Fn(&'a u8)>> = None;
    const ELIDED_DYN: Option<Box<dyn Fn(&u8)>> = None;
}

#[reflect_impl]
impl HrtbConstShapeContract for HrtbConstShapeTarget {}

static LIFETIME_CONST_VALUE: u8 = 31;

#[derive(Reflect)]
struct LifetimeConstTarget;

#[reflect]
trait LifetimeConstContract<'a> {
    const VALUE: &'a u8;
}

#[reflect_impl]
impl LifetimeConstContract<'static> for LifetimeConstTarget {
    const VALUE: &'static u8 = &LIFETIME_CONST_VALUE;
}

struct AppliedGatTarget;

#[reflect]
trait AppliedGatContract<T: 'static> {
    type Output<'a, U>
    where
        T: 'a,
        U: 'a;
}

impl AppliedGatContract<u8> for AppliedGatTarget {
    type Output<'a, U>
        = (&'a u8, U)
    where
        U: 'a;
}

/// Returns the unique implementation of `trait_name` for `target`.
fn reflected_implementation<T: Reflect>(
    trait_name: &str,
) -> &'static ImplDescriptor {
    ReflectRegistry::initialize()
        .expect("associated-item fragments must initialize")
        .implementations(TypeDescriptor::of::<T>().type_id())
        .iter()
        .copied()
        .find(|implementation| {
            implementation
                .implemented_trait()
                .is_some_and(|descriptor| descriptor.rust_name() == trait_name)
        })
        .expect("the reflected trait implementation must be registered")
}

#[test]
fn test_reflect_impl_records_associated_type_bindings_in_declaration_order() {
    let implementation =
        reflected_implementation::<AssociatedItemTarget>("AssociatedContract");
    let bindings = implementation.associated_types();

    assert_eq!(bindings.len(), 2);
    assert_eq!(bindings[0].declaration().rust_name(), "Reflected");
    assert_eq!(bindings[1].declaration().rust_name(), "Unproven");
    assert!(matches!(
        bindings[0].value(),
        TypeExpression::Concrete(value) if value.path.last().is_some_and(|segment| segment.as_ref() == "u16")
    ));
    assert!(std::ptr::eq(
        bindings[0]
            .concrete_type()
            .expect("the declared Reflect bound proves a concrete descriptor"),
        TypeDescriptor::of::<u16>(),
    ));
    assert!(matches!(
        bindings[1].value(),
        TypeExpression::Concrete(value) if value.path.last().is_some_and(|segment| segment.as_ref() == "u8")
    ));
    assert!(bindings[1].concrete_type().is_none());
}

#[test]
fn test_reflect_impl_records_associated_const_sources_and_safe_readers() {
    assert_eq!(<AssociatedItemTarget as AssociatedContract>::UNREADABLE, 29,);
    let implementation =
        reflected_implementation::<AssociatedItemTarget>("AssociatedContract");
    let bindings = implementation.associated_consts();

    assert_eq!(bindings.len(), 3);
    assert_eq!(bindings[0].declaration().rust_name(), "DEFAULTED");
    assert_eq!(bindings[1].declaration().rust_name(), "OVERRIDDEN");
    assert_eq!(bindings[2].declaration().rust_name(), "UNREADABLE");
    assert_eq!(
        bindings[0].implementation_source(),
        AssociatedConstImplementationSource::Defaulted,
    );
    assert_eq!(
        bindings[1].implementation_source(),
        AssociatedConstImplementationSource::Overridden,
    );
    assert_eq!(
        bindings[2].implementation_source(),
        AssociatedConstImplementationSource::Overridden,
    );

    let defaulted = bindings[0]
        .read()
        .expect("a concrete default constant must be readable")
        .downcast::<u32>()
        .unwrap_or_else(|_| {
            panic!("the default constant must retain its u32 type")
        });
    assert_eq!(defaulted, 17);
    let overridden = bindings[1]
        .read()
        .expect("a concrete overridden constant must be readable")
        .downcast::<u32>()
        .unwrap_or_else(|_| {
            panic!("the overridden constant must retain its u32 type")
        });
    assert_eq!(overridden, 23);
    assert_eq!(bindings[0].read_unavailable_reason(), None);
    assert_eq!(bindings[1].read_unavailable_reason(), None);
    let associated = bindings[2]
        .read()
        .expect("the concrete associated type binding proves an owned value")
        .downcast::<u8>()
        .unwrap_or_else(|_| {
            panic!("the associated constant must retain its u8 type")
        });
    assert_eq!(associated, 29);
    assert_eq!(bindings[2].read_unavailable_reason(), None);
}

#[test]
fn test_unproven_generic_const_projection_keeps_a_structured_unavailable_reason()
 {
    assert!(
        <GenericAssociatedItemTarget<u8> as SymbolicAssociatedConstContract<
            u8,
        >>::PROJECTED
            .is_empty()
    );
    let implementation = reflected_implementation::<
        GenericAssociatedItemTarget<u8>,
    >("SymbolicAssociatedConstContract");
    let [binding] = implementation.associated_consts() else {
        panic!("the symbolic associated constant binding must be recorded")
    };

    assert!(binding.read().is_none());
    assert_eq!(
        binding.read_unavailable_reason(),
        Some(AssociatedConstReadUnavailableReason::UnprovenOwnedValue),
    );
}

#[test]
fn test_reflect_impl_preserves_unresolved_generic_associated_type_expression() {
    let implementation = reflected_implementation::<
        GenericAssociatedItemTarget<u8>,
    >("SymbolicAssociatedContract");
    let [binding] = implementation.associated_types() else {
        panic!("the symbolic associated type binding must be recorded")
    };
    let TypeExpression::Associated(expression) = binding.value() else {
        panic!("a projection must remain a structured associated expression")
    };
    assert!(
        matches!(
            expression.self_type.as_ref(),
            TypeExpression::Concrete(value) if value.path.last().is_some_and(|segment| segment.as_ref() == "u8")
        ),
        "unexpected projection self type: {:?}",
        expression.self_type
    );
    assert!(binding.concrete_type().is_none());
}

#[test]
fn test_framework_reflect_alias_proves_associated_type_navigation_without_redundant_static()
 {
    let implementation =
        reflected_implementation::<AliasedBoundTarget>("AliasedBoundContract");
    let [binding] = implementation.associated_types() else {
        panic!("the aliased framework bound must produce one binding")
    };
    assert!(std::ptr::eq(
        binding.concrete_type().expect(
            "Reflect already includes the required static lifetime proof"
        ),
        TypeDescriptor::of::<u32>(),
    ));
}

#[test]
fn test_impl_specialization_bound_proves_associated_type_navigation() {
    let implementation =
        reflected_implementation::<ImplBoundTarget<u8>>("ImplBoundContract");
    let [binding] = implementation.associated_types() else {
        panic!("the specialized impl must produce one binding")
    };
    assert!(std::ptr::eq(
        binding
            .concrete_type()
            .expect("the impl's actual framework bound proves navigation"),
        TypeDescriptor::of::<u8>(),
    ));
}

#[test]
fn test_owned_const_reader_uses_rust_type_proof_for_function_and_boxed_dyn_shapes()
 {
    assert_eq!(
        (<OwnedConstShapeTarget as OwnedConstShapeContract>::CALLBACK)(2),
        3,
    );
    assert!(
        <OwnedConstShapeTarget as OwnedConstShapeContract>::BOXED_DYN.is_none()
    );
    let implementation = reflected_implementation::<OwnedConstShapeTarget>(
        "OwnedConstShapeContract",
    );
    let [callback, boxed] = implementation.associated_consts() else {
        panic!("both owned associated constants must be described")
    };
    let callback = callback
        .read()
        .expect("a function pointer is a sized static owned value")
        .downcast::<fn(u8) -> u8>()
        .unwrap_or_else(|_| {
            panic!("the function pointer type must be preserved")
        });
    assert_eq!(callback(4), 5);
    let boxed = boxed
        .read()
        .expect("Option<Box<dyn Debug>> is a sized static owned value")
        .downcast::<Option<Box<dyn std::fmt::Debug>>>()
        .unwrap_or_else(|_| {
            panic!("the boxed trait-object shape must be preserved")
        });
    assert!(boxed.is_none());
}

#[test]
fn test_owned_const_reader_accepts_hrtb_and_elided_callable_lifetimes() {
    let implementation = reflected_implementation::<HrtbConstShapeTarget>(
        "HrtbConstShapeContract",
    );
    let [bound_function, elided_function, bound_dyn, elided_dyn] =
        implementation.associated_consts()
    else {
        panic!("all HRTB associated constants must be recorded")
    };

    let bound = bound_function
        .read()
        .expect("a bound-lifetime function pointer is owned and static")
        .downcast::<for<'a> fn(&'a u8)>()
        .unwrap_or_else(|_| {
            panic!("the HRTB function pointer type must be retained")
        });
    bound(&7);
    let elided = elided_function
        .read()
        .expect("an elided callable lifetime is late-bound")
        .downcast::<fn(&u8)>()
        .unwrap_or_else(|_| {
            panic!("the elided function pointer type must be retained")
        });
    elided(&11);
    assert!(
        bound_dyn
            .read()
            .expect("a boxed HRTB callable is an owned static value")
            .downcast::<Option<Box<dyn for<'a> Fn(&'a u8)>>>()
            .unwrap_or_else(|_| panic!(
                "the boxed HRTB callable type must be retained"
            ))
            .is_none()
    );
    assert!(
        elided_dyn
            .read()
            .expect("an elided boxed callable lifetime is late-bound")
            .downcast::<Option<Box<dyn Fn(&u8)>>>()
            .unwrap_or_else(|_| panic!(
                "the elided boxed callable type must be retained"
            ))
            .is_none()
    );
}

#[test]
fn test_concrete_static_lifetime_application_enables_associated_const_reader() {
    let implementation = reflected_implementation::<LifetimeConstTarget>(
        "LifetimeConstContract",
    );
    let [binding] = implementation.associated_consts() else {
        panic!("the concrete lifetime-associated constant must be recorded")
    };

    let value = binding
        .read()
        .expect("the concrete static lifetime proves an owned reference")
        .downcast::<&'static u8>()
        .unwrap_or_else(|_| {
            panic!("the associated constant must retain its reference type")
        });
    assert_eq!(*value, LIFETIME_CONST_VALUE);
    assert_eq!(binding.read_unavailable_reason(), None);
}

#[test]
fn test_applied_gat_substitutes_outer_arguments_without_rewriting_local_parameters()
 {
    let payload = <AppliedGatTarget as AppliedGatContract<u8>>::__qubit_reflect_trait_payload();
    let generic = payload.applied().associated_types()[0].generic_definition();
    assert!(matches!(
        generic.predicates.as_ref(),
        [
            PredicateDescriptor::TypeOutlives {
                ty: TypeExpression::Concrete(concrete),
                lifetime: LifetimeExpression::Named(outer_lifetime),
                ..
            },
            PredicateDescriptor::TypeOutlives {
                ty: TypeExpression::Parameter(parameter),
                lifetime: LifetimeExpression::Named(local_lifetime),
                ..
            },
        ] if concrete.path.last().is_some_and(|segment| segment.as_ref() == "u8")
            && outer_lifetime.as_ref() == "a"
            && parameter.as_ref() == "U"
            && local_lifetime.as_ref() == "a"
    ));
}
