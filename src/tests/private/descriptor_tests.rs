// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Tests hidden descriptor factories used by generated code.

use crate::__private::codegen_v1::expression as codegen_expression;
use crate::__private::descriptor;
use crate::capability::TypeCapabilities;
use crate::descriptor::EnumRepr;
use crate::descriptor::FunctionPointerKind;
use crate::descriptor::MapKind;
use crate::descriptor::Mutability;
use crate::descriptor::PrimitiveKind;
use crate::descriptor::ReferenceKind;
use crate::descriptor::SequenceKind;
use crate::descriptor::SetKind;
use crate::descriptor::SmartPointerKind;
use crate::descriptor::StructKind;
use crate::descriptor::TextKind;
use crate::descriptor::TraitDescriptor;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeRef;
use crate::descriptor::VariantKind;
use crate::error::TypeMismatch;
use crate::expression::ConstExpression;
use crate::expression::FunctionAbi;
use crate::expression::GenericArgument;
use crate::expression::LifetimeExpression;
use crate::expression::TraitBoundModifier;
use crate::expression::TypeExpression;
use crate::identity::Visibility;
use crate::value::ReflectedRef;

struct InternedFixture;

/// Builds the descriptor used to test the hidden interning facade.
fn build_interned_fixture() -> TypeDescriptor {
    descriptor::opaque_root::<InternedFixture>("interned-fixture")
}

/// Returns the empty capability set for capability-bearing factory fixtures.
fn empty_capabilities() -> &'static TypeCapabilities {
    crate::capability::empty_capabilities()
}

/// Resolves the built-in debug trait object declaration for a trait-object
/// factory fixture.
fn debug_trait_descriptor() -> &'static TraitDescriptor {
    TypeDescriptor::of::<dyn std::fmt::Debug>()
        .as_trait_object()
        .expect("debug trait object view")
        .trait_descriptor()
}

/// Reports every value as belonging to the fixture variant.
fn active_variant(_target: ReflectedRef<'_>) -> Result<bool, TypeMismatch> {
    Ok(true)
}

/// Returns one constant value used by the associated-constant reader factory.
fn constant_value() -> u8 {
    41
}

/// Verifies generated-code const argument helpers preserve every supported
/// primitive category and exact owned value.
#[test]
fn test_const_argument_factories_cover_supported_primitives() {
    macro_rules! assert_signed {
        ($($value:expr),+ $(,)?) => {
            $(assert!(matches!(
                descriptor::const_argument_expression($value),
                ConstExpression::SignedInteger(_)
            ));)+
        };
    }
    macro_rules! assert_unsigned {
        ($($value:expr),+ $(,)?) => {
            $(assert!(matches!(
                descriptor::const_argument_expression($value),
                ConstExpression::UnsignedInteger(_)
            ));)+
        };
    }

    assert_signed!(1_i8, 2_i16, 3_i32, 4_i64, 5_i128, 6_isize);
    assert_unsigned!(1_u8, 2_u16, 3_u32, 4_u64, 5_u128, 6_usize);
    assert_eq!(
        descriptor::const_argument_expression(true),
        ConstExpression::Boolean(true)
    );
    assert_eq!(
        descriptor::const_argument_expression('x'),
        ConstExpression::Character('x')
    );
    assert_eq!(descriptor::const_argument_diagnostic(7_i8).as_ref(), "7");
    assert_eq!(descriptor::const_argument_diagnostic(false).as_ref(), "false");
    assert_eq!(descriptor::const_argument_diagnostic('x').as_ref(), "'x'");
    assert_eq!(descriptor::const_argument_owned(9_u8).downcast_ref::<u8>(), Some(&9));

    let reader = descriptor::associated_const_reader(constant_value);
    assert_eq!(reader.read().downcast_ref::<u8>(), Some(&41));
    assert_eq!(format!("{reader:?}"), "AssociatedConstReader(..)");
}

/// Verifies the versioned code-generation facade preserves checked expression
/// invariants and every structural input passed by generated code.
#[test]
fn test_codegen_v1_expression_factories_preserve_structural_inputs() {
    let concrete = codegen_expression::concrete(
        vec![
            Box::<str>::from("std"),
            Box::<str>::from("vec"),
            Box::<str>::from("Vec"),
        ]
        .into_boxed_slice(),
        vec![GenericArgument::Type(TypeExpression::SelfType)].into_boxed_slice(),
        "std::vec::Vec<Self>".into(),
    );
    assert_eq!(
        concrete.path().iter().map(AsRef::as_ref).collect::<Vec<_>>(),
        ["std", "vec", "Vec"]
    );
    assert_eq!(concrete.arguments(), &[GenericArgument::Type(TypeExpression::SelfType)]);
    assert_eq!(concrete.diagnostic(), Some("std::vec::Vec<Self>"));

    let const_argument = codegen_expression::const_argument(
        TypeExpression::Concrete(concrete),
        ConstExpression::UnsignedInteger(4),
        "4",
    );
    assert!(matches!(const_argument.declared_type(), TypeExpression::Concrete(_)));
    assert_eq!(const_argument.value(), &ConstExpression::UnsignedInteger(4));
    assert_eq!(const_argument.normalized_diagnostic(), "4");

    let array = codegen_expression::array(TypeExpression::SelfType, ConstExpression::Parameter("N".into()));
    assert_eq!(array.element(), &TypeExpression::SelfType);
    assert_eq!(array.length(), &ConstExpression::Parameter("N".into()));

    let reference =
        codegen_expression::reference(LifetimeExpression::Named("a".into()), true, TypeExpression::SelfType);
    assert_eq!(reference.lifetime(), &LifetimeExpression::Named("a".into()));
    assert!(reference.is_mutable());
    assert_eq!(reference.target(), &TypeExpression::SelfType);

    let type_bound = codegen_expression::type_bound(
        TypeExpression::Parameter("T".into()),
        vec![TypeExpression::Parameter("Display".into())].into_boxed_slice(),
        vec![TraitBoundModifier::None].into_boxed_slice(),
        vec![LifetimeExpression::Named("a".into())].into_boxed_slice(),
    );
    assert!(matches!(
        type_bound,
        crate::expression::PredicateDescriptor::TypeBound { .. }
    ));

    let outlives = codegen_expression::lifetime_outlives(
        LifetimeExpression::Named("a".into()),
        vec![LifetimeExpression::Static].into_boxed_slice(),
    );
    assert!(matches!(
        outlives,
        crate::expression::PredicateDescriptor::LifetimeOutlives { .. }
    ));
}

/// Verifies hidden eager factories retain all typed categories used by macro
/// output without exposing another public construction API.
#[test]
fn test_descriptor_factories_preserve_typed_categories() {
    let element = Box::leak(Box::new(TypeRef::Resolved(TypeDescriptor::of::<u8>())));
    let value = Box::leak(Box::new(TypeRef::Resolved(TypeDescriptor::of::<String>())));
    let parameters = Box::leak(vec![element.clone()].into_boxed_slice());
    let abi = Box::leak(Box::new(FunctionAbi::Rust));

    let primitive = descriptor::primitive::<u8>("primitive", PrimitiveKind::U8);
    let primitive_caps =
        descriptor::primitive_with_capabilities::<u8>("primitive-caps", PrimitiveKind::U8, empty_capabilities);
    let text = descriptor::text::<String>("text", TextKind::String);
    let text_caps = descriptor::text_with_capabilities::<String>("text-caps", TextKind::String, empty_capabilities);
    let structure = descriptor::struct_type::<()>("struct", StructKind::Unit, &[]);
    let enumeration = descriptor::enum_type::<()>("enum", &[]);
    let represented = descriptor::enum_type_with_repr::<()>("repr", &[], &[EnumRepr::C]);
    let tuple = descriptor::tuple::<()>("tuple", parameters);
    let array = descriptor::array::<()>("array", element, 2);
    let optional = descriptor::optional::<()>("optional", element);
    let sequence = descriptor::sequence::<()>("sequence", SequenceKind::Vec, element);
    let set = descriptor::set::<()>("set", SetKind::HashSet, element);
    let map = descriptor::map::<()>("map", MapKind::HashMap, element, value);
    let smart = descriptor::smart_pointer::<()>("smart", SmartPointerKind::Box, element);
    let reference = descriptor::reference::<()>("reference", ReferenceKind::Shared, element);
    let slice = descriptor::slice::<()>("slice", element);
    let raw = descriptor::raw_pointer::<()>("raw", Mutability::Mutable, element);
    let function = descriptor::function::<()>("function", FunctionPointerKind::Safe, abi, false, parameters, value);
    let trait_object = descriptor::trait_object::<dyn std::fmt::Debug>("debug", debug_trait_descriptor);
    let opaque = descriptor::opaque_root::<()>("opaque");
    let opaque_caps = descriptor::opaque_root_with_capabilities::<()>("opaque-caps", empty_capabilities);
    let attached_caps =
        descriptor::with_capabilities(descriptor::opaque_root::<()>("attached-caps"), empty_capabilities);

    assert!(primitive.as_primitive().is_some());
    assert!(primitive_caps.as_primitive().is_some());
    assert!(text.as_text().is_some());
    assert!(text_caps.as_text().is_some());
    assert!(structure.as_struct().is_some());
    assert!(enumeration.as_enum().is_some());
    assert_eq!(
        represented.as_enum().expect("represented enum").representations(),
        &[EnumRepr::C],
    );
    assert_eq!(tuple.as_tuple().expect("tuple").arity(), 1);
    assert_eq!(array.as_array().expect("array").length(), 2);
    assert!(optional.as_optional().is_some());
    assert!(std::ptr::eq(
        sequence.as_sequence().expect("sequence").element_type(),
        element,
    ));
    assert!(std::ptr::eq(set.as_set().expect("set").element_type(), element));
    assert!(map.as_map().is_some());
    assert!(std::ptr::eq(
        smart.as_smart_pointer().expect("smart pointer").pointee_type(),
        element,
    ));
    assert!(std::ptr::eq(
        reference.as_reference().expect("reference").target_type(),
        element,
    ));
    assert!(slice.as_slice().is_some());
    assert!(std::ptr::eq(
        raw.as_raw_pointer().expect("raw pointer").pointee_type(),
        element,
    ));
    assert!(function.as_function().is_some());
    assert!(trait_object.as_trait_object().is_some());
    assert!(format!("{:?}", trait_object.as_trait_object().expect("trait object")).contains("Debug"));
    assert!(opaque.as_opaque().is_some());
    assert!(opaque_caps.as_opaque().is_some());
    assert!(attached_caps.capabilities().descriptors().is_empty());

    let opaque_member = descriptor::opaque_member::<u16>();
    assert_eq!(opaque_member.type_name(), std::any::type_name::<u16>());
    assert!(format!("{opaque_member:?}").contains("OpaqueTypeDescriptor"));

    let opaque_ref = TypeRef::Opaque(Box::leak(Box::new(opaque_member)));
    let symbolic_ref = TypeRef::Symbolic(TypeExpression::SelfType);
    assert!(opaque_ref.as_opaque().is_some());
    assert!(opaque_ref.as_symbolic().is_none());
    assert!(symbolic_ref.as_symbolic().is_some());
    assert!(symbolic_ref.as_opaque().is_none());
    assert!(format!("{opaque_ref:?}").contains("Opaque"));
    assert!(format!("{symbolic_ref:?}").contains("Symbolic"));
    assert!(format!("{element:?}").contains("Resolved"));
}

/// Verifies lazy field, variant, and interning factories remain navigable and
/// preserve stable identity.
#[test]
fn test_relation_and_member_factories_preserve_navigation() {
    let lazy = descriptor::lazy_type_ref::<u8>();
    assert!(std::ptr::eq(
        lazy.get().as_resolved().expect("lazy u8 should resolve"),
        TypeDescriptor::of::<u8>(),
    ));
    let field = descriptor::lazy_field(
        TypeDescriptor::of::<u8>,
        0,
        Some("value"),
        Some("value"),
        lazy,
        Visibility::Private,
    );
    assert!(field.field_type().as_resolved().is_some());
    let fields = Box::leak(vec![field].into_boxed_slice());
    let variant = descriptor::variant(
        TypeDescriptor::of::<u8>,
        0,
        "Value",
        "Value",
        VariantKind::Tuple,
        fields,
        active_variant,
    );
    assert_eq!(variant.rust_name(), "Value");

    let first = descriptor::intern_type::<InternedFixture>(build_interned_fixture);
    let second = descriptor::intern_type::<InternedFixture>(build_interned_fixture);
    assert!(std::ptr::eq(first, second));
}
