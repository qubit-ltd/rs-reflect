// qubit-style: allow explicit-imports
//! Integration tests for concrete generic reflection instances.
use std::sync::Arc;

use qubit_reflect as reflect;
use reflect::Reflect;
use reflect::TypeDescriptor;

#[derive(Reflect)]
struct GenericRecord<T, const N: usize> {
    values: [T; N],
}

#[derive(Reflect)]
#[reflect(opaque)]
struct OpaqueGenericRecord<T, const N: usize> {
    values: [T; N],
}

#[derive(Reflect)]
struct Borrowed<'a> {
    value: &'a str,
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
    assert_eq!(generic.definition().parameters.len(), 2);
    assert_eq!(generic.arguments().len(), 2);
}

/// Verifies opaque generic members retain only the static bound needed by
/// roots.
#[test]
fn test_opaque_generic_instances_do_not_require_reflect_arguments() {
    let descriptor = TypeDescriptor::of::<OpaqueGenericRecord<Arc<()>, 2>>();
    let value = OpaqueGenericRecord {
        values: [Arc::new(()), Arc::new(())],
    };

    assert!(descriptor.as_opaque().is_some());
    assert_eq!(value.values.len(), 2);
}

#[test]
fn test_static_lifetime_generic_root_preserves_definition_without_runtime_lifetime_argument() {
    let descriptor = TypeDescriptor::of::<Borrowed<'static>>();
    let generic = descriptor.concrete_generic().expect("generic definitions are retained");
    assert_eq!(generic.definition().parameters.len(), 1);
    assert!(generic.arguments().is_empty());
    assert_eq!(Borrowed { value: "static" }.value, "static");
}
