// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::BuildHasherDefault;
use std::hash::DefaultHasher;
use std::rc::Rc;
use std::sync::Arc;

use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::Reflect;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::descriptor::type_kind::TypeKind;

macro_rules! assert_kind {
    ($ty:ty, $kind:pat) => {
        assert!(matches!(TypeDescriptor::of::<$ty>().kind(), $kind));
    };
}

fn context_for<T: Reflect>() -> ReflectionContext {
    ReflectionContext::builder()
        .include_type::<T>()
        .build()
        .expect("built-in type registration should produce a valid context")
}

#[test]
fn built_in_collections_register_their_type_kinds() {
    let vector_context = context_for::<Vec<u8>>();
    assert!(matches!(
        vector_context.type_of::<Vec<u8>>().kind(),
        TypeKind::Sequence(_)
    ));

    let option_context = context_for::<Option<u8>>();
    assert!(matches!(
        option_context.type_of::<Option<u8>>().kind(),
        TypeKind::Optional(_)
    ));

    let array_context = context_for::<[u8; 4]>();
    assert!(matches!(array_context.type_of::<[u8; 4]>().kind(), TypeKind::Array(_)));

    let primitive_context = context_for::<u8>();
    assert!(matches!(
        primitive_context.type_of::<u8>().kind(),
        TypeKind::Primitive(_)
    ));

    let text_context = context_for::<String>();
    assert!(matches!(text_context.type_of::<String>().kind(), TypeKind::Text(_)));
}

#[test]
fn built_in_tuples_register_tuple_kind_including_unit() {
    let tuple_context = context_for::<(u8, String)>();
    assert!(matches!(
        tuple_context.type_of::<(u8, String)>().kind(),
        TypeKind::Tuple(_)
    ));

    let unit_context = context_for::<()>();
    assert!(matches!(unit_context.type_of::<()>().kind(), TypeKind::Tuple(_)));
}

#[test]
fn built_in_sets_and_maps_register_container_kinds() {
    assert_kind!(HashSet<u8>, TypeKind::Set(_));
    assert_kind!(BTreeSet<u8>, TypeKind::Set(_));
    assert_kind!(HashMap<u8, String>, TypeKind::Map(_));
    assert_kind!(BTreeMap<u8, String>, TypeKind::Map(_));

    type CustomHashMap = HashMap<u8, String, BuildHasherDefault<DefaultHasher>>;
    assert_kind!(CustomHashMap, TypeKind::Map(_));
}

#[test]
fn built_in_pointer_and_unsized_types_register_shape_kinds() {
    assert_kind!(Box<u8>, TypeKind::SmartPointer(_));
    assert_kind!(Rc<u8>, TypeKind::SmartPointer(_));
    assert_kind!(Arc<u8>, TypeKind::SmartPointer(_));
    assert_kind!(&'static u8, TypeKind::Reference(_));
    assert_kind!(&'static mut u8, TypeKind::Reference(_));
    assert_kind!([u8], TypeKind::Slice(_));
    assert_kind!(*const u8, TypeKind::RawPointer(_));
    assert_kind!(*mut u8, TypeKind::RawPointer(_));
    assert_kind!(fn(u8) -> String, TypeKind::Function(_));
    assert_kind!(unsafe fn(u8) -> String, TypeKind::Function(_));
    assert_kind!(extern "C" fn(u8) -> String, TypeKind::Function(_));
    assert_kind!(dyn std::fmt::Debug, TypeKind::TraitObject(_));
    assert_kind!(dyn std::any::Any, TypeKind::TraitObject(_));
}

#[test]
fn built_in_primitive_registrations_preserve_concrete_type_identity() {
    macro_rules! assert_primitive {
        ($($ty:ty),+ $(,)?) => {
            $(
                let descriptor = TypeDescriptor::of::<$ty>();
                assert_eq!(descriptor.type_id(), std::any::TypeId::of::<$ty>());
                assert!(matches!(descriptor.kind(), TypeKind::Primitive(_)));
            )+
        };
    }

    assert_primitive!(
        bool, char, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64,
    );

    let text = TypeDescriptor::of::<str>();
    assert_eq!(text.type_id(), std::any::TypeId::of::<str>());
    assert!(matches!(text.kind(), TypeKind::Text(_)));
}

#[test]
fn built_in_function_pointer_registrations_cover_abi_and_safety_variants() {
    macro_rules! assert_function {
        ($($ty:ty),+ $(,)?) => {
            $(
                let descriptor = TypeDescriptor::of::<$ty>();
                assert_eq!(descriptor.type_id(), std::any::TypeId::of::<$ty>());
                assert!(matches!(descriptor.kind(), TypeKind::Function(_)));
            )+
        };
    }

    assert_function!(
        fn() -> (),
        unsafe fn(u8) -> String,
        extern "C" fn(u8) -> String,
        unsafe extern "C" fn(u8) -> String,
        extern "C-unwind" fn(u8) -> String,
        unsafe extern "C-unwind" fn(u8) -> String,
        extern "system" fn(u8) -> String,
        unsafe extern "system" fn(u8) -> String,
        extern "system-unwind" fn(u8) -> String,
        unsafe extern "system-unwind" fn(u8) -> String,
    );
}

#[test]
fn built_in_array_and_tuple_registrations_cover_empty_and_large_shapes() {
    assert_kind!([u8; 0], TypeKind::Array(_));
    assert_kind!([[u8; 2]; 3], TypeKind::Array(_));

    assert_kind!((u8,), TypeKind::Tuple(_));
    assert_kind!(
        (u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64),
        TypeKind::Tuple(_)
    );
    assert_kind!(
        (
            u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8,
            u8, u8, u8, u8, u8
        ),
        TypeKind::Tuple(_)
    );
}

#[test]
fn built_in_smart_pointer_and_reference_registrations_accept_unsized_targets() {
    assert_kind!(Box<str>, TypeKind::SmartPointer(_));
    assert_kind!(Rc<str>, TypeKind::SmartPointer(_));
    assert_kind!(Arc<str>, TypeKind::SmartPointer(_));
    assert_kind!(&'static str, TypeKind::Reference(_));
    assert_kind!(&'static mut [u8], TypeKind::Reference(_));
}
