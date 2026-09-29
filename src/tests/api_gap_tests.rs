// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Crate-internal coverage for contracts that require private implementation
//! access.

use std::any::TypeId;
use std::error::Error;

use crate::access::FieldAccessError;
use crate::access::FieldAccessOperation;
use crate::access::FieldIdentity;
use crate::access::FieldSetFailure;
use crate::capability::CapabilityConflictKind;
use crate::capability::TypeCapabilities;
use crate::capability::clone_descriptor;
use crate::capability::clone_key;
use crate::capability::default_descriptor;
use crate::capability::default_key;
use crate::capability::send_descriptor;
use crate::capability::send_key;
use crate::capability::sync_descriptor;
use crate::capability::sync_key;
use crate::identity::FragmentIdentity;
use crate::value::DynamicMut;
use crate::value::DynamicOwned;
use crate::value::DynamicRef;
use crate::value::Local;

fn rejected_field_value(value: u8) -> FieldSetFailure {
    let field = FieldIdentity::new(TypeId::of::<u16>(), "u16", 3, Some("value"));
    FieldSetFailure::before_execution(
        FieldAccessError::Unavailable {
            field: field.clone(),
            operation: FieldAccessOperation::Set,
        },
        field,
        Some("value"),
        crate::value::ReflectedOwned::new(value),
    )
}

fn owned_u8(value: crate::value::ReflectedOwned) -> u8 {
    match value.downcast::<u8>() {
        Ok(value) => value,
        Err(_) => panic!("test recovery must preserve the original u8"),
    }
}

#[test]
fn test_field_failure_recovery_preserves_identity_phase_and_owned_value() {
    let direct = FieldIdentity::new(TypeId::of::<u16>(), "u16", 3, Some("value"));
    let failure = rejected_field_value(7);
    assert_eq!(failure.error().field(), &direct);
    let recovery = failure.recovery().expect("validation failures retain input");
    assert_eq!(recovery.field(), &direct);
    assert_eq!(recovery.query_name(), Some("value"));
    assert_eq!(recovery.value().downcast_ref::<u8>(), Some(&7));
    assert_eq!(
        recovery
            .value_by_name("value")
            .and_then(|value| value.downcast_ref::<u8>()),
        Some(&7)
    );
    assert_eq!(
        recovery.value_at(3).and_then(|value| value.downcast_ref::<u8>()),
        Some(&7)
    );
    assert!(recovery.value_by_name("other").is_none());
    assert!(recovery.value_at(4).is_none());
    assert!(format!("{failure:?}").contains("recovery"));
    assert_eq!(failure.to_string(), failure.error().to_string());
    assert!(failure.source().is_some());
    assert_eq!(AsRef::<FieldAccessError>::as_ref(&failure), failure.error());

    let (error, recovery) = rejected_field_value(8).into_parts();
    assert_eq!(error.field(), &direct);
    assert_eq!(
        owned_u8(recovery.expect("recovery must survive decomposition").into_value()),
        8
    );
    let recovery = rejected_field_value(9)
        .into_recovery()
        .expect("validation failure retains recovery");
    let recovery = match recovery.into_value_by_name("other") {
        Ok(_) => panic!("a wrong name must retain recovery"),
        Err(recovery) => recovery,
    };
    assert_eq!(
        owned_u8(
            recovery
                .into_value_by_name("value")
                .expect("matching name extracts value")
        ),
        9
    );
    let recovery = rejected_field_value(10)
        .into_recovery()
        .expect("validation failure retains recovery");
    let recovery = match recovery.into_value_at(4) {
        Ok(_) => panic!("a wrong index must retain recovery"),
        Err(recovery) => recovery,
    };
    assert_eq!(
        owned_u8(recovery.into_value_at(3).expect("matching index extracts value")),
        10
    );

    let adapter_error = FieldAccessError::Unavailable {
        field: direct,
        operation: FieldAccessOperation::Set,
    };
    let failure: FieldSetFailure<crate::value::Local> = FieldSetFailure::after_execution(adapter_error.clone());
    assert!(failure.recovery().is_none());
    match failure.into_recovery() {
        Ok(_) => panic!("an adapter failure must not synthesize recovery"),
        Err(error) => assert_eq!(error, adapter_error),
    }
    assert_eq!(
        FieldSetFailure::<crate::value::Local>::after_execution(adapter_error.clone()).into_error(),
        adapter_error
    );
}

#[test]
fn test_dynamic_type_probes_and_capability_adapters_enforce_exact_contracts() {
    let shared = 7_u8;
    assert_eq!(
        crate::access::field_adapter::dynamic_ref_type_id(&DynamicRef::<Local>::new(&shared)),
        TypeId::of::<u8>(),
    );
    let mut mutable = 8_u16;
    assert_eq!(
        crate::access::field_adapter::dynamic_mut_type_id(&DynamicMut::<Local>::new(&mut mutable)),
        TypeId::of::<u16>(),
    );
    let owned = DynamicOwned::<Local>::new(9_u32);
    assert_eq!(
        crate::access::field_adapter::dynamic_owned_type_id(&owned),
        TypeId::of::<u32>(),
    );

    let capabilities = TypeCapabilities::try_new(vec![
        default_descriptor::<String>(),
        send_descriptor::<String>(),
        clone_descriptor::<String>(),
        sync_descriptor::<String>(),
    ])
    .expect("distinct built-in capabilities form a valid set");
    assert!(capabilities.contains(send_key()));
    assert!(capabilities.contains(sync_key()));
    let clone_adapter = capabilities
        .get(clone_key())
        .unwrap()
        .expect("clone adapter is registered");
    let source = DynamicOwned::<Local>::new(String::from("value"));
    let cloned = clone_adapter
        .clone_owned(&source)
        .expect("exact source type can be cloned");
    assert_eq!(cloned.downcast_ref::<String>().map(String::as_str), Some("value"));
    assert!(clone_adapter.clone_owned(&DynamicOwned::<Local>::new(1_u8)).is_err());
    let defaulted = capabilities
        .get(default_key())
        .unwrap()
        .expect("default adapter is registered")
        .create();
    assert_eq!(defaulted.downcast_ref::<String>().map(String::as_str), Some(""));
    assert!(format!("{:?}", capabilities.descriptors()[0]).contains("CapabilityDescriptor"));

    let duplicate = TypeCapabilities::try_new(vec![send_descriptor::<u8>(), send_descriptor::<u16>()])
        .expect_err("one stable capability ID cannot be declared twice");
    assert_eq!(duplicate.kind(), CapabilityConflictKind::DuplicateId);
    assert_eq!(duplicate.id(), send_key().id());
    assert_eq!(duplicate.first_adapter_type(), TypeId::of::<()>());
    assert_eq!(duplicate.second_adapter_type(), TypeId::of::<()>());
}

#[test]
fn test_fragment_same_source_identity_ignores_content_fingerprint() {
    let original = FragmentIdentity::new("crate", "crate::module", 10, 4, "field", 17);
    let changed_content = FragmentIdentity::new("crate", "crate::module", 10, 4, "field", 18);

    assert!(original.same_source_identity(&changed_content));
}
