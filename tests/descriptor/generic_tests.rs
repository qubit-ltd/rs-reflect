// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow explicit-imports
//! Integration tests for concrete generic reflection instances.
use qubit_reflect as reflect;
use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;

#[derive(Reflect)]
struct User {
    id: u64,
}

#[derive(Reflect)]
struct Order {
    number: u64,
}

#[derive(Reflect)]
struct Page<T> {
    item: T,
}

#[derive(Reflect)]
struct VectorPage<T> {
    items: std::vec::Vec<T>,
}

type VectorAlias<T> = std::vec::Vec<T>;

#[derive(Reflect)]
struct AliasedVectorPage<T> {
    items: VectorAlias<T>,
}

#[derive(Reflect)]
struct ExplicitAliasedVectorPage<T: ImportedRuntimeReflect> {
    items: VectorAlias<T>,
}

mod custom_container_name {
    use super::reflect::Reflect;

    #[derive(Reflect)]
    #[reflect(opaque)]
    pub(super) struct Vec<T> {
        pub(super) value: T,
    }
}

#[derive(Reflect)]
struct CustomNamedVectorPage<T> {
    items: custom_container_name::Vec<T>,
}

#[derive(Reflect)]
struct GenericRecord<T, const N: usize> {
    values: [T; N],
}

#[derive(Reflect)]
#[reflect(opaque)]
struct OpaqueGenericRecord<T, const N: usize> {
    values: [T; N],
}

struct Unreflected;

#[derive(Reflect)]
struct DirectReflectBound<T: reflect::Reflect> {
    #[reflect(opaque)]
    value: T,
}

#[derive(Reflect)]
struct WhereReflectBound<T>
where
    T: reflect::Reflect,
{
    #[reflect(opaque)]
    value: T,
}

#[derive(Reflect)]
struct Borrowed<'a> {
    value: &'a str,
}

#[derive(Reflect)]
struct Mixed<'a, T, const N: core::primitive::usize> {
    borrowed: &'a str,
    value: T,
    bytes: [u8; N],
}

#[derive(Reflect)]
struct ScalarConsts<
    const B: bool,
    const C: char,
    const I8: i8,
    const I16: i16,
    const I32: i32,
    const I64: i64,
    const I128: i128,
    const ISIZE: isize,
    const U8: u8,
    const U16: u16,
    const U32: u32,
    const U64: u64,
    const U128: u128,
    const USIZE: usize,
>;

#[derive(Reflect)]
struct ExpressionConst<const N: usize>;

mod unrelated_contract {
    pub trait Reflect {}
}

impl unrelated_contract::Reflect for Unreflected {}

#[derive(Reflect)]
#[reflect(opaque)]
struct UnrelatedReflectBound<T: unrelated_contract::Reflect> {
    value: T,
}

trait Family {
    type Item;
}

struct UserFamily;

impl Family for UserFamily {
    type Item = User;
}

#[derive(Reflect)]
struct AssociatedItem<T: Family>
where
    T::Item: reflect::Reflect,
{
    item: T::Item,
}

#[derive(Reflect)]
#[reflect(opaque)]
struct CustomContainer<T> {
    value: T,
}

#[derive(Reflect)]
struct CustomContainerField<T> {
    container: CustomContainer<T>,
}

type Size = usize;
use core::primitive::usize as Width;

#[derive(Reflect)]
struct AliasedConsts<const SIZE: Size, const WIDTH: Width>;

#[derive(Reflect)]
struct Conditional<const N: usize>;

#[derive(Reflect)]
struct ConditionalField<const N: usize> {
    conditional: Conditional<N>,
}

use qubit_reflect::Reflect as ImportedRuntimeReflect;

#[derive(Reflect)]
#[reflect(opaque)]
struct ImportedDirectReflectBound<T: ImportedRuntimeReflect> {
    value: T,
}

#[derive(Reflect)]
#[reflect(opaque)]
struct ImportedWhereReflectBound<T>
where
    T: ImportedRuntimeReflect,
{
    value: T,
}

mod shadowed_runtime_path {
    pub(super) mod qubit_reflect {
        pub trait Reflect {}
    }

    use self::qubit_reflect::Reflect as ShadowedReflect;
    use super::reflect::Reflect as ExternalReflect;

    impl ShadowedReflect for super::Unreflected {}

    #[derive(ExternalReflect)]
    #[reflect(opaque)]
    pub(super) struct ShadowedBound<T: ShadowedReflect> {
        pub(super) value: T,
    }
}

/// Downcasts one reflected const value and compares its exact Rust type and
/// value with `expected`.
fn assert_const_value<T>(generic: &reflect::ConcreteGenericDescriptor, index: usize, expected: T)
where
    T: 'static + std::fmt::Debug + PartialEq,
{
    let actual = generic
        .const_argument_value(index)
        .unwrap_or_else(|| panic!("const argument {index} must expose an owned value"))
        .downcast::<T>()
        .unwrap_or_else(|_| panic!("const argument {index} must retain its declaration type"));
    assert_eq!(actual, expected);
}

/// Verifies every concrete type and const substitution receives its own root.
#[test]
fn test_generic_struct_instances_are_unique_and_interned() {
    let left = TypeDescriptor::of::<GenericRecord<u8, 2>>();
    let again = TypeDescriptor::of::<GenericRecord<u8, 2>>();
    let other_type = TypeDescriptor::of::<GenericRecord<u16, 2>>();
    let other_length = TypeDescriptor::of::<GenericRecord<u8, 3>>();

    assert!(std::ptr::eq(left, again));
    assert_ne!(left.type_id(), other_type.type_id());
    assert_ne!(left.type_id(), other_length.type_id());
    let generic = left
        .concrete_generic()
        .expect("derived generic roots expose substitutions");
    assert_eq!(generic.definition().parameters().len(), 2);
    assert_eq!(generic.arguments().len(), 2);
    assert_eq!(generic.definition_index(0), Some(0));
    assert_eq!(generic.definition_index(2), None);
    assert_eq!(generic.argument_index(1), Some(1));
    assert!(generic.argument_for_definition(0).is_some());
    assert!(generic.type_argument_for_definition(0).is_some());
    assert!(generic.const_argument_value_for_definition(1).is_some());
}

/// Verifies a concrete type argument resolves lazily to its unique reflected
/// root instead of using its diagnostic type name as identity.
#[test]
fn test_concrete_type_argument_navigates_to_exact_root_descriptor() {
    let descriptor = TypeDescriptor::of::<Page<User>>();
    let generic = descriptor
        .concrete_generic()
        .expect("derived generic roots expose substitutions");
    let argument = generic
        .type_argument(0)
        .expect("a reflected concrete type argument exposes its root");

    assert!(std::ptr::eq(argument, TypeDescriptor::of::<User>()));
    assert_eq!(argument.type_id(), TypeDescriptor::of::<User>().type_id());
    assert_eq!(Page { item: User { id: 7 } }.item.id, 7);
}

/// Verifies transparent reflected containers retain navigation to nested
/// concrete type parameters while the field itself keeps its complete bound.
#[test]
fn test_nested_builtin_type_arguments_navigate_to_exact_roots() {
    let vector = TypeDescriptor::of::<VectorPage<User>>()
        .concrete_generic()
        .and_then(|generic| generic.type_argument(0))
        .expect("Vec<T> proves navigation to its reflected element");
    let array = TypeDescriptor::of::<GenericRecord<User, 2>>()
        .concrete_generic()
        .and_then(|generic| generic.type_argument(0))
        .expect("[T; N] proves navigation to its reflected element");

    assert!(std::ptr::eq(vector, TypeDescriptor::of::<User>()));
    assert!(std::ptr::eq(array, TypeDescriptor::of::<User>()));
    assert_eq!(
        VectorPage {
            items: vec![User { id: 12 }]
        }
        .items[0]
            .id,
        12
    );
}

/// Verifies custom same-named containers and aliases do not create inferred
/// element bounds, while an explicit runtime bound restores navigation.
#[test]
fn test_container_path_inference_is_conservative() {
    let custom = TypeDescriptor::of::<CustomNamedVectorPage<Unreflected>>()
        .concrete_generic()
        .expect("the custom-container root retains generic metadata");
    let aliased = TypeDescriptor::of::<AliasedVectorPage<User>>()
        .concrete_generic()
        .expect("the aliased-container root retains generic metadata");
    let explicit = TypeDescriptor::of::<ExplicitAliasedVectorPage<User>>()
        .concrete_generic()
        .and_then(|generic| generic.type_argument(0))
        .expect("an explicit runtime bound restores alias element navigation");

    assert!(custom.type_argument(0).is_none());
    assert!(aliased.type_argument(0).is_none());
    assert!(std::ptr::eq(explicit, TypeDescriptor::of::<User>()));
    let value = CustomNamedVectorPage {
        items: custom_container_name::Vec { value: Unreflected },
    };
    let _ = value.items.value;
    assert_eq!(
        AliasedVectorPage {
            items: vec![User { id: 15 }]
        }
        .items[0]
            .id,
        15
    );
}

/// Verifies a field parameterized only by a const generic retains its full
/// reflected field relationship and runtime const argument.
#[test]
fn test_const_only_generic_field_is_reflected() {
    let descriptor = TypeDescriptor::of::<ConditionalField<3>>();
    let generic = descriptor
        .concrete_generic()
        .expect("the const-only field root retains generic metadata");
    let field_type = descriptor
        .fields()
        .first()
        .and_then(|field| field.field_type().as_resolved())
        .expect("the conditional const field type resolves");

    assert!(std::ptr::eq(field_type, TypeDescriptor::of::<Conditional<3>>()));
    assert_eq!(generic.arguments().len(), 1);
    assert_const_value(generic, 0, 3_usize);
    let _ = ConditionalField::<3> {
        conditional: Conditional,
    }
    .conditional;
}

/// Verifies all monomorphizations of one generic declaration share the same
/// declaration-level descriptor object.
#[test]
fn test_generic_definition_is_shared_across_concrete_instances() {
    let user = TypeDescriptor::of::<Page<User>>()
        .concrete_generic()
        .expect("the User page exposes its generic declaration");
    let order = TypeDescriptor::of::<Page<Order>>()
        .concrete_generic()
        .expect("the Order page exposes its generic declaration");

    assert!(std::ptr::eq(user.definition(), order.definition()));
    assert_eq!(
        Page {
            item: Order { number: 9 }
        }
        .item
        .number,
        9
    );
}

/// Verifies a safely representable const argument can be cloned into the
/// local reflected-owned boundary while retaining its declaration type.
#[test]
fn test_const_argument_exposes_typed_reflected_owned_value() {
    let descriptor = TypeDescriptor::of::<GenericRecord<u8, 2>>();
    let generic = descriptor
        .concrete_generic()
        .expect("derived generic roots expose substitutions");
    let reflect::expression::GenericArgument::Const(argument) = &generic.arguments()[1] else {
        panic!("the second concrete argument must be const");
    };
    let value = generic
        .const_argument_value(1)
        .expect("primitive const arguments expose reflected-owned values");

    assert_eq!(argument.normalized_diagnostic(), "2");
    let value = value
        .downcast::<usize>()
        .unwrap_or_else(|_| panic!("const value retains usize type"));
    assert_eq!(value, 2);
    assert!(generic.type_argument(1).is_none());
    assert!(generic.const_argument_value(0).is_none());
}

/// Verifies opaque generic members retain only the static bound needed by
/// roots.
#[test]
fn test_opaque_generic_instances_do_not_require_reflect_arguments() {
    let descriptor = TypeDescriptor::of::<OpaqueGenericRecord<Unreflected, 2>>();
    let value = OpaqueGenericRecord {
        values: [Unreflected, Unreflected],
    };
    let generic = descriptor
        .concrete_generic()
        .expect("opaque generic roots retain structural substitutions");

    assert!(descriptor.as_opaque().is_some());
    assert_eq!(value.values.len(), 2);
    assert!(generic.type_argument(0).is_none());
}

/// Verifies explicit direct and where-clause reflection bounds provide enough
/// proof for concrete type-argument navigation even through opaque fields.
#[test]
fn test_explicit_reflect_bounds_enable_type_argument_navigation() {
    let direct = TypeDescriptor::of::<DirectReflectBound<User>>()
        .concrete_generic()
        .and_then(|generic| generic.type_argument(0))
        .expect("a direct Reflect bound proves runtime navigation");
    let where_bound = TypeDescriptor::of::<WhereReflectBound<Order>>()
        .concrete_generic()
        .and_then(|generic| generic.type_argument(0))
        .expect("a where-clause Reflect bound proves runtime navigation");

    assert!(std::ptr::eq(direct, TypeDescriptor::of::<User>()));
    assert!(std::ptr::eq(where_bound, TypeDescriptor::of::<Order>()));
    assert_eq!(DirectReflectBound { value: User { id: 5 } }.value.id, 5);
    assert_eq!(
        WhereReflectBound {
            value: Order { number: 6 }
        }
        .value
        .number,
        6
    );
}

/// Verifies an unrelated user trait named `Reflect` cannot create a runtime
/// resolver that requires the qubit reflection contract.
#[test]
fn test_unrelated_reflect_bound_is_not_treated_as_qubit_reflect() {
    let descriptor = TypeDescriptor::of::<UnrelatedReflectBound<Unreflected>>();
    let generic = descriptor
        .concrete_generic()
        .expect("the opaque root retains its structural type argument");

    assert!(generic.type_argument(0).is_none());
    assert!(descriptor.as_opaque().is_some());
    let value = UnrelatedReflectBound { value: Unreflected };
    let _ = value.value;
}

/// Verifies imported aliases of the runtime reflection contract are resolved
/// semantically for both direct and where-clause bounds.
#[test]
fn test_imported_reflect_aliases_enable_type_argument_navigation() {
    let direct = TypeDescriptor::of::<ImportedDirectReflectBound<User>>()
        .concrete_generic()
        .and_then(|generic| generic.type_argument(0))
        .expect("the imported direct bound resolves the concrete argument");
    let where_bound = TypeDescriptor::of::<ImportedWhereReflectBound<Order>>()
        .concrete_generic()
        .and_then(|generic| generic.type_argument(0))
        .expect("the imported where bound resolves the concrete argument");

    assert!(std::ptr::eq(direct, TypeDescriptor::of::<User>()));
    assert!(std::ptr::eq(where_bound, TypeDescriptor::of::<Order>()));
    let _ = ImportedDirectReflectBound { value: User { id: 13 } }.value;
    let _ = ImportedWhereReflectBound {
        value: Order { number: 14 },
    }
    .value;
}

/// Verifies a relative local module named like the runtime crate cannot be
/// mistaken for the actual derive facade.
#[test]
fn test_shadowed_runtime_path_does_not_enable_type_argument_navigation() {
    let descriptor = TypeDescriptor::of::<shadowed_runtime_path::ShadowedBound<Unreflected>>();
    let generic = descriptor
        .concrete_generic()
        .expect("the shadowed-bound root retains structural metadata");

    assert!(generic.type_argument(0).is_none());
    let value = shadowed_runtime_path::ShadowedBound { value: Unreflected };
    let _ = value.value;
}

/// Verifies generated reflection bounds apply to the complete visible field
/// type instead of over-constraining its generic receiver.
#[test]
fn test_associated_field_uses_complete_type_reflect_bound() {
    let descriptor = TypeDescriptor::of::<AssociatedItem<UserFamily>>();
    let generic = descriptor
        .concrete_generic()
        .expect("the associated-item root retains its generic declaration");
    let field = descriptor
        .fields()
        .first()
        .expect("the associated item field is reflected");

    assert!(generic.type_argument(0).is_none());
    assert!(std::ptr::eq(
        field.field_type().as_resolved().expect("the item type resolves"),
        TypeDescriptor::of::<User>(),
    ));
    assert_eq!(AssociatedItem::<UserFamily> { item: User { id: 11 } }.item.id, 11);
}

/// Verifies a visible custom-container field constrains the container as a
/// complete type without requiring its opaque element to implement Reflect.
#[test]
fn test_custom_container_field_does_not_overconstrain_element() {
    let descriptor = TypeDescriptor::of::<CustomContainerField<Unreflected>>();
    let generic = descriptor
        .concrete_generic()
        .expect("the custom-container root retains its generic declaration");
    let field_type = descriptor
        .fields()
        .first()
        .and_then(|field| field.field_type().as_resolved())
        .expect("the complete custom-container field type resolves");

    assert!(generic.type_argument(0).is_none());
    assert!(std::ptr::eq(
        field_type,
        TypeDescriptor::of::<CustomContainer<Unreflected>>(),
    ));
    let value = CustomContainerField {
        container: CustomContainer { value: Unreflected },
    };
    let _ = value.container.value;
}

/// Verifies runtime argument indices remain unambiguous when definition-only
/// lifetime parameters are interleaved with type and const parameters.
#[test]
fn test_runtime_argument_indices_map_to_definition_parameters() {
    let descriptor = TypeDescriptor::of::<Mixed<'static, User, 3>>();
    let generic = descriptor
        .concrete_generic()
        .expect("the mixed generic root exposes substitutions");

    assert_eq!(generic.definition().parameters().len(), 3);
    assert_eq!(generic.arguments().len(), 2);
    assert_eq!(generic.definition_index(0), Some(1));
    assert_eq!(generic.definition_index(1), Some(2));
    assert_eq!(generic.argument_index(0), None);
    assert_eq!(generic.argument_index(1), Some(0));
    assert_eq!(generic.argument_index(2), Some(1));
    assert!(generic.argument_for_definition(0).is_none());
    assert!(std::ptr::eq(
        generic
            .type_argument_for_definition(1)
            .expect("the type definition parameter resolves"),
        TypeDescriptor::of::<User>(),
    ));
    let length = generic
        .const_argument_value_for_definition(2)
        .expect("the qualified primitive const parameter resolves")
        .downcast::<usize>()
        .unwrap_or_else(|_| panic!("the const value retains its qualified usize type"));
    assert_eq!(length, 3);
    let reflect::expression::GenericArgument::Type(expression) = &generic.arguments()[0] else {
        panic!("the first runtime argument must be a type");
    };
    assert!(matches!(
        expression,
        reflect::expression::TypeExpression::Parameter(name) if name.as_ref() == "T"
    ));
    let reflect::expression::GenericArgument::Const(argument) = &generic.arguments()[1] else {
        panic!("the second runtime argument must be const");
    };
    let reflect::expression::TypeExpression::Concrete(declared_type) = argument.declared_type() else {
        panic!("the qualified primitive declaration type must stay concrete");
    };
    assert_eq!(
        declared_type.path().iter().map(Box::as_ref).collect::<Vec<_>>(),
        ["core", "primitive", "usize"],
    );
    assert_eq!(
        Mixed {
            borrowed: "value",
            value: User { id: 8 },
            bytes: [1, 2, 3],
        }
        .bytes
        .len(),
        3,
    );
}

/// Verifies every stable primitive const-parameter family preserves its exact
/// owned type, including negative and platform-sized integer values.
#[test]
fn test_primitive_const_argument_matrix_preserves_exact_types() {
    type Values = ScalarConsts<true, 'λ', { -8 }, { -16 }, { -32 }, { -64 }, { -128 }, { -7 }, 8, 16, 32, 64, 128, 7>;
    let generic = TypeDescriptor::of::<Values>()
        .concrete_generic()
        .expect("the scalar matrix exposes concrete arguments");

    assert_eq!(generic.arguments().len(), 14);
    assert_const_value(generic, 0, true);
    assert_const_value(generic, 1, 'λ');
    assert_const_value(generic, 2, -8_i8);
    assert_const_value(generic, 3, -16_i16);
    assert_const_value(generic, 4, -32_i32);
    assert_const_value(generic, 5, -64_i64);
    assert_const_value(generic, 6, -128_i128);
    assert_const_value(generic, 7, -7_isize);
    assert_const_value(generic, 8, 8_u8);
    assert_const_value(generic, 9, 16_u16);
    assert_const_value(generic, 10, 32_u32);
    assert_const_value(generic, 11, 64_u64);
    assert_const_value(generic, 12, 128_u128);
    assert_const_value(generic, 13, 7_usize);
}

/// Verifies an evaluated const expression uses its canonical value for
/// diagnostics rather than its source tokens or parameter name.
#[test]
fn test_const_expression_uses_normalized_value_diagnostic() {
    let generic = TypeDescriptor::of::<ExpressionConst<{ 1 + 2 }>>()
        .concrete_generic()
        .expect("the const expression exposes its substitution");
    let reflect::expression::GenericArgument::Const(argument) = &generic.arguments()[0] else {
        panic!("the expression substitution must be const");
    };

    assert_eq!(argument.normalized_diagnostic(), "3");
    assert_const_value(generic, 0, 3_usize);
}

/// Verifies local and imported aliases of primitive const types preserve
/// argument positions and expose exact typed readers.
#[test]
fn test_const_primitive_aliases_preserve_arguments_and_values() {
    let generic = TypeDescriptor::of::<AliasedConsts<5, 7>>()
        .concrete_generic()
        .expect("aliased const parameters retain runtime substitutions");

    assert_eq!(generic.definition().parameters().len(), 2);
    assert_eq!(generic.arguments().len(), 2);
    assert_eq!(generic.definition_index(0), Some(0));
    assert_eq!(generic.definition_index(1), Some(1));
    assert_const_value(generic, 0, 5_usize);
    assert_const_value(generic, 1, 7_usize);
}

/// Verifies concurrent first navigation shares the interned concrete root,
/// declaration object, and lazily cached type-argument target.
#[test]
fn test_type_argument_lazy_resolution_is_concurrently_cached() {
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(16));
    let handles: Vec<_> = (0..16)
        .map(|_| {
            let barrier = std::sync::Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                let root = TypeDescriptor::of::<Page<User>>();
                let generic = root.concrete_generic().expect("the Page root exposes generic metadata");
                let argument = generic
                    .type_argument(0)
                    .expect("the reflected argument resolves lazily");
                (
                    root as *const TypeDescriptor as usize,
                    generic.definition() as *const _ as usize,
                    argument as *const TypeDescriptor as usize,
                )
            })
        })
        .collect();
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("generic navigation thread must finish"))
        .collect();

    assert!(results.windows(2).all(|pair| pair[0] == pair[1]));
}

#[test]
fn test_static_lifetime_generic_root_preserves_definition_without_runtime_lifetime_argument() {
    let descriptor = TypeDescriptor::of::<Borrowed<'static>>();
    let generic = descriptor.concrete_generic().expect("generic definitions are retained");
    assert_eq!(generic.definition().parameters().len(), 1);
    assert!(generic.arguments().is_empty());
    assert_eq!(Borrowed { value: "static" }.value, "static");
}
