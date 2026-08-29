//! Integration tests for safe reflected field access.

use std::any::TypeId;

use qubit_reflect::access::FieldAccessError;
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

#[derive(Debug, Eq, PartialEq)]
struct Account {
    secret: String,
}

struct OtherTarget;

/// Returns the descriptor root used by the hand-written field adapter.
fn account_descriptor() -> &'static TypeDescriptor {
    &ACCOUNT_DESCRIPTOR
}

/// Reads the private field after descriptor-level target validation.
fn get_secret<'a>(target: ReflectedRef<'a>) -> Result<ReflectedRef<'a>, FieldAccessError> {
    let account = target
        .downcast::<Account>()
        .unwrap_or_else(|_| panic!("the descriptor must validate the adapter target type"));
    Ok(ReflectedRef::new(&account.secret))
}

/// Mutably borrows the private field after descriptor-level target validation.
fn get_secret_mut<'a>(target: ReflectedMut<'a>) -> Result<ReflectedMut<'a>, FieldAccessError> {
    let account = target
        .downcast::<Account>()
        .unwrap_or_else(|_| panic!("the descriptor must validate the adapter target type"));
    Ok(ReflectedMut::new(&mut account.secret))
}

/// Replaces the private field after descriptor-level target and value validation.
fn set_secret(target: ReflectedMut<'_>, value: ReflectedOwned) -> Result<(), FieldAccessError> {
    let account = target
        .downcast::<Account>()
        .unwrap_or_else(|_| panic!("the descriptor must validate the adapter target type"));
    let secret = match value.downcast::<String>() {
        Ok(secret) => secret,
        Err(_) => panic!("the descriptor must validate the adapter value type"),
    };
    account.secret = secret;
    Ok(())
}

static SECRET_TYPE: OpaqueTypeDescriptor =
    OpaqueTypeDescriptor::new::<String>("alloc::string::String");
static SECRET_TYPE_REF: TypeRef = TypeRef::Opaque(&SECRET_TYPE);
static ACCOUNT_FIELDS: [FieldDescriptor; 1] = [FieldDescriptor::new(
    account_descriptor,
    0,
    Some("secret"),
    Some("secret"),
    &SECRET_TYPE_REF,
    Visibility::Private,
)
.with_access(
    FieldAccessPolicy::ReadWrite,
    Some(get_secret),
    Some(get_secret_mut),
    Some(set_secret),
)];
static ACCOUNT_DESCRIPTOR: TypeDescriptor = TypeDescriptor::new_struct::<Account>(
    "field_tests::Account",
    "Account",
    StructKind::Named,
    &ACCOUNT_FIELDS,
);

/// Verifies shared, mutable, and owned replacement adapters can safely access a
/// private field while preserving the target borrow lifetime.
#[test]
fn test_field_descriptor_reads_and_writes_private_field() {
    let field = ACCOUNT_DESCRIPTOR
        .field("secret")
        .expect("the private field descriptor should remain queryable");
    let mut account = Account {
        secret: String::from("initial"),
    };

    let borrowed = field
        .get(ReflectedRef::new(&account))
        .expect("shared access should succeed");
    assert_eq!(
        borrowed.downcast_ref::<String>().map(String::as_str),
        Some("initial")
    );

    {
        let mut borrowed = field
            .get_mut(ReflectedMut::new(&mut account))
            .expect("mutable access should succeed");
        borrowed
            .downcast_mut::<String>()
            .expect("the field adapter should retain the exact field type")
            .push_str("-mutated");
    }
    assert_eq!(account.secret, "initial-mutated");

    field
        .set(
            ReflectedMut::new(&mut account),
            ReflectedOwned::new(String::from("replaced")),
        )
        .expect("whole-value replacement should succeed for an opaque field");
    assert_eq!(account.secret, "replaced");
}

/// Verifies descriptor validation rejects incorrect target and value types
/// before invoking a mutating adapter, so failed operations are atomic.
#[test]
fn test_field_descriptor_rejects_type_mismatches_without_modifying_target() {
    let field = &ACCOUNT_FIELDS[0];
    let other = OtherTarget;

    let error = match field.get(ReflectedRef::new(&other)) {
        Ok(_) => panic!("an unrelated target type must be rejected"),
        Err(error) => error,
    };
    let FieldAccessError::TargetTypeMismatch { mismatch, .. } = error else {
        panic!("the error should classify a target type mismatch");
    };
    assert_eq!(mismatch.expected(), TypeId::of::<Account>());
    assert_eq!(mismatch.actual(), TypeId::of::<OtherTarget>());

    let mut account = Account {
        secret: String::from("unchanged"),
    };
    let error = field
        .set(ReflectedMut::new(&mut account), ReflectedOwned::new(42_u32))
        .expect_err("a non-String replacement must be rejected");
    let FieldAccessError::ValueTypeMismatch { mismatch, .. } = error else {
        panic!("the error should classify a field value type mismatch");
    };
    assert_eq!(mismatch.expected(), TypeId::of::<String>());
    assert_eq!(mismatch.actual(), TypeId::of::<u32>());
    assert_eq!(account.secret, "unchanged");
}
