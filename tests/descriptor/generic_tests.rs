//! Integration tests for concrete generic reflection instances.

use std::sync::Arc;

use qubit_reflect::{Reflect, TypeDescriptor};

#[derive(Reflect)]
struct GenericRecord<T, const N: usize> {
    values: [T; N],
}

#[derive(Reflect)]
#[reflect(opaque)]
struct OpaqueGenericRecord<T, const N: usize> {
    values: [T; N],
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
}

/// Verifies opaque generic members retain only the static bound needed by roots.
#[test]
fn test_opaque_generic_instances_do_not_require_reflect_arguments() {
    let descriptor = TypeDescriptor::of::<OpaqueGenericRecord<Arc<()>, 2>>();
    let value = OpaqueGenericRecord {
        values: [Arc::new(()), Arc::new(())],
    };

    assert!(descriptor.as_opaque().is_some());
    assert_eq!(value.values.len(), 2);
}
