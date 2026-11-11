// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::__private::codegen_v4::expression::ConstExpression;
use qubit_reflect::__private::codegen_v4::expression::ExpressionData;
use qubit_reflect::__private::codegen_v4::expression::FunctionPointerData;
use qubit_reflect::__private::codegen_v4::expression::GenericExpressionArgument;
use qubit_reflect::__private::codegen_v4::expression::LifetimeExpression;
use qubit_reflect::__private::codegen_v4::expression::OpaqueTypeData;
use qubit_reflect::__private::codegen_v4::expression::PredicateData;
use qubit_reflect::__private::codegen_v4::expression::TraitBoundModifier;
use qubit_reflect::__private::codegen_v4::expression::TraitExpressionData;
use qubit_reflect::__private::codegen_v4::expression::TraitObjectData;
use qubit_reflect::__private::codegen_v4::expression::type_expression_in;
use qubit_reflect::ReflectionContext;

static U8: ExpressionData = ExpressionData::Named {
    path: "u8",
    definition: None,
    arguments: &[],
};
static U16: ExpressionData = ExpressionData::Named {
    path: "u16",
    definition: None,
    arguments: &[],
};
static ITEM_U8_ARGS: [GenericExpressionArgument; 1] = [GenericExpressionArgument::AssociatedType {
    name: "Item",
    value: &U8,
}];
static ITEM_U16_ARGS: [GenericExpressionArgument; 1] = [GenericExpressionArgument::AssociatedType {
    name: "Item",
    value: &U16,
}];
static ITER_U8: TraitExpressionData = TraitExpressionData::new("core::iter::Iterator", &ITEM_U8_ARGS, &[]);
static ITER_U16: TraitExpressionData = TraitExpressionData::new("core::iter::Iterator", &ITEM_U16_ARGS, &[]);
static OBJECT_U8: TraitObjectData = TraitObjectData::new(&[ITER_U8], None);
static OBJECT_U16: TraitObjectData = TraitObjectData::new(&[ITER_U16], None);
static EXPRESSION_U8: ExpressionData = ExpressionData::TraitObject(&OBJECT_U8);
static EXPRESSION_U16: ExpressionData = ExpressionData::TraitObject(&OBJECT_U16);
static PARAM_T: ExpressionData = ExpressionData::ParameterName("T");
static PARAM_T2: ExpressionData = ExpressionData::ParameterName("T");
static PARAM_U: ExpressionData = ExpressionData::ParameterName("U");
static SELF: ExpressionData = ExpressionData::SelfType;
static NEVER: ExpressionData = ExpressionData::Never;
static REF: ExpressionData = ExpressionData::Reference {
    lifetime: Some(LifetimeExpression::Static),
    mutable: false,
    target: &U8,
};
static REF_ELIDED: ExpressionData = ExpressionData::Reference {
    lifetime: None,
    mutable: false,
    target: &U8,
};
static REF_MUT: ExpressionData = ExpressionData::Reference {
    lifetime: Some(LifetimeExpression::Static),
    mutable: true,
    target: &U8,
};
static PTR: ExpressionData = ExpressionData::RawPointer {
    mutable: false,
    target: &U8,
};
static PTR_MUT: ExpressionData = ExpressionData::RawPointer {
    mutable: true,
    target: &U8,
};
static SLICE: ExpressionData = ExpressionData::Slice(&U8);
static ARRAY_2: ExpressionData = ExpressionData::Array {
    element: &U8,
    length: ConstExpression::UnsignedInteger(2),
};
static ARRAY_3: ExpressionData = ExpressionData::Array {
    element: &U8,
    length: ConstExpression::UnsignedInteger(3),
};
static TUPLE: ExpressionData = ExpressionData::Tuple(&[&U8, &U16]);
static TUPLE_2: ExpressionData = ExpressionData::Tuple(&[&U8]);
static FN: FunctionPointerData = FunctionPointerData::new(
    &[&U8],
    Some(&U16),
    false,
    None,
    false,
    &[LifetimeExpression::Named("a")],
);
static FN_BINDER: FunctionPointerData = FunctionPointerData::new(
    &[&U8],
    Some(&U16),
    false,
    None,
    false,
    &[LifetimeExpression::Named("b")],
);
static FN_RESULT: FunctionPointerData =
    FunctionPointerData::new(&[&U8], Some(&U8), false, None, false, &[LifetimeExpression::Named("a")]);
static FN_ABI: FunctionPointerData = FunctionPointerData::new(
    &[&U8],
    Some(&U16),
    false,
    Some("C"),
    false,
    &[LifetimeExpression::Named("a")],
);
static FN_UNSAFE: FunctionPointerData =
    FunctionPointerData::new(&[&U8], Some(&U16), true, None, false, &[LifetimeExpression::Named("a")]);
static FN_VARIADIC: FunctionPointerData =
    FunctionPointerData::new(&[&U8], Some(&U16), false, None, true, &[LifetimeExpression::Named("a")]);
static FN_EXPR: ExpressionData = ExpressionData::FunctionPointer(&FN);
static FN_BINDER_EXPR: ExpressionData = ExpressionData::FunctionPointer(&FN_BINDER);
static FN_RESULT_EXPR: ExpressionData = ExpressionData::FunctionPointer(&FN_RESULT);
static FN_ABI_EXPR: ExpressionData = ExpressionData::FunctionPointer(&FN_ABI);
static FN_UNSAFE_EXPR: ExpressionData = ExpressionData::FunctionPointer(&FN_UNSAFE);
static FN_VARIADIC_EXPR: ExpressionData = ExpressionData::FunctionPointer(&FN_VARIADIC);
static TRAIT_LIFE: TraitExpressionData = TraitExpressionData::new("Trait", &[], &[LifetimeExpression::Named("a")]);
static TRAIT_LIFE_B: TraitExpressionData = TraitExpressionData::new("Trait", &[], &[LifetimeExpression::Named("b")]);
static TRAIT_BOUNDS: [PredicateData; 1] = [PredicateData::TraitBound {
    ty: &U8,
    trait_: &ITER_U8,
    modifier: TraitBoundModifier::None,
}];
static TRAIT_BOUNDS_CHANGED: [PredicateData; 1] = [PredicateData::TraitBound {
    ty: &U16,
    trait_: &ITER_U8,
    modifier: TraitBoundModifier::Maybe,
}];
static PREDICATE_VARIANTS: [PredicateData; 5] = [
    PredicateData::TraitBound {
        ty: &U8,
        trait_: &ITER_U8,
        modifier: TraitBoundModifier::None,
    },
    PredicateData::LifetimeOutlives {
        lifetime: LifetimeExpression::Named("a"),
        outlives: LifetimeExpression::Static,
    },
    PredicateData::TypeOutlives {
        ty: &U8,
        lifetime: LifetimeExpression::Static,
    },
    PredicateData::TypeEquality { left: &U8, right: &U16 },
    PredicateData::ConstEvaluatable(ConstExpression::Boolean(true)),
];
static PREDICATE_VARIANTS_CHANGED: [PredicateData; 5] = [
    PredicateData::TraitBound {
        ty: &U8,
        trait_: &ITER_U8,
        modifier: TraitBoundModifier::None,
    },
    PredicateData::LifetimeOutlives {
        lifetime: LifetimeExpression::Named("b"),
        outlives: LifetimeExpression::Static,
    },
    PredicateData::TypeOutlives {
        ty: &U16,
        lifetime: LifetimeExpression::Static,
    },
    PredicateData::TypeEquality { left: &U16, right: &U8 },
    PredicateData::ConstEvaluatable(ConstExpression::Boolean(false)),
];
static OPAQUE: OpaqueTypeData = OpaqueTypeData::new(&TRAIT_BOUNDS);
static OPAQUE_CHANGED: OpaqueTypeData = OpaqueTypeData::new(&TRAIT_BOUNDS_CHANGED);
static OPAQUE_EXPR: ExpressionData = ExpressionData::Opaque(&OPAQUE);
static OPAQUE_CHANGED_EXPR: ExpressionData = ExpressionData::Opaque(&OPAQUE_CHANGED);
static OPAQUE_VARIANTS: OpaqueTypeData = OpaqueTypeData::new(&PREDICATE_VARIANTS);
static OPAQUE_VARIANTS_CHANGED: OpaqueTypeData = OpaqueTypeData::new(&PREDICATE_VARIANTS_CHANGED);
static OPAQUE_VARIANTS_EXPR: ExpressionData = ExpressionData::Opaque(&OPAQUE_VARIANTS);
static OPAQUE_VARIANTS_CHANGED_EXPR: ExpressionData = ExpressionData::Opaque(&OPAQUE_VARIANTS_CHANGED);
static PREDICATES_REVERSED: [PredicateData; 2] = [PREDICATE_VARIANTS[1], PREDICATE_VARIANTS[0]];
static OPAQUE_REVERSED: OpaqueTypeData = OpaqueTypeData::new(&PREDICATES_REVERSED);
static OPAQUE_REVERSED_EXPR: ExpressionData = ExpressionData::Opaque(&OPAQUE_REVERSED);
static ASSOC: ExpressionData = ExpressionData::AssociatedType {
    self_type: &U8,
    trait_: Some(&ITER_U8),
    name: "Item",
    arguments: &[],
};
static ASSOC_CHANGED: ExpressionData = ExpressionData::AssociatedType {
    self_type: &U8,
    trait_: Some(&ITER_U16),
    name: "Item",
    arguments: &[],
};
static NAMED_WITH_LINK: ExpressionData = ExpressionData::Named {
    path: "T",
    definition: None,
    arguments: &[],
};
static LINKED_GENERICS: qubit_reflect::descriptor::GenericDefinitionMetadata =
    qubit_reflect::descriptor::GenericDefinitionMetadata::new(&[], &[]);
static LINKED_DEFINITION: qubit_reflect::descriptor::TypeDefinitionMetadata =
    qubit_reflect::descriptor::TypeDefinitionMetadata::new::<u8>(
        "T",
        "T",
        qubit_reflect::descriptor::TypeDefinitionKind::Struct(qubit_reflect::descriptor::StructKind::Named),
        &LINKED_GENERICS,
    );
static NAMED_WITH_DEFINITION: ExpressionData = ExpressionData::Named {
    path: "T",
    definition: Some(&LINKED_DEFINITION),
    arguments: &[],
};
static TRAIT_OBJECT_LIFE_A: TraitObjectData = TraitObjectData::new(&[TRAIT_LIFE], None);
static TRAIT_OBJECT_LIFE_B: TraitObjectData = TraitObjectData::new(&[TRAIT_LIFE_B], None);
static TRAIT_LIFE_EXPR_A: ExpressionData = ExpressionData::TraitObject(&TRAIT_OBJECT_LIFE_A);
static TRAIT_LIFE_EXPR_B: ExpressionData = ExpressionData::TraitObject(&TRAIT_OBJECT_LIFE_B);
static ARGUMENTS_ALL: [GenericExpressionArgument; 5] = [
    GenericExpressionArgument::Type(&U8),
    GenericExpressionArgument::Lifetime(LifetimeExpression::Static),
    GenericExpressionArgument::Const(ConstExpression::Boolean(true)),
    GenericExpressionArgument::AssociatedType {
        name: "Item",
        value: &U8,
    },
    GenericExpressionArgument::AssociatedTypeBound {
        name: "Error",
        bounds: &TRAIT_BOUNDS,
    },
];
static ARGUMENTS_CHANGED: [GenericExpressionArgument; 5] = [
    GenericExpressionArgument::Type(&U16),
    GenericExpressionArgument::Lifetime(LifetimeExpression::Static),
    GenericExpressionArgument::Const(ConstExpression::Boolean(true)),
    GenericExpressionArgument::AssociatedType {
        name: "Item",
        value: &U8,
    },
    GenericExpressionArgument::AssociatedTypeBound {
        name: "Error",
        bounds: &TRAIT_BOUNDS,
    },
];
static ARG_OWNER: ExpressionData = ExpressionData::Named {
    path: "Container",
    definition: None,
    arguments: &ARGUMENTS_ALL,
};
static ARG_OWNER_CHANGED: ExpressionData = ExpressionData::Named {
    path: "Container",
    definition: None,
    arguments: &ARGUMENTS_CHANGED,
};
static ARGUMENTS_LIFETIME_CHANGED: [GenericExpressionArgument; 5] = [
    GenericExpressionArgument::Type(&U8),
    GenericExpressionArgument::Lifetime(LifetimeExpression::Named("a")),
    GenericExpressionArgument::Const(ConstExpression::Boolean(true)),
    GenericExpressionArgument::AssociatedType {
        name: "Item",
        value: &U8,
    },
    GenericExpressionArgument::AssociatedTypeBound {
        name: "Error",
        bounds: &TRAIT_BOUNDS,
    },
];
static ARGUMENTS_CONST_CHANGED: [GenericExpressionArgument; 5] = [
    GenericExpressionArgument::Type(&U8),
    GenericExpressionArgument::Lifetime(LifetimeExpression::Static),
    GenericExpressionArgument::Const(ConstExpression::Boolean(false)),
    GenericExpressionArgument::AssociatedType {
        name: "Item",
        value: &U8,
    },
    GenericExpressionArgument::AssociatedTypeBound {
        name: "Error",
        bounds: &TRAIT_BOUNDS,
    },
];
static ARGUMENTS_ASSOC_VALUE_CHANGED: [GenericExpressionArgument; 5] = [
    GenericExpressionArgument::Type(&U8),
    GenericExpressionArgument::Lifetime(LifetimeExpression::Static),
    GenericExpressionArgument::Const(ConstExpression::Boolean(true)),
    GenericExpressionArgument::AssociatedType {
        name: "Item",
        value: &U16,
    },
    GenericExpressionArgument::AssociatedTypeBound {
        name: "Error",
        bounds: &TRAIT_BOUNDS,
    },
];
static ARGUMENTS_ASSOC_NAME_CHANGED: [GenericExpressionArgument; 5] = [
    GenericExpressionArgument::Type(&U8),
    GenericExpressionArgument::Lifetime(LifetimeExpression::Static),
    GenericExpressionArgument::Const(ConstExpression::Boolean(true)),
    GenericExpressionArgument::AssociatedType {
        name: "Other",
        value: &U8,
    },
    GenericExpressionArgument::AssociatedTypeBound {
        name: "Error",
        bounds: &TRAIT_BOUNDS,
    },
];
static ARGUMENTS_BOUND_CHANGED: [GenericExpressionArgument; 5] = [
    GenericExpressionArgument::Type(&U8),
    GenericExpressionArgument::Lifetime(LifetimeExpression::Static),
    GenericExpressionArgument::Const(ConstExpression::Boolean(true)),
    GenericExpressionArgument::AssociatedType {
        name: "Item",
        value: &U8,
    },
    GenericExpressionArgument::AssociatedTypeBound {
        name: "Error",
        bounds: &TRAIT_BOUNDS_CHANGED,
    },
];
static ARGUMENTS_BOUND_NAME_CHANGED: [GenericExpressionArgument; 5] = [
    GenericExpressionArgument::Type(&U8),
    GenericExpressionArgument::Lifetime(LifetimeExpression::Static),
    GenericExpressionArgument::Const(ConstExpression::Boolean(true)),
    GenericExpressionArgument::AssociatedType {
        name: "Item",
        value: &U8,
    },
    GenericExpressionArgument::AssociatedTypeBound {
        name: "Other",
        bounds: &TRAIT_BOUNDS,
    },
];
static ARG_OWNER_LIFE: ExpressionData = ExpressionData::Named {
    path: "Container",
    definition: None,
    arguments: &ARGUMENTS_LIFETIME_CHANGED,
};
static ARG_OWNER_CONST: ExpressionData = ExpressionData::Named {
    path: "Container",
    definition: None,
    arguments: &ARGUMENTS_CONST_CHANGED,
};
static ARG_OWNER_ASSOC_VALUE: ExpressionData = ExpressionData::Named {
    path: "Container",
    definition: None,
    arguments: &ARGUMENTS_ASSOC_VALUE_CHANGED,
};
static ARG_OWNER_ASSOC_NAME: ExpressionData = ExpressionData::Named {
    path: "Container",
    definition: None,
    arguments: &ARGUMENTS_ASSOC_NAME_CHANGED,
};
static ARG_OWNER_BOUND: ExpressionData = ExpressionData::Named {
    path: "Container",
    definition: None,
    arguments: &ARGUMENTS_BOUND_CHANGED,
};
static ARG_OWNER_BOUND_NAME: ExpressionData = ExpressionData::Named {
    path: "Container",
    definition: None,
    arguments: &ARGUMENTS_BOUND_NAME_CHANGED,
};
static REF_U16: ExpressionData = ExpressionData::Reference {
    lifetime: Some(LifetimeExpression::Static),
    mutable: false,
    target: &U16,
};
static SLICE_U16: ExpressionData = ExpressionData::Slice(&U16);
static ARRAY_U16: ExpressionData = ExpressionData::Array {
    element: &U16,
    length: ConstExpression::UnsignedInteger(2),
};
static FN_ARG: FunctionPointerData = FunctionPointerData::new(
    &[&U16],
    Some(&U16),
    false,
    None,
    false,
    &[LifetimeExpression::Named("a")],
);
static FN_ARG_EXPR: ExpressionData = ExpressionData::FunctionPointer(&FN_ARG);
static ASSOC_SELF: ExpressionData = ExpressionData::AssociatedType {
    self_type: &U16,
    trait_: Some(&ITER_U8),
    name: "Item",
    arguments: &[],
};
static ASSOC_NAME: ExpressionData = ExpressionData::AssociatedType {
    self_type: &U8,
    trait_: Some(&ITER_U8),
    name: "Other",
    arguments: &[],
};
static ASSOC_NO_TRAIT: ExpressionData = ExpressionData::AssociatedType {
    self_type: &U8,
    trait_: None,
    name: "Item",
    arguments: &[],
};
static ASSOC_ARGUMENTS: [GenericExpressionArgument; 1] = [GenericExpressionArgument::Type(&U8)];
static ASSOC_WITH_ARGUMENT: ExpressionData = ExpressionData::AssociatedType {
    self_type: &U8,
    trait_: Some(&ITER_U8),
    name: "Item",
    arguments: &ASSOC_ARGUMENTS,
};
static OBJECT_TRAITS_REVERSED: TraitObjectData = TraitObjectData::new(&[ITER_U16, ITER_U8], None);
static OBJECT_TRAITS_ORDERED: TraitObjectData = TraitObjectData::new(&[ITER_U8, ITER_U16], None);
static OBJECT_TRAITS_REVERSED_EXPR: ExpressionData = ExpressionData::TraitObject(&OBJECT_TRAITS_REVERSED);
static OBJECT_TRAITS_ORDERED_EXPR: ExpressionData = ExpressionData::TraitObject(&OBJECT_TRAITS_ORDERED);

#[test]
fn test_structural_comparison_keeps_source_differences_and_ignores_context() {
    let first = ReflectionContext::builder().build().expect("empty context builds");
    let second = ReflectionContext::builder()
        .build()
        .expect("second empty context builds");
    let left = type_expression_in(&EXPRESSION_U8, &first);
    let right = type_expression_in(&EXPRESSION_U16, &first);
    assert_eq!(left.display().to_string(), "dyn core::iter::Iterator<Item = u8>");
    assert_eq!(right.display().to_string(), "dyn core::iter::Iterator<Item = u16>");
    assert!(!left.structurally_eq(right));
    assert!(left.structurally_eq(type_expression_in(&EXPRESSION_U8, &second)));
    assert!(type_expression_in(&PARAM_T, &first).structurally_eq(type_expression_in(&PARAM_T2, &second)));
    assert!(!type_expression_in(&PARAM_T, &first).structurally_eq(type_expression_in(&PARAM_U, &second)));
}

#[test]
fn test_structural_matrix_covers_expression_variants_and_source_fields() {
    let ctx = ReflectionContext::builder().build().unwrap();
    let same = |a: &'static ExpressionData, b: &'static ExpressionData| {
        type_expression_in(a, &ctx).structurally_eq(type_expression_in(b, &ctx))
    };
    for (a, b) in [
        (&U8, &U8),
        (&SELF, &SELF),
        (&NEVER, &NEVER),
        (&REF, &REF),
        (&PTR, &PTR),
        (&SLICE, &SLICE),
        (&ARRAY_2, &ARRAY_2),
        (&TUPLE, &TUPLE),
        (&FN_EXPR, &FN_EXPR),
        (&EXPRESSION_U8, &EXPRESSION_U8),
        (&OPAQUE_EXPR, &OPAQUE_EXPR),
        (&OPAQUE_VARIANTS_EXPR, &OPAQUE_VARIANTS_EXPR),
        (&ASSOC, &ASSOC),
        (&PARAM_T, &PARAM_T2),
        (&NAMED_WITH_LINK, &NAMED_WITH_DEFINITION),
        (&ARG_OWNER, &ARG_OWNER),
    ] {
        assert!(same(a, b), "expected structurally equal expressions");
    }
    for (a, b) in [
        (&U8, &U16),
        (&PARAM_T, &PARAM_U),
        (&REF, &REF_ELIDED),
        (&REF, &REF_MUT),
        (&REF, &REF_U16),
        (&PTR, &PTR_MUT),
        (&ARRAY_2, &ARRAY_3),
        (&ARRAY_2, &ARRAY_U16),
        (&SLICE, &SLICE_U16),
        (&TUPLE, &TUPLE_2),
        (&FN_EXPR, &FN_BINDER_EXPR),
        (&FN_EXPR, &FN_RESULT_EXPR),
        (&FN_EXPR, &FN_ABI_EXPR),
        (&FN_EXPR, &FN_UNSAFE_EXPR),
        (&FN_EXPR, &FN_VARIADIC_EXPR),
        (&FN_EXPR, &FN_ARG_EXPR),
        (&EXPRESSION_U8, &EXPRESSION_U16),
        (&OPAQUE_EXPR, &OPAQUE_CHANGED_EXPR),
        (&OPAQUE_VARIANTS_EXPR, &OPAQUE_VARIANTS_CHANGED_EXPR),
        (&OPAQUE_VARIANTS_EXPR, &OPAQUE_REVERSED_EXPR),
        (&ASSOC, &ASSOC_CHANGED),
        (&ASSOC, &ASSOC_SELF),
        (&ASSOC, &ASSOC_NAME),
        (&ASSOC, &ASSOC_NO_TRAIT),
        (&ASSOC, &ASSOC_WITH_ARGUMENT),
        (&TRAIT_LIFE_EXPR_A, &TRAIT_LIFE_EXPR_B),
        (&ARG_OWNER, &ARG_OWNER_CHANGED),
        (&ARG_OWNER, &ARG_OWNER_LIFE),
        (&ARG_OWNER, &ARG_OWNER_CONST),
        (&ARG_OWNER, &ARG_OWNER_ASSOC_VALUE),
        (&ARG_OWNER, &ARG_OWNER_ASSOC_NAME),
        (&ARG_OWNER, &ARG_OWNER_BOUND),
        (&ARG_OWNER, &ARG_OWNER_BOUND_NAME),
        (&OBJECT_TRAITS_REVERSED_EXPR, &OBJECT_TRAITS_ORDERED_EXPR),
    ] {
        assert!(!same(a, b), "expected structural difference");
    }
}
