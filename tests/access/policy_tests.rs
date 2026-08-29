//! Integration tests for reflected field access policies.

use qubit_reflect::__private::descriptor;
use qubit_reflect::access::FieldAccessError;
use qubit_reflect::access::FieldAccessOperation;
use qubit_reflect::access::FieldAccessPolicy;
use qubit_reflect::descriptor::FieldDescriptor;
use qubit_reflect::descriptor::OpaqueTypeDescriptor;
use qubit_reflect::descriptor::StructKind;
use qubit_reflect::descriptor::TypeDescriptor;
use qubit_reflect::descriptor::TypeRef;
use qubit_reflect::identity::Visibility;
use qubit_reflect::value::ReflectedMut;
use qubit_reflect::value::ReflectedOwned;
use qubit_reflect::value::ReflectedRef;

struct PolicyRecord {
    read_only: u32,
    skipped: u32,
}

/// Returns the declaring descriptor used by policy test fields.
fn policy_record_descriptor() -> &'static TypeDescriptor {
    &POLICY_RECORD_DESCRIPTOR
}

/// Reads the read-only field after descriptor-level validation.
fn get_read_only<'a>(target: ReflectedRef<'a>) -> Result<ReflectedRef<'a>, FieldAccessError> {
    let record = target
        .downcast::<PolicyRecord>()
        .unwrap_or_else(|_| panic!("the descriptor must validate the adapter target type"));
    Ok(ReflectedRef::new(&record.read_only))
}

/// Mutably borrows the read-only field if policy validation is bypassed.
fn get_read_only_mut<'a>(target: ReflectedMut<'a>) -> Result<ReflectedMut<'a>, FieldAccessError> {
    let record = target
        .downcast::<PolicyRecord>()
        .unwrap_or_else(|_| panic!("the descriptor must validate the adapter target type"));
    Ok(ReflectedMut::new(&mut record.read_only))
}

/// Replaces the read-only field if policy validation is bypassed.
fn set_read_only(target: ReflectedMut<'_>, value: ReflectedOwned) -> Result<(), FieldAccessError> {
    let record = target
        .downcast::<PolicyRecord>()
        .unwrap_or_else(|_| panic!("the descriptor must validate the adapter target type"));
    let value = match value.downcast::<u32>() {
        Ok(value) => value,
        Err(_) => panic!("the descriptor must validate the adapter value type"),
    };
    record.read_only = value;
    Ok(())
}

/// Reads the skipped field if policy validation is bypassed.
fn get_skipped<'a>(target: ReflectedRef<'a>) -> Result<ReflectedRef<'a>, FieldAccessError> {
    let record = target
        .downcast::<PolicyRecord>()
        .unwrap_or_else(|_| panic!("the descriptor must validate the adapter target type"));
    Ok(ReflectedRef::new(&record.skipped))
}

static U32_TYPE: OpaqueTypeDescriptor = descriptor::opaque_member::<u32>();
static U32_TYPE_REF: TypeRef = TypeRef::Opaque(&U32_TYPE);
static POLICY_FIELDS: [FieldDescriptor; 2] = [
    descriptor::field(
        policy_record_descriptor,
        0,
        Some("read_only"),
        Some("read_only"),
        &U32_TYPE_REF,
        Visibility::Private,
    )
    .with_access(
        FieldAccessPolicy::ReadOnly,
        Some(get_read_only),
        Some(get_read_only_mut),
        Some(set_read_only),
    ),
    descriptor::field(
        policy_record_descriptor,
        1,
        Some("skipped"),
        Some("skipped"),
        &U32_TYPE_REF,
        Visibility::Private,
    )
    .with_access(FieldAccessPolicy::Skipped, Some(get_skipped), None, None),
];
static POLICY_RECORD_DESCRIPTOR: TypeDescriptor =
    descriptor::struct_type::<PolicyRecord>("policy_tests::PolicyRecord", StructKind::Named, &POLICY_FIELDS);

/// Verifies read-only fields remain readable and described while all mutable
/// entry points are rejected before their adapters can change the target.
#[test]
fn test_read_only_policy_rejects_mutation() {
    let field = &POLICY_FIELDS[0];
    let mut record = PolicyRecord {
        read_only: 7,
        skipped: 9,
    };

    assert_eq!(field.access_policy(), FieldAccessPolicy::ReadOnly);
    assert_eq!(
        field
            .get(ReflectedRef::new(&record))
            .expect("read-only fields should retain shared access")
            .downcast_ref::<u32>(),
        Some(&7)
    );

    let error = match field.get_mut(ReflectedMut::new(&mut record)) {
        Ok(_) => panic!("read-only fields must reject mutable borrows"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        FieldAccessError::ReadOnly {
            operation: FieldAccessOperation::GetMut,
            ..
        }
    ));

    let error = field
        .set(ReflectedMut::new(&mut record), ReflectedOwned::new(11_u32))
        .expect_err("read-only fields must reject replacement");
    assert!(matches!(
        error,
        FieldAccessError::ReadOnly {
            operation: FieldAccessOperation::Set,
            ..
        }
    ));
    assert_eq!(record.read_only, 7);
}

/// Verifies skipped fields retain their structural index and name but disable
/// every dynamic access path.
#[test]
fn test_skipped_policy_preserves_descriptor_and_disables_access() {
    let field = POLICY_RECORD_DESCRIPTOR
        .field_at(1)
        .expect("a skipped field must retain its source index");
    let mut record = PolicyRecord {
        read_only: 7,
        skipped: 9,
    };

    assert_eq!(field.rust_name(), Some("skipped"));
    assert_eq!(field.query_name(), Some("skipped"));
    assert_eq!(field.access_policy(), FieldAccessPolicy::Skipped);
    assert!(matches!(
        field.get(ReflectedRef::new(&record)),
        Err(FieldAccessError::Skipped {
            operation: FieldAccessOperation::Get,
            ..
        })
    ));
    assert!(matches!(
        field.get_mut(ReflectedMut::new(&mut record)),
        Err(FieldAccessError::Skipped {
            operation: FieldAccessOperation::GetMut,
            ..
        })
    ));
    assert!(matches!(
        field.set(ReflectedMut::new(&mut record), ReflectedOwned::new(12_u32)),
        Err(FieldAccessError::Skipped {
            operation: FieldAccessOperation::Set,
            ..
        })
    ));
    assert_eq!(record.skipped, 9);
}
