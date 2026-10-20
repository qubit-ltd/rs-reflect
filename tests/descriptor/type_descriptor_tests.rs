// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// =============================================================================

//! Integration tests for context-bound root type descriptors.

use std::any::TypeId;

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::StructKind;
use qubit_reflect::descriptor::TypeDefinitionKind;
use qubit_reflect::descriptor::TypeKind;
use qubit_reflect::lookup::Lookup;

#[Reflect]
struct NamedRecord {
    number: u8,
}

#[Reflect]
struct OpaqueContainer {
    member: OpaqueMember,
}

struct OpaqueMember;

#[Reflect]
struct RecursiveNode {
    next: Option<Box<RecursiveNode>>,
}

#[Reflect]
struct AggregatedNavigationTarget;

#[Reflect]
trait AmbiguousNavigationMethod {
    fn repeated(&self) -> u8;
}

#[ReflectImpl(reflected_trait = AmbiguousNavigationMethod)]
impl AmbiguousNavigationMethod for AggregatedNavigationTarget {
    fn repeated(&self) -> u8 {
        1
    }
}

#[ReflectImpl]
impl AggregatedNavigationTarget {
    fn inherent_only(&self) -> u8 {
        2
    }

    fn repeated(&self) -> u8 {
        3
    }
}

/// Verifies root identity, type names, shape, and field navigation.
#[test]
fn test_type_descriptor_exposes_identity_names_and_struct_shape() {
    let descriptor = TypeDescriptor::of::<NamedRecord>();
    let record = NamedRecord { number: 7 };

    assert_eq!(record.number, 7);
    assert_eq!(descriptor.type_id(), TypeId::of::<NamedRecord>());
    assert_eq!(descriptor.rust_type_name(), std::any::type_name::<NamedRecord>());
    assert_eq!(descriptor.name(), "NamedRecord");
    assert!(matches!(
        descriptor.definition().unwrap().kind(),
        TypeDefinitionKind::Struct(StructKind::Named)
    ));
    assert_eq!(descriptor.fields().len(), 1);
    assert_eq!(descriptor.field("number").expect("field exists").name(), "number");
    assert!(descriptor.variants().is_empty());
}

/// Verifies unit retains its zero-element tuple shape.
#[test]
fn test_type_descriptor_represents_unit_as_zero_arity_tuple() {
    let descriptor = TypeDescriptor::of::<()>();

    assert_eq!(descriptor.type_id(), TypeId::of::<()>());
    assert!(matches!(descriptor.kind(), TypeKind::Tuple(view) if view.fields().is_empty()));
    assert!(descriptor.fields().is_empty());
}

/// Verifies a field whose type has no `Reflect` implementation keeps its
/// expression without claiming a concrete descriptor.
#[test]
fn test_type_descriptor_keeps_unreflectable_field_expression() {
    let descriptor = TypeDescriptor::of::<OpaqueContainer>();
    let field = descriptor.field("member").expect("the named field is present");

    assert!(field.find_concrete_type().is_none());
    assert!(field.type_expression().display().to_string().contains("OpaqueMember"));
}

/// Verifies recursive descriptor formatting stays finite.
#[test]
fn test_type_descriptor_debug_is_bounded_for_recursive_fields() {
    let rendered = TypeDescriptor::of::<RecursiveNode>()
        .field("next")
        .expect("recursive field exists")
        .type_expression()
        .display()
        .to_string();

    assert!(rendered.contains("RecursiveNode"));
    assert!(rendered.len() < 1_024, "recursive debug output was unexpectedly large");
}

/// Verifies public descriptors remain shareable across threads.
#[test]
fn test_type_descriptor_is_send_and_sync() {
    fn assert_send_and_sync<T: Send + Sync>() {}

    assert_send_and_sync::<TypeDescriptor<'static>>();
}

/// Verifies the type descriptor exposes its linked impls and method lookup.
#[test]
fn test_type_descriptor_navigates_impls_and_reports_method_ambiguity() {
    let descriptor = TypeDescriptor::of::<AggregatedNavigationTarget>();
    let implementations = descriptor.impls().expect("linked impls are available");

    assert_eq!(implementations.len(), 2);
    assert!(
        implementations
            .iter()
            .all(|implementation| implementation.target().type_id() == descriptor.type_id())
    );

    assert!(matches!(descriptor.lookup_method("missing").unwrap(), Lookup::Missing));
    assert!(matches!(
        descriptor.lookup_method("inherent_only").unwrap(),
        Lookup::Unique(_)
    ));
    assert!(matches!(
        descriptor.lookup_method("repeated").unwrap(),
        Lookup::Ambiguous(_)
    ));
}
