// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow explicit-imports
//! Integration tests for the root type descriptor and its typed views.
use std::any::TypeId;

use qubit_reflect as reflect;
use reflect::Reflect;
use reflect::TypeDescriptor;
use reflect::descriptor::FieldDescriptor;
use reflect::descriptor::MethodLookup;
use reflect::descriptor::OpaqueTypeDescriptor;
use reflect::descriptor::StructKind;
use reflect::descriptor::TypeKind;
use reflect::descriptor::TypeRef;
use reflect::identity::Visibility;

struct NamedRecord {
    number: u8,
}

static NAMED_RECORD: TypeDescriptor =
    reflect::__private::descriptor::struct_type::<NamedRecord>("record", StructKind::Named, &[]);

impl Reflect for NamedRecord {
    fn type_descriptor() -> &'static TypeDescriptor {
        &NAMED_RECORD
    }
}

static UNIT_TUPLE: TypeDescriptor = reflect::__private::descriptor::tuple::<()>("unit", &[]);

struct ReflectedMember;

static REFLECTED_MEMBER: TypeDescriptor =
    reflect::__private::descriptor::struct_type::<ReflectedMember>("reflected_member", StructKind::Unit, &[]);

impl Reflect for ReflectedMember {
    fn type_descriptor() -> &'static TypeDescriptor {
        &REFLECTED_MEMBER
    }
}

struct OpaqueContainer;

fn opaque_container_descriptor() -> &'static TypeDescriptor {
    &OPAQUE_CONTAINER
}

static OPAQUE_MEMBER: OpaqueTypeDescriptor = reflect::__private::descriptor::opaque_member::<ReflectedMember>();
static OPAQUE_MEMBER_TYPE: TypeRef = TypeRef::Opaque(&OPAQUE_MEMBER);
static OPAQUE_FIELDS: [FieldDescriptor; 1] = [reflect::__private::descriptor::field(
    opaque_container_descriptor,
    0,
    Some("member"),
    Some("member"),
    &OPAQUE_MEMBER_TYPE,
    Visibility::Private,
)];
static OPAQUE_CONTAINER: TypeDescriptor = reflect::__private::descriptor::struct_type::<OpaqueContainer>(
    "opaque_container",
    StructKind::Named,
    &OPAQUE_FIELDS,
);

impl Reflect for OpaqueContainer {
    fn type_descriptor() -> &'static TypeDescriptor {
        &OPAQUE_CONTAINER
    }
}

struct RecursiveNode;

fn recursive_node_descriptor() -> &'static TypeDescriptor {
    &RECURSIVE_NODE
}

static RECURSIVE_NODE_TYPE: TypeRef = TypeRef::Resolved(&RECURSIVE_NODE);
static RECURSIVE_FIELDS: [FieldDescriptor; 1] = [reflect::__private::descriptor::field(
    recursive_node_descriptor,
    0,
    Some("next"),
    Some("next"),
    &RECURSIVE_NODE_TYPE,
    Visibility::Private,
)];
static RECURSIVE_NODE: TypeDescriptor = reflect::__private::descriptor::struct_type::<RecursiveNode>(
    "recursive_node",
    StructKind::Named,
    &RECURSIVE_FIELDS,
);

impl Reflect for RecursiveNode {
    fn type_descriptor() -> &'static TypeDescriptor {
        &RECURSIVE_NODE
    }
}

#[derive(reflect::Reflect)]
struct AggregatedNavigationTarget;

trait AmbiguousNavigationMethod {
    fn repeated(&self) -> u8;
}

#[reflect::reflect_impl(external_trait_id = "fixture.type_descriptor.ambiguous")]
impl AmbiguousNavigationMethod for AggregatedNavigationTarget {
    fn repeated(&self) -> u8 {
        1
    }
}

#[reflect::reflect_impl]
impl AggregatedNavigationTarget {
    fn inherent_only(&self) -> u8 {
        2
    }

    fn repeated(&self) -> u8 {
        3
    }
}

/// Verifies root identity, names, shape, and `Reflect` navigation.
#[test]
fn test_type_descriptor_exposes_identity_names_and_struct_shape() {
    let descriptor = TypeDescriptor::of::<NamedRecord>();
    let record = NamedRecord { number: 7 };

    assert_eq!(record.number, 7);
    assert!(std::ptr::eq(descriptor, NamedRecord::type_descriptor()));
    assert_eq!(descriptor.type_id(), TypeId::of::<NamedRecord>());
    assert_eq!(descriptor.type_name(), std::any::type_name::<NamedRecord>());
    assert_eq!(descriptor.query_name(), "record");
    assert_eq!(descriptor.kind(), TypeKind::Struct(StructKind::Named));
    assert_eq!(descriptor.as_struct().map(|view| view.kind()), Some(StructKind::Named));
    assert!(descriptor.fields().is_empty());
    assert!(descriptor.variants().is_empty());
}

/// Verifies the empty tuple is represented as a tuple of arity zero.
#[test]
fn test_type_descriptor_represents_unit_as_zero_arity_tuple() {
    let descriptor = &UNIT_TUPLE;

    assert_eq!(descriptor.kind(), TypeKind::Tuple);
    assert_eq!(descriptor.type_id(), TypeId::of::<()>());
    assert_eq!(descriptor.type_name(), std::any::type_name::<()>());
    assert_eq!(descriptor.as_tuple().map(|view| view.arity()), Some(0));
    assert!(
        descriptor
            .as_tuple()
            .expect("tuple kind should expose its typed view")
            .elements()
            .is_empty()
    );
}

/// Verifies typed navigation rejects descriptors of the wrong kind.
#[test]
fn test_type_descriptor_returns_none_for_wrong_typed_view() {
    let descriptor = TypeDescriptor::of::<NamedRecord>();

    assert!(descriptor.as_tuple().is_none());
    assert!(descriptor.as_sequence().is_none());
    assert!(descriptor.as_map().is_none());
}

/// Verifies a member explicitly declared opaque remains separate from its root
/// descriptor.
#[test]
fn test_type_descriptor_keeps_opaque_member_separate_from_root_descriptor() {
    let descriptor = TypeDescriptor::of::<OpaqueContainer>();
    let field = descriptor.field("member").expect("the named field should be present");

    let TypeRef::Opaque(opaque) = field.field_type() else {
        panic!("the member must remain explicitly opaque");
    };
    assert_eq!(opaque.type_id(), TypeId::of::<ReflectedMember>());
    assert_eq!(opaque.type_name(), TypeDescriptor::of::<ReflectedMember>().type_name());
    assert_eq!(
        TypeDescriptor::of::<ReflectedMember>().kind(),
        TypeKind::Struct(StructKind::Unit)
    );
}

/// Verifies recursive descriptor formatting is finite and still diagnostic.
#[test]
fn test_type_descriptor_debug_is_bounded_for_recursive_fields() {
    let rendered = format!("{:?}", TypeDescriptor::of::<RecursiveNode>());

    assert!(rendered.contains("RecursiveNode"));
    assert!(rendered.len() < 1_024, "recursive debug output was unexpectedly large");
}

/// Verifies published descriptor graphs can be queried concurrently through
/// shared references.
#[test]
fn test_type_descriptor_graph_is_send_and_sync() {
    fn assert_send_and_sync<T: Send + Sync>() {}

    assert_send_and_sync::<TypeDescriptor>();
    assert_send_and_sync::<TypeRef>();
    assert_send_and_sync::<OpaqueTypeDescriptor>();
    assert_send_and_sync::<FieldDescriptor>();
}

/// Verifies the root descriptor is the unified navigation entry point for all
/// registered impl fragments and their effective method instances.
#[test]
fn test_type_descriptor_navigates_registered_implementations_and_methods() {
    let descriptor = TypeDescriptor::of::<AggregatedNavigationTarget>();
    let implementations = descriptor
        .impls()
        .expect("the linked reflection registry should initialize");
    let methods = descriptor
        .methods()
        .expect("effective methods should be available through the root descriptor");

    assert_eq!(implementations.len(), 2);
    assert!(
        implementations
            .iter()
            .all(|implementation| std::ptr::eq(implementation.target_type(), descriptor))
    );
    assert!(
        methods
            .iter()
            .any(|method| method.declaration().query_name() == "inherent_only")
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.declaration().query_name() == "repeated")
            .count(),
        2
    );
}

/// Verifies unqualified lookup distinguishes a unique method from a name that
/// occurs in both the inherent and trait namespaces.
#[test]
fn test_type_descriptor_methods_named_reports_ambiguity() {
    let descriptor = TypeDescriptor::of::<AggregatedNavigationTarget>();

    assert!(matches!(
        descriptor
            .methods_named("missing")
            .expect("registry initialization should succeed"),
        MethodLookup::Missing
    ));
    assert!(matches!(
        descriptor
            .methods_named("inherent_only")
            .expect("registry initialization should succeed"),
        MethodLookup::Unique(_)
    ));
    assert!(matches!(
        descriptor
            .methods_named("repeated")
            .expect("registry initialization should succeed"),
        MethodLookup::Ambiguous
    ));
}
