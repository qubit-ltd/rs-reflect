// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Tests construction diagnostics that require crate-private field identity
//! creation.

use std::any::TypeId;

use crate::__private::descriptor;
use crate::construct::ConstructionError;
use crate::construct::ConstructionFieldId;
use crate::construct::ConstructionShape;
use crate::construct::ConstructionUnavailableReason;
use crate::descriptor::FieldDescriptor;
use crate::descriptor::OpaqueTypeDescriptor;
use crate::descriptor::StructKind;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeRef;
use crate::error::TypeMismatch;
use crate::identity::Visibility;

struct ConstructionErrorFixture;

/// Returns the fixture root used by static field descriptors.
fn fixture_descriptor() -> &'static TypeDescriptor {
    &FIXTURE_DESCRIPTOR
}

static OPAQUE_VALUE: OpaqueTypeDescriptor = descriptor::opaque_member::<u8>();
static OPAQUE_VALUE_REF: TypeRef = TypeRef::Opaque(&OPAQUE_VALUE);
pub(super) static FIXTURE_FIELDS: [FieldDescriptor; 4] = [
    descriptor::field(
        fixture_descriptor,
        0,
        Some("named"),
        Some("renamed"),
        &OPAQUE_VALUE_REF,
        Visibility::Private,
    ),
    descriptor::field(
        fixture_descriptor,
        1,
        None,
        None,
        &OPAQUE_VALUE_REF,
        Visibility::Private,
    ),
    descriptor::field(
        fixture_descriptor,
        0,
        Some("named"),
        Some("named"),
        &OPAQUE_VALUE_REF,
        Visibility::Private,
    )
    .with_variant(2, "Data"),
    descriptor::field(
        fixture_descriptor,
        1,
        None,
        None,
        &OPAQUE_VALUE_REF,
        Visibility::Private,
    )
    .with_variant(2, "Data"),
];
static FIXTURE_DESCRIPTOR: TypeDescriptor =
    descriptor::struct_type::<ConstructionErrorFixture>("ConstructionErrorFixture", StructKind::Named, &FIXTURE_FIELDS);

/// Verifies construction field IDs retain every source-form component and
/// format direct and variant fields distinctly.
#[test]
fn test_construction_field_id_preserves_source_identity_forms() {
    let ids: Vec<_> = FIXTURE_FIELDS
        .iter()
        .map(ConstructionFieldId::from_descriptor)
        .collect();

    assert_eq!(ids[0].declaring_type(), TypeId::of::<ConstructionErrorFixture>());
    assert_eq!(
        ids[0].declaring_type_name(),
        std::any::type_name::<ConstructionErrorFixture>()
    );
    assert_eq!(ids[0].index(), 0);
    assert_eq!(ids[0].rust_name(), Some("named"));
    assert_eq!(ids[0].query_name(), Some("renamed"));
    assert_eq!(ids[0].variant_index(), None);
    assert_eq!(ids[0].variant_rust_name(), None);
    assert!(ids[0].to_string().ends_with("::named"));
    assert!(ids[1].to_string().ends_with(" field #1"));
    assert!(ids[2].to_string().ends_with("::Data.named"));
    assert!(ids[3].to_string().ends_with("::Data field #1"));
    assert_eq!(ids[2].variant_index(), Some(2));
    assert_eq!(ids[2].variant_rust_name(), Some("Data"));
}

/// Verifies every construction error and policy reason has complete stable
/// context when formatted.
#[test]
fn test_construction_error_variants_format_complete_context() {
    let field = Box::new(ConstructionFieldId::from_descriptor(&FIXTURE_FIELDS[0]));
    let mismatch = TypeMismatch::new(TypeId::of::<u8>(), TypeId::of::<u16>());
    let errors = [
        ConstructionError::TargetUnavailable,
        ConstructionError::WrongShape {
            expected: ConstructionShape::Named,
            actual: ConstructionShape::Tuple,
        },
        ConstructionError::UnknownField { name: "unknown".into() },
        ConstructionError::DuplicateField {
            name: "duplicate".into(),
        },
        ConstructionError::MissingField { field: field.clone() },
        ConstructionError::UnknownPosition { index: 4 },
        ConstructionError::ValueTypeMismatch {
            field: field.clone(),
            mismatch,
        },
        ConstructionError::BaseTypeMismatch { mismatch },
        ConstructionError::Unavailable {
            field,
            reason: ConstructionUnavailableReason::MissingDefaultProvider,
        },
    ];

    for error in errors {
        assert!(!error.to_string().is_empty());
    }
    assert_eq!(ConstructionShape::Named.to_string(), "named");
    assert_eq!(ConstructionShape::Tuple.to_string(), "tuple");
    assert_eq!(ConstructionShape::Unit.to_string(), "unit");
    assert_eq!(
        ConstructionUnavailableReason::MissingDefaultProvider.to_string(),
        "missing explicit default provider",
    );
    assert_eq!(
        ConstructionUnavailableReason::SymbolicFieldType.to_string(),
        "field type is not concrete",
    );
    assert_eq!(
        ConstructionUnavailableReason::CallerValueForbidden.to_string(),
        "field value must come from its generated provider",
    );
    assert_eq!(
        ConstructionUnavailableReason::UpdateForbidden.to_string(),
        "field update is unavailable",
    );
}
