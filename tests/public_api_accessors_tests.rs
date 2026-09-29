// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Public accessor and query contracts exercised through the external crate
//! API.

use std::any::TypeId;

use qubit_reflect::access::FieldAccessError;
use qubit_reflect::access::FieldAccessOperation;
use qubit_reflect::access::FieldIdentity;
use qubit_reflect::descriptor::AssociatedConstReader;
use qubit_reflect::descriptor::ConcreteGenericDescriptor;
#[cfg(feature = "derive")]
use qubit_reflect::descriptor::ImplDescriptor;
#[cfg(feature = "derive")]
use qubit_reflect::descriptor::InvocationUnavailableReason;
#[cfg(feature = "derive")]
use qubit_reflect::descriptor::MethodLookup;
#[cfg(feature = "derive")]
use qubit_reflect::descriptor::MethodQualifier;
#[cfg(feature = "derive")]
use qubit_reflect::descriptor::ParameterPassingMode;
#[cfg(feature = "derive")]
use qubit_reflect::descriptor::ReturnKind;
use qubit_reflect::error::RegistryError;
use qubit_reflect::error::TypeMismatch;
use qubit_reflect::expression::ArrayTypeExpression;
use qubit_reflect::expression::AssociatedTypeExpression;
use qubit_reflect::expression::ConcreteTypeExpression;
use qubit_reflect::expression::ConstExpression;
use qubit_reflect::expression::DiagnosticText;
use qubit_reflect::expression::FunctionAbi;
use qubit_reflect::expression::FunctionPointerExpression;
use qubit_reflect::expression::FunctionSafety;
use qubit_reflect::expression::GenericArgument;
use qubit_reflect::expression::GenericDefinitionDescriptor;
use qubit_reflect::expression::GenericParameterDescriptor;
use qubit_reflect::expression::LifetimeExpression;
use qubit_reflect::expression::OpaqueTypeExpression;
use qubit_reflect::expression::PredicateDescriptor;
use qubit_reflect::expression::RawPointerTypeExpression;
use qubit_reflect::expression::ReferenceTypeExpression;
use qubit_reflect::expression::TraitBoundModifier;
use qubit_reflect::expression::TraitObjectExpression;
use qubit_reflect::expression::TypeExpression;
use qubit_reflect::identity::FragmentIdentity;
use qubit_reflect::identity::MemberId;
use qubit_reflect::identity::Visibility;
use qubit_reflect::identity::VisibilityKind;
use qubit_reflect::invoke::Invocation;
use qubit_reflect::invoke::InvocationArg;
use qubit_reflect::invoke::InvocationBinding;
use qubit_reflect::invoke::InvocationPanic;
use qubit_reflect::invoke::InvocationReceiver;
use qubit_reflect::invoke::ReceiverExpectation;
use qubit_reflect::registry::ReflectRegistry;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;
use qubit_reflect::value::ReflectedOwned;

fn fragment(fingerprint: u64) -> FragmentIdentity {
    FragmentIdentity::new("crate", "crate::module", 10, 4, "field", fingerprint)
}

#[test]
fn test_small_identity_and_error_accessors_preserve_input_facts() {
    let identity = fragment(17);
    assert_eq!(identity.declaring_crate(), "crate");
    assert_eq!(identity.module_path(), "crate::module");
    assert_eq!(identity.line(), 10);
    assert_eq!(identity.column(), 4);
    assert_eq!(identity.member_kind(), "field");
    assert_eq!(identity.content_fingerprint(), 17);
    let member = MemberId::new("Type", "field", 1, identity.clone());
    assert_eq!(member.fragment(), &identity);

    let visibility = Visibility::from_source("pub(in crate::model)");
    assert_eq!(visibility.kind(), VisibilityKind::Restricted);
    assert_eq!(visibility.restricted_path(), Some("crate::model"));

    let direct = FieldIdentity::new(TypeId::of::<u8>(), "u8", 0, Some("value"));
    assert_eq!(direct.rust_name(), Some("value"));
    let variant = FieldIdentity::new_variant(TypeId::of::<u8>(), "u8", 0, None, 2, "Ready");
    assert_eq!(variant.variant_rust_name(), Some("Ready"));

    let mismatch = TypeMismatch::new(TypeId::of::<u8>(), TypeId::of::<u16>()).with_diagnostic_names("u8", "u16");
    assert_eq!(mismatch.expected(), TypeId::of::<u8>());
    assert_eq!(mismatch.actual(), TypeId::of::<u16>());
    assert_eq!(mismatch.expected_name(), Some("u8"));
    assert_eq!(mismatch.actual_name(), Some("u16"));

    let conflict = RegistryError::duplicate_fragment(fragment(1), fragment(2));
    assert!(conflict.conflicting_fragments().is_some());
}

#[test]
fn test_field_error_accessors_preserve_public_identity_facts() {
    let direct = FieldIdentity::new(TypeId::of::<u16>(), "u16", 3, Some("value"));
    assert_eq!(direct.declaring_type(), TypeId::of::<u16>());
    assert_eq!(direct.declaring_type_name(), "u16");
    assert_eq!(direct.index(), 3);
    assert_eq!(direct.variant_index(), None);
    assert_eq!(direct.to_string(), "u16::value");
    assert_eq!(FieldAccessOperation::Set.to_string(), "set");

    let variant = FieldIdentity::new_variant(TypeId::of::<u16>(), "u16", 3, None, 2, "Ready");
    assert_eq!(variant.variant_index(), Some(2));
    assert_eq!(variant.to_string(), "u16::Ready field #3");
    let inactive = FieldAccessError::inactive_variant(variant.clone());
    assert_eq!(inactive.field(), &variant);
}

#[test]
fn test_expression_constructors_preserve_navigable_structural_facts() {
    let concrete = ConcreteTypeExpression::new(
        ["core", "option", "Option"],
        [GenericArgument::Type(TypeExpression::SelfType)],
    )
    .expect("the concrete path is non-empty")
    .with_diagnostic("Option<Self>");
    assert_eq!(
        concrete.path().iter().map(AsRef::as_ref).collect::<Vec<_>>(),
        ["core", "option", "Option"]
    );
    assert_eq!(concrete.arguments().len(), 1);
    assert_eq!(concrete.diagnostic(), Some("Option<Self>"));

    let associated = AssociatedTypeExpression::new(
        TypeExpression::Parameter("T".into()),
        Some(TypeExpression::Parameter("Iterator".into())),
        "Item",
        Box::<[GenericArgument]>::default(),
    )
    .with_diagnostic("<T as Iterator>::Item");
    assert_eq!(associated.self_type(), &TypeExpression::Parameter("T".into()));
    assert_eq!(
        associated.trait_path(),
        Some(&TypeExpression::Parameter("Iterator".into()))
    );
    assert_eq!(associated.item(), "Item");
    assert!(associated.arguments().is_empty());
    assert_eq!(associated.diagnostic(), Some("<T as Iterator>::Item"));

    let reference = ReferenceTypeExpression::new(LifetimeExpression::Named("a".into()), true, TypeExpression::SelfType)
        .with_diagnostic("&'a mut Self");
    assert_eq!(reference.lifetime(), &LifetimeExpression::Named("a".into()));
    assert!(reference.is_mutable());
    assert_eq!(reference.target(), &TypeExpression::SelfType);
    assert_eq!(reference.diagnostic(), Some("&'a mut Self"));

    let raw = RawPointerTypeExpression::new(false, TypeExpression::SelfType).with_diagnostic("*const Self");
    assert!(!raw.is_mutable());
    assert_eq!(raw.target(), &TypeExpression::SelfType);
    assert_eq!(raw.diagnostic(), Some("*const Self"));

    let array = ArrayTypeExpression::new(TypeExpression::SelfType, ConstExpression::UnsignedInteger(3))
        .with_diagnostic("[Self; 3]");
    assert_eq!(array.element(), &TypeExpression::SelfType);
    assert_eq!(array.length(), &ConstExpression::UnsignedInteger(3));
    assert_eq!(array.diagnostic(), Some("[Self; 3]"));

    let function = FunctionPointerExpression::new(
        FunctionAbi::C,
        FunctionSafety::Unsafe,
        true,
        [LifetimeExpression::Named("a".into())],
        [TypeExpression::SelfType],
        TypeExpression::Never,
    )
    .with_diagnostic("unsafe extern C fn(Self, ...) -> !");
    assert_eq!(function.abi(), &FunctionAbi::C);
    assert_eq!(function.safety(), &FunctionSafety::Unsafe);
    assert!(function.is_variadic());
    assert_eq!(
        function.higher_ranked_lifetimes(),
        &[LifetimeExpression::Named("a".into())]
    );
    assert_eq!(function.parameters(), &[TypeExpression::SelfType]);
    assert_eq!(function.return_type(), &TypeExpression::Never);
    assert_eq!(function.diagnostic(), Some("unsafe extern C fn(Self, ...) -> !"));

    let predicate = PredicateDescriptor::type_bound(
        TypeExpression::Parameter("T".into()),
        [TypeExpression::Parameter("Display".into())],
        [TraitBoundModifier::None],
        Box::<[LifetimeExpression]>::default(),
    )
    .expect("one modifier is supplied for the non-empty bound")
    .with_diagnostic("T: Display");
    assert_eq!(predicate.diagnostic(), Some("T: Display"));
    let outlives =
        PredicateDescriptor::lifetime_outlives(LifetimeExpression::Named("a".into()), [LifetimeExpression::Static])
            .expect("the lifetime bound is non-empty");

    let trait_object = TraitObjectExpression::new([predicate.clone()]).with_diagnostic("dyn Display");
    assert_eq!(trait_object.bounds(), std::slice::from_ref(&predicate));
    assert_eq!(trait_object.diagnostic(), Some("dyn Display"));
    let opaque = OpaqueTypeExpression::new([predicate.clone()]).with_diagnostic("impl Display");
    assert_eq!(opaque.bounds(), std::slice::from_ref(&predicate));
    assert_eq!(opaque.diagnostic(), Some("impl Display"));

    let definition = GenericDefinitionDescriptor::new(
        [GenericParameterDescriptor::Lifetime {
            name: "a".into(),
            bounds: vec![LifetimeExpression::Static].into_boxed_slice(),
            diagnostic: DiagnosticText::default(),
        }],
        [outlives],
    )
    .with_diagnostic("<'a> where 'a: 'static");
    assert_eq!(definition.parameters().len(), 1);
    assert_eq!(definition.predicates().len(), 1);
    assert_eq!(definition.diagnostic(), Some("<'a> where 'a: 'static"));
}

#[test]
fn test_small_generic_and_invocation_accessors_preserve_input_facts() {
    let diagnostic = DiagnosticText::from(String::from("source"));
    assert_eq!(diagnostic.as_deref(), Some("source"));

    let definition = Box::leak(Box::new(GenericDefinitionDescriptor::new(
        Box::<[GenericParameterDescriptor]>::default(),
        Box::default(),
    )));
    let generic = ConcreteGenericDescriptor::new(definition, &[]);
    assert!(std::ptr::eq(generic.definition(), definition));
    assert!(generic.arguments().is_empty());

    let binding = InvocationBinding::<Local>::named("value", InvocationArg::Owned(DynamicOwned::<Local>::new(3_u8)));
    assert_eq!(binding.name(), Some("value"));
    assert!(matches!(binding.argument(), InvocationArg::Owned(_)));

    let reader = AssociatedConstReader::from_getter(|| 7_u8);
    assert_eq!(reader.read().downcast_ref::<u8>(), Some(&7));

    let panic = InvocationPanic::new(
        MemberId::new("Type", "method", 0, fragment(19)),
        Box::new(String::from("payload")),
    );
    let payload = panic
        .downcast_payload::<String>()
        .expect("the exact panic payload type must be recoverable");
    assert_eq!(payload, "payload");
}

#[test]
fn test_registry_query_views_preserve_empty_and_exact_lookup_contracts() {
    let registry = ReflectRegistry::initialize().expect("the linked unit-test inventory is valid");
    let u8_descriptor = registry
        .get(TypeId::of::<u8>())
        .expect("built-in integer registration is linked");

    let by_type_name = registry.find_by_type_name(u8_descriptor.type_name());
    assert!(!by_type_name.is_empty());
    assert!(
        by_type_name
            .iter()
            .any(|candidate| std::ptr::eq(candidate, u8_descriptor))
    );
    assert_eq!(by_type_name.len(), by_type_name.into_iter().count());

    let by_query_name = registry.find_by_query_name(u8_descriptor.query_name());
    assert!(!by_query_name.is_empty());
    assert!(
        by_query_name
            .iter()
            .any(|candidate| std::ptr::eq(candidate, u8_descriptor))
    );
    assert!(registry.find_by_query_name("missing::query").is_empty());

    let definitions = registry.find_impl_definitions_by_target(&TypeExpression::Never);
    assert!(definitions.is_empty());
    assert_eq!(definitions.len(), definitions.iter().count());
    assert_eq!(definitions.into_iter().count(), 0);
    assert!(registry.implementations(TypeId::of::<u8>()).is_empty());
    assert!(registry.impl_definitions().iter().all(|definition| {
        registry
            .find_impl_definitions_by_target(definition.target_type())
            .iter()
            .any(|candidate| std::ptr::eq(candidate, *definition))
    }));

    assert!(registry.effective_view(TypeId::of::<u8>()).methods().is_empty());
    assert!(registry.trait_definition_by_path("missing::Trait").is_none());
    let traits = registry.find_trait_definitions_by_path("missing::Trait");
    assert!(traits.is_empty());
    assert_eq!(traits.len(), 0);
    assert_eq!(traits.iter().count(), 0);
    assert!(traits.only().is_none());
    assert_eq!(traits.into_iter().count(), 0);
}

#[test]
fn test_missing_receiver_and_owned_unit_keep_distinct_validation_facts() {
    let absent = ReceiverExpectation::none();
    let unit = ReceiverExpectation::owned::<()>();
    assert_eq!(absent.type_id(), None);
    assert_eq!(absent.type_name(), None);
    assert_eq!(unit.type_id(), Some(TypeId::of::<()>()));
    assert_eq!(unit.type_name(), Some(std::any::type_name::<()>()));

    let identity = MemberId::new("Unit", "run", 0, fragment(91));
    let invocation = Invocation::<Local>::associated([]);
    let failure = match invocation.validate(&identity, unit, &[]) {
        Ok(_) => {
            panic!("an absent receiver cannot satisfy an owned unit receiver")
        }
        Err(failure) => failure,
    };
    assert_eq!(failure.error().method_identity(), &identity);
    let recovered = failure.into_recovery().into_invocation();
    assert!(recovered.validate(&identity, absent, &[]).is_ok());
    let invocation = Invocation::<Local>::new(Some(InvocationReceiver::Owned(ReflectedOwned::new(()))), []);
    assert!(invocation.validate(&identity, unit, &[]).is_ok());
}

#[cfg(feature = "derive")]
#[derive(qubit_reflect::Reflect)]
#[reflect(crate = qubit_reflect)]
struct BorrowedSignature;

#[cfg(feature = "derive")]
#[qubit_reflect::reflect_impl(crate = qubit_reflect)]
impl BorrowedSignature {
    pub fn values(input: &[u8]) -> &[u8] {
        input
    }
}

/// Generated borrowed-slice methods remain discoverable without a dynamic
/// adapter, including their parameter and return declaration facts.
#[cfg(feature = "derive")]
#[test]
fn test_described_borrowed_signature_keeps_parameter_and_return_facts() {
    let input = [3, 4];
    assert!(std::ptr::eq(BorrowedSignature::values(&input), &input[..]));
    let registry = ReflectRegistry::initialize().expect("registry");
    let implementations = registry.implementations(TypeId::of::<BorrowedSignature>());
    let MethodLookup::Unique(instance) =
        ImplDescriptor::lookup_method(implementations, MethodQualifier::Inherent, "values")
    else {
        panic!("described-only method remains discoverable")
    };
    assert!(instance.adapter().is_none());
    assert_eq!(
        instance.unavailable_reasons(),
        [InvocationUnavailableReason::UnsupportedUnsizedValue]
    );
    let method = instance.declaration();
    let parameter = method.parameter_at(0).expect("slice parameter");
    assert_eq!(parameter.index(), 0);
    assert_eq!(parameter.name(), Some("input"));
    assert_eq!(parameter.passing_mode(), ParameterPassingMode::SharedBorrow);
    assert!(matches!(parameter.signature_type(), TypeExpression::Reference(_)));
    assert!(parameter.concrete_type().is_none());
    let output = method.return_value();
    assert_eq!(output.kind(), ReturnKind::Reference);
    assert!(matches!(output.signature_type(), Some(TypeExpression::Reference(_))));
    assert!(output.concrete_type().is_none());
}
