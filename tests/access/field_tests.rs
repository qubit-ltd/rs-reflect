// qubit-style: allow explicit-imports
//! Integration tests for safe reflected field access.
use std::any::TypeId;
use std::cell::Cell;
use std::rc::Rc;

use qubit_reflect as reflect;
use reflect::__private::descriptor;
use reflect::access::FieldAccessError;
use reflect::access::FieldAccessPolicy;
use reflect::descriptor::FieldDescriptor;
use reflect::descriptor::OpaqueTypeDescriptor;
use reflect::descriptor::StructKind;
use reflect::descriptor::TypeDescriptor;
use reflect::descriptor::TypeRef;
use reflect::expression::TypeExpression;
use reflect::identity::Visibility;
use reflect::value::ReflectedMut;
use reflect::value::ReflectedOwned;
use reflect::value::ReflectedRef;

#[derive(Debug, Eq, PartialEq)]
struct Account {
    secret: String,
}

struct OtherTarget;

struct RecoveryRecord {
    value: DropProbe,
}

/// Counts destruction so recovery tests can observe ownership precisely.
struct DropProbe {
    drops: Rc<Cell<usize>>,
}

impl Drop for DropProbe {
    /// Records destruction of the recovered replacement value.
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}

/// Returns the descriptor root used by the hand-written field adapter.
fn account_descriptor() -> &'static TypeDescriptor {
    &ACCOUNT_DESCRIPTOR
}

/// Returns the descriptor root used by field-set recovery tests.
fn recovery_record_descriptor() -> &'static TypeDescriptor {
    &RECOVERY_RECORD_DESCRIPTOR
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

/// Replaces the private field after descriptor-level target and value
/// validation.
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

/// Replaces the probe field after descriptor-level validation.
fn set_probe(target: ReflectedMut<'_>, value: ReflectedOwned) -> Result<(), FieldAccessError> {
    let record = target
        .downcast::<RecoveryRecord>()
        .unwrap_or_else(|_| panic!("the descriptor must validate the adapter target type"));
    let value = match value.downcast::<DropProbe>() {
        Ok(value) => value,
        Err(_) => panic!("the descriptor must validate the adapter value type"),
    };
    record.value = value;
    Ok(())
}

static SECRET_TYPE: OpaqueTypeDescriptor = descriptor::opaque_member::<String>();
static SECRET_TYPE_REF: TypeRef = TypeRef::Opaque(&SECRET_TYPE);
static ACCOUNT_FIELDS: [FieldDescriptor; 1] = [descriptor::field(
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
static ACCOUNT_DESCRIPTOR: TypeDescriptor =
    descriptor::struct_type::<Account>("field_tests::Account", StructKind::Named, &ACCOUNT_FIELDS);
static DROP_PROBE_TYPE: OpaqueTypeDescriptor = descriptor::opaque_member::<DropProbe>();
static DROP_PROBE_TYPE_REF: TypeRef = TypeRef::Opaque(&DROP_PROBE_TYPE);
static SYMBOLIC_TYPE_REF: TypeRef = TypeRef::Symbolic(TypeExpression::SelfType);
static RECOVERY_FIELDS: [FieldDescriptor; 5] = [
    descriptor::field(
        recovery_record_descriptor,
        0,
        Some("value"),
        Some("value"),
        &DROP_PROBE_TYPE_REF,
        Visibility::Private,
    )
    .with_access(FieldAccessPolicy::ReadWrite, None, None, Some(set_probe)),
    descriptor::field(
        recovery_record_descriptor,
        0,
        Some("value"),
        Some("value"),
        &DROP_PROBE_TYPE_REF,
        Visibility::Private,
    )
    .with_access(FieldAccessPolicy::ReadOnly, None, None, Some(set_probe)),
    descriptor::field(
        recovery_record_descriptor,
        0,
        Some("value"),
        Some("value"),
        &DROP_PROBE_TYPE_REF,
        Visibility::Private,
    ),
    descriptor::field(
        recovery_record_descriptor,
        0,
        Some("value"),
        Some("value"),
        &DROP_PROBE_TYPE_REF,
        Visibility::Private,
    )
    .with_access(FieldAccessPolicy::Skipped, None, None, None),
    descriptor::field(
        recovery_record_descriptor,
        0,
        Some("value"),
        Some("value"),
        &SYMBOLIC_TYPE_REF,
        Visibility::Private,
    )
    .with_access(FieldAccessPolicy::ReadWrite, None, None, Some(set_probe)),
];
static RECOVERY_RECORD_DESCRIPTOR: TypeDescriptor =
    descriptor::struct_type::<RecoveryRecord>("field_tests::RecoveryRecord", StructKind::Named, &RECOVERY_FIELDS);

/// Creates a replacement probe and its shared destructor counter.
fn create_drop_probe() -> (Rc<Cell<usize>>, ReflectedOwned) {
    let drops = Rc::new(Cell::new(0));
    let value = ReflectedOwned::new(DropProbe {
        drops: Rc::clone(&drops),
    });
    (drops, value)
}

/// Extracts and destroys a named recovered probe after checking it was kept
/// alive throughout error inspection.
fn assert_named_probe_recovered(failure: reflect::access::FieldSetFailure, drops: &Rc<Cell<usize>>, name: &str) {
    assert_eq!(drops.get(), 0, "the failed set must retain the replacement");
    assert!(
        failure
            .recovery()
            .expect("pre-execution failure must contain recovery")
            .value_by_name(name)
            .is_some()
    );
    let recovery = failure
        .into_recovery()
        .unwrap_or_else(|_| panic!("pre-execution failure must contain recovery"));
    let value = recovery
        .into_value_by_name(name)
        .unwrap_or_else(|_| panic!("the original named value must be recoverable"));
    let probe = value
        .downcast::<DropProbe>()
        .unwrap_or_else(|_| panic!("recovery must preserve the exact replacement type"));
    assert_eq!(drops.get(), 0, "taking recovery must not destroy the value");
    drop(probe);
    assert_eq!(drops.get(), 1, "the caller should control final destruction");
}

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
    assert_eq!(borrowed.downcast_ref::<String>().map(String::as_str), Some("initial"));

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
    let FieldAccessError::ValueTypeMismatch { mismatch, .. } = error.error() else {
        panic!("the error should classify a field value type mismatch");
    };
    assert_eq!(mismatch.expected(), TypeId::of::<String>());
    assert_eq!(mismatch.actual(), TypeId::of::<u32>());
    assert_eq!(account.secret, "unchanged");
}

/// Verifies a target-type mismatch returns the untouched replacement before
/// any generated adapter runs.
#[test]
fn test_field_set_target_mismatch_recovers_replacement_without_dropping() {
    let field = &RECOVERY_FIELDS[0];
    let mut target = OtherTarget;
    let (drops, value) = create_drop_probe();

    let failure = field
        .set(ReflectedMut::new(&mut target), value)
        .expect_err("an unrelated target must be rejected");

    assert!(matches!(failure.error(), FieldAccessError::TargetTypeMismatch { .. }));
    assert_named_probe_recovered(failure, &drops, "value");
}

/// Verifies read-only policy rejection returns the untouched replacement.
#[test]
fn test_field_set_read_only_recovers_replacement_without_dropping() {
    let field = &RECOVERY_FIELDS[1];
    let drops_in_target = Rc::new(Cell::new(0));
    let mut target = RecoveryRecord {
        value: DropProbe {
            drops: Rc::clone(&drops_in_target),
        },
    };
    let (drops, value) = create_drop_probe();

    let failure = field
        .set(ReflectedMut::new(&mut target), value)
        .expect_err("a read-only field must reject replacement");

    assert!(matches!(failure.error(), FieldAccessError::ReadOnly { .. }));
    assert_named_probe_recovered(failure, &drops, "value");
    drop(target);
    assert_eq!(drops_in_target.get(), 1);
}

/// Verifies an exact field-value mismatch returns the untouched replacement.
#[test]
fn test_field_set_value_mismatch_recovers_replacement_without_dropping() {
    let field = &ACCOUNT_FIELDS[0];
    let mut target = Account {
        secret: String::from("unchanged"),
    };
    let (drops, value) = create_drop_probe();

    let failure = field
        .set(ReflectedMut::new(&mut target), value)
        .expect_err("a non-String replacement must be rejected");

    assert!(matches!(failure.error(), FieldAccessError::ValueTypeMismatch { .. }));
    assert_eq!(target.secret, "unchanged");
    assert_named_probe_recovered(failure, &drops, "secret");
}

/// Verifies a missing set adapter returns recovery by source index.
#[test]
fn test_field_set_missing_adapter_recovers_replacement_without_dropping() {
    let field = &RECOVERY_FIELDS[2];
    let drops_in_target = Rc::new(Cell::new(0));
    let mut target = RecoveryRecord {
        value: DropProbe {
            drops: Rc::clone(&drops_in_target),
        },
    };
    let (drops, value) = create_drop_probe();

    let failure = field
        .set(ReflectedMut::new(&mut target), value)
        .expect_err("a descriptor without a set adapter must reject replacement");

    assert!(matches!(failure.error(), FieldAccessError::Unavailable { .. }));
    assert_eq!(drops.get(), 0);
    assert!(
        failure
            .recovery()
            .expect("pre-execution failure must contain recovery")
            .value_at(0)
            .is_some()
    );
    let value = failure
        .into_recovery()
        .unwrap_or_else(|_| panic!("pre-execution failure must contain recovery"))
        .into_value_at(0)
        .unwrap_or_else(|_| panic!("the original indexed value must be recoverable"));
    let probe = value
        .downcast::<DropProbe>()
        .unwrap_or_else(|_| panic!("recovery must preserve the exact replacement type"));
    assert_eq!(drops.get(), 0);
    drop(probe);
    assert_eq!(drops.get(), 1);
    drop(target);
    assert_eq!(drops_in_target.get(), 1);
}

/// Verifies skipped policy returns the untouched replacement without running
/// or requiring a set adapter.
#[test]
fn test_field_set_skipped_recovers_replacement_without_dropping() {
    let field = &RECOVERY_FIELDS[3];
    let drops_in_target = Rc::new(Cell::new(0));
    let mut target = RecoveryRecord {
        value: DropProbe {
            drops: Rc::clone(&drops_in_target),
        },
    };
    let (drops, value) = create_drop_probe();

    let failure = field
        .set(ReflectedMut::new(&mut target), value)
        .expect_err("a skipped field must reject replacement");

    assert!(matches!(failure.error(), FieldAccessError::Skipped { .. }));
    assert_named_probe_recovered(failure, &drops, "value");
    drop(target);
    assert_eq!(drops_in_target.get(), 1);
}

/// Verifies a symbolic field type returns the untouched replacement because it
/// has no exact runtime identity for safe whole-value replacement.
#[test]
fn test_field_set_symbolic_type_recovers_replacement_without_dropping() {
    let field = &RECOVERY_FIELDS[4];
    let drops_in_target = Rc::new(Cell::new(0));
    let mut target = RecoveryRecord {
        value: DropProbe {
            drops: Rc::clone(&drops_in_target),
        },
    };
    let (drops, value) = create_drop_probe();

    let failure = field
        .set(ReflectedMut::new(&mut target), value)
        .expect_err("a symbolic field must reject replacement");

    assert!(matches!(failure.error(), FieldAccessError::Unavailable { .. }));
    assert_named_probe_recovered(failure, &drops, "value");
    drop(target);
    assert_eq!(drops_in_target.get(), 1);
}
