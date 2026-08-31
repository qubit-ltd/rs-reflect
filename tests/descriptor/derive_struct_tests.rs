// qubit-style: allow explicit-imports
//! Integration tests for `Reflect` struct derives.
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Barrier;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use qubit_reflect as reflect;
use reflect::Reflect;
use reflect::TypeDescriptor;
use reflect::descriptor::StructKind;
use reflect::descriptor::TypeKind;
use reflect::registry::ReflectRegistry;
use reflect::value::ReflectedMut;
use reflect::value::ReflectedOwned;
use reflect::value::ReflectedRef;

#[derive(Reflect)]
struct DerivedNamed {
    value: u8,
}

#[derive(Reflect)]
struct DerivedTuple(u8, String);

#[derive(Reflect)]
struct DerivedNewtype(u16);

#[derive(Reflect)]
struct DerivedUnit;

#[derive(Reflect)]
#[reflect(opaque)]
struct DerivedOpaque<T> {
    value: T,
}

#[derive(Reflect)]
struct DerivedGeneric<T> {
    value: T,
}

#[derive(Reflect)]
struct DerivedOpaqueMember<T> {
    #[reflect(opaque)]
    value: T,
}

#[derive(Reflect)]
#[reflect(rename = "renamed-record")]
struct RenamedFields {
    #[reflect(rename = "payload", read_only)]
    pub(crate) value: u8,
    #[reflect(skip)]
    skipped: String,
}

#[derive(Reflect)]
struct DirectRecursiveNode {
    next: Option<Box<DirectRecursiveNode>>,
}

#[derive(Reflect)]
struct IndirectRecursiveLeft {
    right: Option<Box<IndirectRecursiveRight>>,
}

#[derive(Reflect)]
struct IndirectRecursiveRight {
    left: Option<Box<IndirectRecursiveLeft>>,
}

#[derive(Reflect)]
struct RecursiveVectorNode {
    children: Vec<RecursiveVectorNode>,
}

#[derive(Reflect)]
struct RecursiveCompositeNode {
    array: [Box<RecursiveCompositeNode>; 1],
    tuple: (Box<RecursiveCompositeNode>,),
    map: std::collections::BTreeMap<u8, RecursiveCompositeNode>,
}

/// Runs one exact integration test in a child process and fails after a
/// bounded wait if descriptor initialization deadlocks.
fn assert_isolated_test_completes(test_name: &str, marker: &str) {
    let executable = std::env::current_exe().expect("integration-test executable should be available");
    let mut child = Command::new(executable)
        .arg("--exact")
        .arg(test_name)
        .arg("--nocapture")
        .env(marker, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("isolated recursive-descriptor test should start");
    let deadline = Instant::now() + Duration::from_secs(5);

    loop {
        match child.try_wait().expect("isolated test status should be readable") {
            Some(status) => {
                assert!(status.success(), "isolated recursive-descriptor test failed: {status}");
                return;
            }
            None if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            None => {
                child.kill().expect("deadlocked isolated test should be terminated");
                let _ = child.wait();
                panic!("recursive descriptor initialization did not complete within five seconds");
            }
        }
    }
}

/// Returns the resolved target descriptor for one reflected field.
fn resolved_field_type(descriptor: &'static TypeDescriptor) -> &'static TypeDescriptor {
    descriptor
        .field_at(0)
        .expect("recursive descriptor should retain its field")
        .field_type()
        .as_resolved()
        .expect("recursive field should resolve to a concrete descriptor")
}

/// Navigates an `Option<Box<T>>` relation and returns the pointee descriptor.
fn optional_box_pointee(descriptor: &'static TypeDescriptor) -> &'static TypeDescriptor {
    let optional_element = descriptor
        .as_optional()
        .expect("recursive field should resolve to Option")
        .element_type()
        .as_resolved()
        .expect("Option element should resolve to Box");
    optional_element
        .as_smart_pointer()
        .expect("Option element should expose the smart-pointer view")
        .pointee_type()
        .as_resolved()
        .expect("Box pointee should resolve to the recursive root")
}

/// Navigates a `Vec<T>` relationship and returns its element descriptor.
fn sequence_element(descriptor: &'static TypeDescriptor) -> &'static TypeDescriptor {
    descriptor
        .as_sequence()
        .expect("recursive field should resolve to Vec")
        .element_type()
        .as_resolved()
        .expect("Vec element should resolve to the recursive root")
}

/// Returns the resolved type descriptor of a field at `index`.
fn resolved_field_type_at(descriptor: &'static TypeDescriptor, index: usize) -> &'static TypeDescriptor {
    descriptor
        .field_at(index)
        .expect("recursive descriptor should retain every field")
        .field_type()
        .as_resolved()
        .expect("recursive field should resolve to a concrete descriptor")
}

/// Verifies concurrent first lookup of a direct recursive derive completes and
/// preserves the full `Node -> Option -> Box -> Node` identity cycle.
#[test]
fn test_derive_reflect_initializes_direct_recursive_descriptor_without_deadlock() {
    const MARKER: &str = "QUBIT_REFLECT_DIRECT_RECURSION_CHILD";
    if std::env::var_os(MARKER).is_none() {
        assert_isolated_test_completes(
            "descriptor::derive_struct_tests::test_derive_reflect_initializes_direct_recursive_descriptor_without_deadlock",
            MARKER,
        );
        return;
    }

    let barrier = Arc::new(Barrier::new(17));
    let threads: Vec<_> = (0..16)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                let descriptor = TypeDescriptor::of::<DirectRecursiveNode>();
                assert_eq!(descriptor.fields().len(), 1);
                let optional = resolved_field_type(descriptor);
                assert!(std::ptr::eq(optional_box_pointee(optional), descriptor));
                descriptor
            })
        })
        .collect();
    barrier.wait();
    let root = TypeDescriptor::of::<DirectRecursiveNode>();
    for worker in threads {
        assert!(std::ptr::eq(
            root,
            worker.join().expect("recursive descriptor worker should complete"),
        ));
    }

    let optional = resolved_field_type(root);
    assert!(std::ptr::eq(optional_box_pointee(optional), root));
    assert!(format!("{root:?}").contains("DirectRecursiveNode"));
}

/// Verifies indirect recursive derives initialize without exposing partial
/// shapes and navigate back to the original root.
#[test]
fn test_derive_reflect_initializes_indirect_recursive_descriptors_without_deadlock() {
    const MARKER: &str = "QUBIT_REFLECT_INDIRECT_RECURSION_CHILD";
    if std::env::var_os(MARKER).is_none() {
        assert_isolated_test_completes(
            "descriptor::derive_struct_tests::test_derive_reflect_initializes_indirect_recursive_descriptors_without_deadlock",
            MARKER,
        );
        return;
    }

    let barrier = Arc::new(Barrier::new(17));
    let threads: Vec<_> = (0..16)
        .map(|index| {
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                if index % 2 == 0 {
                    let left = TypeDescriptor::of::<IndirectRecursiveLeft>();
                    let right = optional_box_pointee(resolved_field_type(left));
                    assert!(std::ptr::eq(optional_box_pointee(resolved_field_type(right)), left,));
                    left.type_id()
                } else {
                    let right = TypeDescriptor::of::<IndirectRecursiveRight>();
                    let left = optional_box_pointee(resolved_field_type(right));
                    assert!(std::ptr::eq(optional_box_pointee(resolved_field_type(left)), right,));
                    right.type_id()
                }
            })
        })
        .collect();
    barrier.wait();
    let left = TypeDescriptor::of::<IndirectRecursiveLeft>();
    let right = optional_box_pointee(resolved_field_type(left));
    let returned_left = optional_box_pointee(resolved_field_type(right));

    for worker in threads {
        let identity = worker.join().expect("indirect recursive worker should complete");
        assert!(identity == left.type_id() || identity == right.type_id());
    }

    assert!(std::ptr::eq(right, TypeDescriptor::of::<IndirectRecursiveRight>()));
    assert!(std::ptr::eq(returned_left, left));
    assert_eq!(left.fields().len(), 1);
    assert_eq!(right.fields().len(), 1);
}

/// Verifies a legal direct `Node -> Vec<Node>` relation initializes and
/// navigates without recursively waiting on the node root.
#[test]
fn test_derive_reflect_initializes_recursive_vector_descriptor_without_deadlock() {
    const MARKER: &str = "QUBIT_REFLECT_VECTOR_RECURSION_CHILD";
    if std::env::var_os(MARKER).is_none() {
        assert_isolated_test_completes(
            "descriptor::derive_struct_tests::test_derive_reflect_initializes_recursive_vector_descriptor_without_deadlock",
            MARKER,
        );
        return;
    }

    let root = TypeDescriptor::of::<RecursiveVectorNode>();
    let sequence = resolved_field_type(root);
    assert!(std::ptr::eq(sequence_element(sequence), root));
    assert_eq!(root.fields().len(), 1);
}

/// Verifies legal array, tuple, and map indirection paths can each navigate
/// back to the same recursive root without eager child initialization.
#[test]
fn test_derive_reflect_initializes_recursive_composite_descriptors_without_deadlock() {
    const MARKER: &str = "QUBIT_REFLECT_COMPOSITE_RECURSION_CHILD";
    if std::env::var_os(MARKER).is_none() {
        assert_isolated_test_completes(
            "descriptor::derive_struct_tests::test_derive_reflect_initializes_recursive_composite_descriptors_without_deadlock",
            MARKER,
        );
        return;
    }

    let root = TypeDescriptor::of::<RecursiveCompositeNode>();

    let array = resolved_field_type_at(root, 0)
        .as_array()
        .expect("array field should expose its typed view")
        .element_type()
        .as_resolved()
        .expect("array element should resolve to Box");
    let array_target = array
        .as_smart_pointer()
        .expect("array element should expose the Box view")
        .pointee_type()
        .as_resolved()
        .expect("array Box target should resolve to the root");

    let tuple_elements = resolved_field_type_at(root, 1)
        .as_tuple()
        .expect("tuple field should expose its typed view")
        .elements();
    let tuple_target = tuple_elements[0]
        .as_resolved()
        .expect("tuple element should resolve to Box")
        .as_smart_pointer()
        .expect("tuple element should expose the Box view")
        .pointee_type()
        .as_resolved()
        .expect("tuple Box target should resolve to the root");

    let map = resolved_field_type_at(root, 2)
        .as_map()
        .expect("map field should expose its typed view");
    let map_value = map
        .value_type()
        .as_resolved()
        .expect("map value should resolve to the root");

    assert!(std::ptr::eq(array_target, root));
    assert!(std::ptr::eq(tuple_target, root));
    assert!(std::ptr::eq(map_value, root));
    assert_eq!(root.fields().len(), 3);
}

/// Verifies the derive creates a root descriptor with direct named fields.
#[test]
fn test_derive_reflect_builds_named_struct_descriptor() {
    let descriptor = TypeDescriptor::of::<DerivedNamed>();

    assert_eq!(
        descriptor
            .as_struct()
            .expect("derived descriptor must expose a struct view")
            .kind(),
        StructKind::Named
    );
    let field = descriptor
        .field("value")
        .expect("derived named field must be discoverable");
    assert_eq!(field.index(), 0);
    assert_eq!(field.rust_name(), Some("value"));
}

/// Verifies a derived private field uses the safe generated access adapters.
#[test]
fn test_derive_reflect_accesses_private_named_field() {
    let field = TypeDescriptor::of::<DerivedNamed>()
        .field("value")
        .expect("derived field must be discoverable");
    let mut value = DerivedNamed { value: 7 };

    assert_eq!(
        field
            .get(ReflectedRef::new(&value))
            .expect("private field read must succeed")
            .downcast_ref::<u8>(),
        Some(&7)
    );
    field
        .set(ReflectedMut::new(&mut value), ReflectedOwned::new(9_u8))
        .expect("private field replacement must succeed");
    assert_eq!(value.value, 9);
}

/// Verifies tuple, newtype, and unit shapes retain their distinct descriptor
/// views.
#[test]
fn test_derive_reflect_preserves_non_named_struct_shapes() {
    let tuple = TypeDescriptor::of::<DerivedTuple>();
    let newtype = TypeDescriptor::of::<DerivedNewtype>();
    let unit = TypeDescriptor::of::<DerivedUnit>();

    assert_eq!(tuple.as_struct().expect("tuple view").kind(), StructKind::Tuple);
    assert_eq!(newtype.as_struct().expect("newtype view").kind(), StructKind::Newtype);
    assert_eq!(unit.as_struct().expect("unit view").kind(), StructKind::Unit);
    assert_eq!(tuple.field_at(0).expect("tuple field").rust_name(), None);
    assert_eq!(tuple.field_at(1).expect("tuple field").query_name(), None);
}

/// Verifies type-level opacity does not impose reflection bounds on its fields.
#[test]
fn test_derive_reflect_type_level_opaque_supports_unreflectable_generic_members() {
    let descriptor = TypeDescriptor::of::<DerivedOpaque<std::rc::Rc<()>>>();
    let opaque = DerivedOpaque {
        value: std::rc::Rc::new(()),
    };

    assert_eq!(descriptor.kind(), TypeKind::Opaque);
    assert!(descriptor.fields().is_empty());
    assert_eq!(std::rc::Rc::strong_count(&opaque.value), 1);
}

/// Verifies generic specializations receive distinct interned descriptors.
#[test]
fn test_derive_reflect_generic_struct_resolves_field_type_per_specialization() {
    let u8_descriptor = TypeDescriptor::of::<DerivedGeneric<u8>>();
    let u16_descriptor = TypeDescriptor::of::<DerivedGeneric<u16>>();

    assert_ne!(u8_descriptor.type_id(), u16_descriptor.type_id());
    assert!(std::ptr::eq(
        u8_descriptor
            .field_at(0)
            .expect("generic field")
            .field_type()
            .as_resolved()
            .expect("resolved field"),
        TypeDescriptor::of::<u8>(),
    ));
    assert!(std::ptr::eq(
        u16_descriptor
            .field_at(0)
            .expect("generic field")
            .field_type()
            .as_resolved()
            .expect("resolved field"),
        TypeDescriptor::of::<u16>(),
    ));
}

/// Verifies an opaque field does not require its generic argument to implement
/// `Reflect`.
#[test]
fn test_derive_reflect_opaque_generic_field_does_not_require_reflect_bound() {
    let descriptor = TypeDescriptor::of::<DerivedOpaqueMember<std::rc::Rc<()>>>();

    assert!(matches!(
        descriptor.field_at(0).expect("opaque field").field_type(),
        reflect::descriptor::TypeRef::Opaque(_)
    ));
}

/// Verifies a concrete derived root contributes a linker-discovered type
/// fragment.
#[test]
fn test_derive_reflect_registers_concrete_struct() {
    let registry = ReflectRegistry::initialize().expect("derived type fragments must validate");

    assert!(std::ptr::eq(
        registry
            .get(TypeDescriptor::of::<DerivedNamed>().type_id())
            .expect("derived struct must be registered"),
        TypeDescriptor::of::<DerivedNamed>(),
    ));
}

/// Verifies field policies retain source visibility and explicit query names.
#[test]
fn test_derive_reflect_honors_field_names_visibility_and_access_policies() {
    let descriptor = TypeDescriptor::of::<RenamedFields>();
    let payload = descriptor.field("payload").expect("renamed field");
    let skipped = descriptor.field_at(1).expect("skipped field");
    let values = RenamedFields {
        value: 1,
        skipped: "hidden".into(),
    };

    assert_eq!(descriptor.query_name(), "renamed-record");
    assert_eq!(payload.rust_name(), Some("value"));
    assert_eq!(payload.query_name(), Some("payload"));
    assert_eq!(
        payload.visibility().as_declared(),
        Some(&reflect::identity::Visibility::Crate)
    );
    assert_eq!(payload.access_policy(), reflect::access::FieldAccessPolicy::ReadOnly);
    assert_eq!(skipped.access_policy(), reflect::access::FieldAccessPolicy::Skipped);
    assert_eq!(values.skipped, "hidden");
}
