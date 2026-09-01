// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Tests internal eager descriptor constructors that generated static data
//! cannot expose through the public API.

use std::any::TypeId;

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
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeKind;
use crate::descriptor::TypeRef;
use crate::expression::FunctionAbi;

/// Creates an eager resolved type relation with process lifetime for internal
/// descriptor fixtures.
fn resolved_type_ref<T: crate::descriptor::Reflect>() -> &'static TypeRef {
    Box::leak(Box::new(TypeRef::Resolved(TypeDescriptor::of::<T>())))
}

/// Verifies every eager typed-view constructor retains its exact category and
/// navigable relation data.
#[test]
fn test_type_descriptor_eager_constructors_preserve_typed_views() {
    let element = resolved_type_ref::<u8>();
    let value = resolved_type_ref::<String>();
    let parameters = Box::leak(vec![element.clone()].into_boxed_slice());
    let abi = Box::leak(Box::new(FunctionAbi::Rust));

    let primitive =
        TypeDescriptor::new_primitive::<u8>("number", PrimitiveKind::U8);
    let text = TypeDescriptor::new_text::<String>("text", TextKind::String);
    let structure =
        TypeDescriptor::new_struct::<()>("record", StructKind::Unit, &[]);
    let enumeration = TypeDescriptor::new_enum::<()>("choice", &[]);
    let tuple = TypeDescriptor::new_tuple::<()>("tuple", parameters);
    let array = TypeDescriptor::new_array::<()>("array", element, 3);
    let optional = TypeDescriptor::new_optional::<()>("optional", element);
    let sequence = TypeDescriptor::new_sequence::<()>(
        "sequence",
        SequenceKind::Vec,
        element,
    );
    let set = TypeDescriptor::new_set::<()>("set", SetKind::BTreeSet, element);
    let map =
        TypeDescriptor::new_map::<()>("map", MapKind::BTreeMap, element, value);
    let smart_pointer = TypeDescriptor::new_smart_pointer::<()>(
        "pointer",
        SmartPointerKind::Arc,
        element,
    );
    let reference = TypeDescriptor::new_reference::<()>(
        "reference",
        ReferenceKind::Mutable,
        element,
    );
    let slice = TypeDescriptor::new_slice::<()>("slice", element);
    let raw_pointer = TypeDescriptor::new_raw_pointer::<()>(
        "raw",
        Mutability::Const,
        element,
    );
    let function = TypeDescriptor::new_function::<()>(
        "function",
        FunctionPointerKind::Unsafe,
        abi,
        true,
        parameters,
        value,
    );
    let opaque = TypeDescriptor::new_opaque::<()>("opaque");

    assert_eq!(
        primitive.as_primitive().expect("primitive view").kind(),
        PrimitiveKind::U8
    );
    assert_eq!(text.as_text().expect("text view").kind(), TextKind::String);
    assert_eq!(
        structure.as_struct().expect("struct view").kind(),
        StructKind::Unit
    );
    assert!(
        enumeration
            .as_enum()
            .expect("enum view")
            .representations()
            .is_empty()
    );
    assert_eq!(tuple.as_tuple().expect("tuple view").elements().len(), 1);
    assert_eq!(array.as_array().expect("array view").length(), 3);
    assert!(std::ptr::eq(
        optional
            .as_optional()
            .expect("optional view")
            .element_type(),
        element,
    ));
    assert_eq!(
        sequence.as_sequence().expect("sequence view").kind(),
        SequenceKind::Vec,
    );
    assert_eq!(set.as_set().expect("set view").kind(), SetKind::BTreeSet);
    assert!(std::ptr::eq(
        map.as_map().expect("map view").key_type(),
        element
    ));
    assert!(std::ptr::eq(
        map.as_map().expect("map view").value_type(),
        value
    ));
    assert_eq!(
        smart_pointer
            .as_smart_pointer()
            .expect("smart pointer view")
            .kind(),
        SmartPointerKind::Arc,
    );
    assert_eq!(
        reference.as_reference().expect("reference view").kind(),
        ReferenceKind::Mutable,
    );
    assert!(std::ptr::eq(
        slice.as_slice().expect("slice view").element_type(),
        element
    ));
    assert_eq!(
        raw_pointer
            .as_raw_pointer()
            .expect("raw pointer view")
            .mutability(),
        Mutability::Const,
    );
    let function_view = function.as_function().expect("function view");
    assert_eq!(function_view.kind(), FunctionPointerKind::Unsafe);
    assert_eq!(function_view.abi(), abi);
    assert!(function_view.is_variadic());
    assert_eq!(function_view.parameters().len(), 1);
    assert!(std::ptr::eq(function_view.return_type(), value));
    assert!(opaque.as_opaque().is_some());

    assert_eq!(primitive.query_name(), "number");
    assert_eq!(primitive.type_id(), TypeId::of::<u8>());
    assert_eq!(primitive.type_name(), std::any::type_name::<u8>());
    assert_eq!(primitive.kind(), TypeKind::Primitive(PrimitiveKind::U8));
    assert!(primitive.as_text().is_none());
    assert!(primitive.fields().is_empty());
    assert!(primitive.variants().is_empty());
}
