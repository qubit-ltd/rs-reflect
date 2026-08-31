// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow explicit-imports
//! Integration tests for reflected trait declarations.
use std::any::TypeId;

use qubit_reflect as reflect;
use reflect::descriptor::TraitId;
use reflect::descriptor::TypeDescriptor;
use reflect::reflect;
use reflect::registry::ReflectRegistry;

#[reflect]
trait ReflectedService {
    type Output: Send;
    const LIMIT: usize = 8;

    fn required(&self) -> Self::Output;

    fn defaulted(&self) -> usize {
        Self::LIMIT
    }
}

#[reflect(supertrait(ReflectedService), external_trait(Send, id = "core.marker.Send"))]
trait Worker: ReflectedService + Send {
    fn work(&self);
}

#[reflect]
trait DynService {
    fn value(&self) -> usize;
}

#[reflect(rename = "renamed_dyn_service")]
trait RenamedDynService {
    fn renamed_value(&self) -> usize;
}

#[reflect(external_trait(::std::fmt::Debug, id = "core.fmt.Debug"))]
trait DebugDynService: ::std::fmt::Debug {
    fn debug_value(&self) -> usize;
}

#[reflect]
trait NonDynService {
    const VALUE: usize;

    fn generic<T>(&self, value: T) -> T;
}

#[reflect]
trait GenericDynService<T, const N: usize> {
    fn transform(&self, value: T) -> T;

    fn repeat(&self, value: T) -> [T; N];

    fn limit(&self) -> usize {
        N
    }
}

#[reflect]
trait LifetimeDynService<'a> {
    fn borrowed(&self, value: &'a str) -> &'a str;
}

#[reflect]
trait AssociatedDynService
where
    Self::Item: std::fmt::Debug,
{
    type Item;

    fn item(&self) -> Self::Item;
}

#[reflect]
trait ReceiverDynService {
    fn consume(self);

    fn boxed(self: Box<Self>);

    fn pinned(self: std::pin::Pin<Box<Self>>);
}

#[reflect]
trait LocalDynParent {
    fn parent(&self) -> usize;
}

#[reflect(supertrait(LocalDynParent), dyn_compatible)]
trait LocalDynChild: LocalDynParent {
    fn child(&self) -> usize;
}

#[reflect(supertrait(LocalDynChild), dyn_compatible)]
trait LocalDynGrandchild: LocalDynChild {
    fn grandchild(&self) -> usize;
}

#[reflect]
trait GenericService<T> {
    fn transform(&self, value: T) -> T;

    fn generic<'a, 'b: 'a, U, const N: usize>(&self, value: &'a U) -> &'a U
    where
        U: Send + 'a;
}

#[reflect]
trait ConstService<const N: usize> {
    fn limit(&self) -> usize {
        N
    }
}

#[reflect]
trait LifetimeAndLiteralDefaultService<
    'b,
    'a: 'b,
    const SIGNED: i16 = -7i16,
    const UNSIGNED: u8 = 42u8,
    const ESCAPED: char = '\n',
>
{
    fn borrowed(&'a self) -> &'b str;
}

#[reflect]
pub(crate) trait StructuralService {
    fn structural(&self, value: Vec<[u8; 4]>) -> fn(*const u8) -> usize;
}

#[reflect]
trait MaybeService<T: ?Sized> {}

#[reflect]
trait HrtbService<T>
where
    for<'a> T: Fn(&'a str),
{
}

#[reflect]
trait WhereBoundService<T>
where
    for<'a> T: Fn(&'a str) + Send,
{
}

#[reflect]
trait WhereLifetimeService<T>
where
    T: Send + 'static,
{
}

#[reflect]
trait AssocHrtb {
    type Output: for<'a> Fn(&'a str);
}

struct ServiceMarkerProbe;

#[derive(Debug)]
struct UnreflectedDynArgument;

impl ReflectedService for ServiceMarkerProbe {
    type Output = usize;

    fn required(&self) -> Self::Output {
        Self::LIMIT
    }
}

impl Worker for ServiceMarkerProbe {
    fn work(&self) {}
}

impl DynService for ServiceMarkerProbe {
    fn value(&self) -> usize {
        8
    }
}

impl RenamedDynService for ServiceMarkerProbe {
    fn renamed_value(&self) -> usize {
        13
    }
}

impl std::fmt::Debug for ServiceMarkerProbe {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ServiceMarkerProbe")
    }
}

impl DebugDynService for ServiceMarkerProbe {
    fn debug_value(&self) -> usize {
        21
    }
}

impl NonDynService for ServiceMarkerProbe {
    const VALUE: usize = 34;

    fn generic<T>(&self, value: T) -> T {
        value
    }
}

impl GenericDynService<u8, 2> for ServiceMarkerProbe {
    fn transform(&self, value: u8) -> u8 {
        value
    }

    fn repeat(&self, value: u8) -> [u8; 2] {
        [value; 2]
    }
}

impl LifetimeDynService<'static> for ServiceMarkerProbe {
    fn borrowed(&self, value: &'static str) -> &'static str {
        value
    }
}

impl AssociatedDynService for ServiceMarkerProbe {
    type Item = u8;

    fn item(&self) -> Self::Item {
        55
    }
}

impl ReceiverDynService for ServiceMarkerProbe {
    fn consume(self) {}

    fn boxed(self: Box<Self>) {
        drop(self);
    }

    fn pinned(self: std::pin::Pin<Box<Self>>) {
        drop(self);
    }
}

impl LocalDynParent for ServiceMarkerProbe {
    fn parent(&self) -> usize {
        89
    }
}

impl LocalDynChild for ServiceMarkerProbe {
    fn child(&self) -> usize {
        144
    }
}

impl LocalDynGrandchild for ServiceMarkerProbe {
    fn grandchild(&self) -> usize {
        233
    }
}

impl GenericService<usize> for ServiceMarkerProbe {
    fn transform(&self, value: usize) -> usize {
        value
    }

    fn generic<'a, 'b: 'a, U, const N: usize>(&self, value: &'a U) -> &'a U
    where
        U: Send + 'a,
    {
        value
    }
}

impl<const N: usize> ConstService<N> for ServiceMarkerProbe {}
impl<'b, 'a: 'b, const SIGNED: i16, const UNSIGNED: u8, const ESCAPED: char>
    LifetimeAndLiteralDefaultService<'b, 'a, SIGNED, UNSIGNED, ESCAPED> for ServiceMarkerProbe
{
    fn borrowed(&'a self) -> &'b str {
        ""
    }
}
impl MaybeService<str> for ServiceMarkerProbe {}
impl HrtbService<for<'a> fn(&'a str)> for ServiceMarkerProbe {}
impl WhereBoundService<for<'a> fn(&'a str)> for ServiceMarkerProbe {}
impl WhereLifetimeService<usize> for ServiceMarkerProbe {}
impl StructuralService for ServiceMarkerProbe {
    fn structural(&self, value: Vec<[u8; 4]>) -> fn(*const u8) -> usize {
        drop(value);
        |_| 0
    }
}
impl AssocHrtb for ServiceMarkerProbe {
    type Output = for<'a> fn(&'a str);
}

#[test]
fn test_reflect_trait_registers_complete_marker_backed_definition() {
    let registry = ReflectRegistry::initialize().expect("trait fragments must initialize");
    let definition = registry
        .trait_definition_by_path("integration_tests::descriptor::reflect_trait_tests::ReflectedService")
        .expect("reflected trait must be registered");

    assert_eq!(definition.rust_name(), "ReflectedService");
    assert!(matches!(definition.trait_id(), TraitId::Reflected(_)));
    assert_eq!(definition.generic_definition().parameters.len(), 0);
    let candidates =
        registry.find_trait_definitions_by_path("integration_tests::descriptor::reflect_trait_tests::ReflectedService");
    assert_eq!(candidates.len(), 1);
    assert!(std::ptr::eq(
        candidates.only().expect("a unique path must have one candidate"),
        definition
    ));
    assert!(
        registry
            .find_trait_definitions_by_path("integration_tests::missing::Trait")
            .is_empty()
    );
}

#[test]
fn test_reflect_trait_preserves_dyn_compatibility_and_default_semantics() {
    let service: &dyn DynService = &ServiceMarkerProbe;
    let renamed: &dyn RenamedDynService = &ServiceMarkerProbe;
    let debug: &dyn DebugDynService = &ServiceMarkerProbe;

    assert_eq!(service.value(), 8);
    assert_eq!(renamed.renamed_value(), 13);
    assert_eq!(debug.debug_value(), 21);
    assert_eq!(NonDynService::generic(&ServiceMarkerProbe, 34), 34);
    assert_eq!(<ServiceMarkerProbe as NonDynService>::VALUE, 34);
    assert_eq!(GenericDynService::<u8, 2>::transform(&ServiceMarkerProbe, 5), 5);
    assert_eq!(GenericDynService::<u8, 2>::repeat(&ServiceMarkerProbe, 5), [5; 2]);
    assert_eq!(GenericDynService::<u8, 2>::limit(&ServiceMarkerProbe), 2);
    assert_eq!(
        LifetimeDynService::borrowed(&ServiceMarkerProbe, "borrowed"),
        "borrowed"
    );
    assert_eq!(AssociatedDynService::item(&ServiceMarkerProbe), 55);
    assert_eq!(LocalDynParent::parent(&ServiceMarkerProbe), 89);
    assert_eq!(LocalDynChild::child(&ServiceMarkerProbe), 144);
    assert_eq!(LocalDynGrandchild::grandchild(&ServiceMarkerProbe), 233);
    ReceiverDynService::consume(ServiceMarkerProbe);
    ReceiverDynService::boxed(Box::new(ServiceMarkerProbe));
    ReceiverDynService::pinned(Box::pin(ServiceMarkerProbe));
    assert_eq!(ReflectedService::defaulted(&ServiceMarkerProbe), 8);
    assert_eq!(ReflectedService::required(&ServiceMarkerProbe), 8);
    Worker::work(&ServiceMarkerProbe);
    assert_eq!(GenericService::transform(&ServiceMarkerProbe, 8), 8);
    assert_eq!(
        GenericService::generic::<_, 1>(&ServiceMarkerProbe, &"generic"),
        &"generic"
    );
    let _ = TypeId::of::<ServiceMarkerProbe>();
}

#[test]
fn test_dyn_compatible_trait_object_navigates_to_applied_trait_descriptor() {
    let descriptor = TypeDescriptor::of::<dyn DynService>();
    let repeated = TypeDescriptor::of::<dyn DynService>();
    let trait_object = descriptor
        .as_trait_object()
        .expect("a reflected dyn-compatible trait must expose a trait-object view");
    let applied = trait_object.trait_descriptor();
    let registry = ReflectRegistry::initialize().expect("trait fragments must initialize");
    let definition = registry
        .trait_definition_by_path("integration_tests::descriptor::reflect_trait_tests::DynService")
        .expect("the reflected trait definition must be registered");

    assert!(std::ptr::eq(descriptor, repeated));
    assert!(std::ptr::eq(applied, trait_object.trait_descriptor()));
    assert!(std::ptr::eq(applied.definition(), definition));
    assert_eq!(descriptor.type_id(), TypeId::of::<dyn DynService>());
    assert_eq!(descriptor.query_name(), "DynService");
    assert_eq!(applied.methods()[0].rust_name(), "value");

    let non_dyn_definition = registry
        .trait_definition_by_path("integration_tests::descriptor::reflect_trait_tests::NonDynService")
        .expect("a non-dyn-compatible reflected trait still has a definition descriptor");
    assert!(matches!(non_dyn_definition.trait_id(), TraitId::Reflected(_)));
}

#[test]
fn test_dyn_trait_object_preserves_rename_and_provable_supertrait_navigation() {
    let renamed = TypeDescriptor::of::<dyn RenamedDynService>();
    assert_eq!(renamed.query_name(), "renamed_dyn_service");
    assert_eq!(
        renamed
            .as_trait_object()
            .expect("renamed dyn trait must expose its typed view")
            .trait_descriptor()
            .query_name(),
        "renamed_dyn_service"
    );

    let with_supertrait = TypeDescriptor::of::<dyn DebugDynService>();
    let applied = with_supertrait
        .as_trait_object()
        .expect("dyn trait with a known-compatible supertrait must have a descriptor")
        .trait_descriptor();
    assert_eq!(applied.direct_supertraits().len(), 1);
    assert_eq!(applied.direct_supertraits()[0].rust_name(), "Debug");
}

#[test]
fn test_generic_and_associated_dyn_applications_substitute_concrete_signatures() {
    let generic = TypeDescriptor::of::<dyn GenericDynService<u8, 2>>();
    let applied = generic
        .as_trait_object()
        .expect("generic dyn application must expose a trait-object view")
        .trait_descriptor();
    assert_eq!(applied.arguments().len(), 2);
    assert!(matches!(
        applied.methods()[0].parameters()[0].signature_type(),
        reflect::expression::TypeExpression::Concrete(value)
            if value.path[0].as_ref() == std::any::type_name::<u8>()
    ));
    let output = applied.methods()[0]
        .return_value()
        .signature_type()
        .expect("generic output signature must be retained");
    assert!(matches!(
        output,
        reflect::expression::TypeExpression::Concrete(value)
            if value.path[0].as_ref() == std::any::type_name::<u8>()
    ));
    assert!(matches!(
        applied.methods()[1]
            .return_value()
            .signature_type()
            .expect("const-generic output signature must be retained"),
        reflect::expression::TypeExpression::Array(value)
            if matches!(value.length, reflect::expression::ConstExpression::UnsignedInteger(2))
    ));

    let lifetime = TypeDescriptor::of::<dyn LifetimeDynService<'static>>();
    assert_eq!(
        lifetime
            .as_trait_object()
            .expect("concrete lifetime application must be reflected")
            .trait_descriptor()
            .rust_name(),
        "LifetimeDynService"
    );

    let associated = TypeDescriptor::of::<dyn AssociatedDynService<Item = u8>>();
    let applied = associated
        .as_trait_object()
        .expect("associated-type dyn application must be reflected")
        .trait_descriptor();
    assert_eq!(applied.associated_type_arguments().len(), 1);
    assert!(matches!(
        applied.methods()[0]
            .return_value()
            .signature_type()
            .expect("associated return must be concrete"),
        reflect::expression::TypeExpression::Concrete(value)
            if value.path[0].as_ref() == std::any::type_name::<u8>()
    ));

    assert_eq!(
        TypeDescriptor::of::<dyn GenericDynService<UnreflectedDynArgument, 2>>()
            .as_trait_object()
            .expect("dyn generic arguments need not implement Reflect")
            .trait_descriptor()
            .arguments()
            .len(),
        2
    );
    assert_eq!(
        TypeDescriptor::of::<dyn AssociatedDynService<Item = UnreflectedDynArgument>>()
            .as_trait_object()
            .expect("dyn associated bindings need not implement Reflect")
            .trait_descriptor()
            .associated_type_arguments()
            .len(),
        1
    );
}

#[test]
fn test_dyn_receivers_and_reflected_supertraits_preserve_navigation() {
    let receivers = TypeDescriptor::of::<dyn ReceiverDynService>();
    assert_eq!(
        receivers
            .as_trait_object()
            .expect("supported dyn receivers must retain a descriptor")
            .trait_descriptor()
            .methods()
            .len(),
        3
    );

    let child = TypeDescriptor::of::<dyn LocalDynChild>();
    let child = child
        .as_trait_object()
        .expect("local reflected child trait must expose a descriptor")
        .trait_descriptor();
    assert_eq!(child.direct_supertraits().len(), 1);
    assert_eq!(child.direct_supertraits()[0].rust_name(), "LocalDynParent");
    assert_eq!(child.all_supertraits().len(), 1);

    let grandchild = TypeDescriptor::of::<dyn LocalDynGrandchild>();
    let grandchild = grandchild
        .as_trait_object()
        .expect("three-level reflected child must expose a descriptor")
        .trait_descriptor();
    assert_eq!(grandchild.direct_supertraits().len(), 1);
    assert_eq!(grandchild.direct_supertraits()[0].rust_name(), "LocalDynChild");
    assert_eq!(grandchild.all_supertraits().len(), 2);
    assert_eq!(
        grandchild
            .all_supertraits()
            .iter()
            .map(|descriptor| descriptor.rust_name())
            .collect::<Vec<_>>(),
        ["LocalDynChild", "LocalDynParent"]
    );
}

#[test]
fn test_reflect_trait_accepts_explicit_external_supertrait_mapping() {
    let registry = ReflectRegistry::initialize().expect("trait fragments must initialize");
    let definition = registry
        .trait_definition_by_path("integration_tests::descriptor::reflect_trait_tests::Worker")
        .expect("mapped trait must be registered");

    assert_eq!(definition.rust_name(), "Worker");
    assert!(matches!(definition.trait_id(), TraitId::Reflected(_)));
    let payload = <ServiceMarkerProbe as Worker>::__qubit_reflect_trait_payload();
    let repeated = <ServiceMarkerProbe as Worker>::__qubit_reflect_trait_payload();
    assert!(std::ptr::eq(payload.applied(), repeated.applied()));
    assert_eq!(payload.applied().methods().len(), 1);
    assert_eq!(payload.applied().methods()[0].rust_name(), "work");
    assert_eq!(
        payload.applied().methods()[0].receiver(),
        Some(&reflect::descriptor::ReceiverDescriptor::Shared)
    );
    let direct_paths: Vec<_> = payload
        .applied()
        .direct_supertraits()
        .iter()
        .map(|descriptor| descriptor.rust_name())
        .collect();
    assert_eq!(direct_paths, ["ReflectedService", "Send"]);
}

#[test]
fn test_reflect_trait_registers_generic_definition_without_dyn_descriptor() {
    fn assert_hrtb<T: HrtbService<for<'a> fn(&'a str)>>() {}
    fn assert_where_bound<T: WhereBoundService<for<'a> fn(&'a str)>>() {}
    fn assert_lifetime_bound<T: WhereLifetimeService<usize>>() {}
    fn assert_lifetime_parameter_bound<T: LifetimeAndLiteralDefaultService<'static, 'static>>() {}
    let registry = ReflectRegistry::initialize().expect("trait fragments must initialize");
    let definition = registry
        .trait_definition_by_path("integration_tests::descriptor::reflect_trait_tests::GenericService")
        .expect("generic trait must be registered");

    assert_eq!(definition.generic_definition().parameters.len(), 1);
    let payload = <ServiceMarkerProbe as GenericService<usize>>::__qubit_reflect_trait_payload();
    assert_eq!(payload.applied().arguments().len(), 1);
    let generic_method = &payload.applied().methods()[1];
    assert_eq!(generic_method.rust_name(), "generic");
    assert_eq!(generic_method.generic_definition().parameters.len(), 4);
    assert_eq!(generic_method.generic_definition().predicates.len(), 2);
    let reflect::expression::GenericParameterDescriptor::Lifetime { bounds, .. } =
        &generic_method.generic_definition().parameters[1]
    else {
        panic!("generic method must retain its lifetime parameter")
    };
    assert_eq!(bounds.len(), 1);
    let reflect::expression::GenericParameterDescriptor::Const { ty, .. } =
        &generic_method.generic_definition().parameters[3]
    else {
        panic!("generic method must retain its const parameter")
    };
    assert!(
        matches!(ty.as_ref(), reflect::expression::TypeExpression::Concrete(value) if value.path[0].as_ref() == "usize")
    );
    assert!(matches!(
        generic_method.parameters()[0].signature_type(),
        reflect::expression::TypeExpression::Reference(value)
            if matches!(value.target.as_ref(), reflect::expression::TypeExpression::Parameter(name) if name.as_ref() == "U")
    ));
    let service = <ServiceMarkerProbe as ReflectedService>::__qubit_reflect_trait_payload();
    assert_eq!(service.applied().methods().len(), 2);
    assert_eq!(
        service.applied().methods()[0].receiver(),
        Some(&reflect::descriptor::ReceiverDescriptor::Shared)
    );
    assert!(!service.applied().methods()[0].has_default());
    assert!(service.applied().methods()[1].has_default());
    assert_eq!(service.applied().associated_types()[0].rust_name(), "Output");
    assert_eq!(service.applied().associated_types()[0].bounds().len(), 1);
    assert_eq!(service.applied().associated_consts()[0].rust_name(), "LIMIT");
    assert!(service.applied().associated_consts()[0].has_default());
    let const_payload = <ServiceMarkerProbe as ConstService<8>>::__qubit_reflect_trait_payload();
    assert_eq!(const_payload.applied().arguments().len(), 1);
    assert_eq!(ConstService::<8>::limit(&ServiceMarkerProbe), 8);
    let literal_defaults = registry
        .trait_definition_by_path(
            "integration_tests::descriptor::reflect_trait_tests::LifetimeAndLiteralDefaultService",
        )
        .expect("literal-default trait must register");
    let parameters = literal_defaults.generic_definition().parameters.as_ref();
    let reflect::expression::GenericParameterDescriptor::Lifetime { bounds, .. } = &parameters[1] else {
        panic!("expected a lifetime parameter")
    };
    assert_eq!(
        bounds.as_ref(),
        [reflect::expression::LifetimeExpression::Named("b".into())]
    );
    let reflect::expression::GenericParameterDescriptor::Const { default, .. } = &parameters[2] else {
        panic!("expected the signed const parameter")
    };
    assert_eq!(default, &Some(reflect::expression::ConstExpression::SignedInteger(-7)));
    let reflect::expression::GenericParameterDescriptor::Const { default, .. } = &parameters[3] else {
        panic!("expected the unsigned const parameter")
    };
    assert_eq!(
        default,
        &Some(reflect::expression::ConstExpression::UnsignedInteger(42))
    );
    let reflect::expression::GenericParameterDescriptor::Const { default, .. } = &parameters[4] else {
        panic!("expected the character const parameter")
    };
    assert_eq!(default, &Some(reflect::expression::ConstExpression::Character('\n')));
    let maybe = <ServiceMarkerProbe as MaybeService<str>>::__qubit_reflect_trait_payload();
    assert_eq!(maybe.applied().arguments().len(), 1);
    let definition = maybe.definition();
    let reflect::expression::GenericParameterDescriptor::Type { bounds, .. } =
        &definition.generic_definition().parameters[0]
    else {
        panic!("expected type parameter")
    };
    let reflect::expression::PredicateDescriptor::TypeBound { bound_modifiers, .. } = &bounds[0] else {
        panic!("expected type bound")
    };
    assert_eq!(
        bound_modifiers.as_ref(),
        [reflect::expression::TraitBoundModifier::Maybe]
    );
    let registry = ReflectRegistry::initialize().expect("registry must initialize");
    let hrtb = registry
        .trait_definition_by_path("integration_tests::descriptor::reflect_trait_tests::HrtbService")
        .expect("HRTB trait must register");
    assert_eq!(hrtb.generic_definition().predicates.len(), 1);
    let where_bound = registry
        .trait_definition_by_path("integration_tests::descriptor::reflect_trait_tests::WhereBoundService")
        .expect("where-bound trait must register");
    let predicates = where_bound.generic_definition().predicates.as_ref();
    assert_eq!(predicates.len(), 1);
    let reflect::expression::PredicateDescriptor::TypeBound {
        bounds,
        bound_modifiers,
        higher_ranked_lifetimes,
        ..
    } = &predicates[0]
    else {
        panic!("where trait bounds must remain structural")
    };
    assert_eq!(bounds.len(), 2);
    assert_eq!(
        bound_modifiers.as_ref(),
        [
            reflect::expression::TraitBoundModifier::None,
            reflect::expression::TraitBoundModifier::None,
        ]
    );
    assert_eq!(higher_ranked_lifetimes.len(), 1);
    let lifetime_bound = registry
        .trait_definition_by_path("integration_tests::descriptor::reflect_trait_tests::WhereLifetimeService")
        .expect("lifetime-bound trait must register");
    assert!(matches!(
        lifetime_bound.generic_definition().predicates.as_ref(),
        [
            reflect::expression::PredicateDescriptor::TypeBound { .. },
            reflect::expression::PredicateDescriptor::TypeOutlives {
                lifetime: reflect::expression::LifetimeExpression::Static,
                ..
            },
        ]
    ));
    assert_hrtb::<ServiceMarkerProbe>();
    assert_where_bound::<ServiceMarkerProbe>();
    assert_lifetime_bound::<ServiceMarkerProbe>();
    let structural = <ServiceMarkerProbe as StructuralService>::__qubit_reflect_trait_payload();
    assert_eq!(
        structural.definition().visibility(),
        &reflect::identity::Visibility::Crate
    );
    let structural_call = StructuralService::structural(&ServiceMarkerProbe, Vec::new());
    assert_eq!(structural_call(std::ptr::null()), 0);
    let parameter = &structural.applied().methods()[0].parameters()[0];
    let reflect::expression::TypeExpression::Concrete(vector) = parameter.signature_type() else {
        panic!("Vec parameter must remain a structured concrete path")
    };
    assert_eq!(vector.path[0].as_ref(), "Vec");
    let reflect::expression::GenericArgument::Type(reflect::expression::TypeExpression::Array(array)) =
        &vector.arguments[0]
    else {
        panic!("Vec element must preserve its array argument")
    };
    assert!(matches!(
        array.length,
        reflect::expression::ConstExpression::UnsignedInteger(4)
    ));
    let return_value = structural.applied().methods()[0]
        .return_value()
        .signature_type()
        .expect("structural method must retain its return type");
    assert!(matches!(
        return_value,
        reflect::expression::TypeExpression::FunctionPointer(_)
    ));
    assert_lifetime_parameter_bound::<ServiceMarkerProbe>();
    assert_eq!(
        <ServiceMarkerProbe as LifetimeAndLiteralDefaultService<'_, '_, -7i16, 42u8, '\n'>>::borrowed(
            &ServiceMarkerProbe,
        ),
        ""
    );
    let assoc = <ServiceMarkerProbe as AssocHrtb>::__qubit_reflect_trait_payload();
    assert_eq!(assoc.applied().associated_types()[0].bounds().len(), 1);
    let reflect::expression::PredicateDescriptor::TypeBound {
        bound_modifiers,
        higher_ranked_lifetimes,
        ..
    } = &assoc.applied().associated_types()[0].bounds()[0]
    else {
        panic!("expected associated type bound")
    };
    assert_eq!(
        bound_modifiers.as_ref(),
        [reflect::expression::TraitBoundModifier::None]
    );
    assert_eq!(higher_ranked_lifetimes.len(), 1);
}
