// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Focused crate-unit coverage for small public accessors that otherwise only
//! appear in separately instrumented integration-test code generation units.

use std::any::TypeId;
use std::sync::LazyLock;

use crate::access::FieldIdentity;
use crate::descriptor::ConcreteGenericDescriptor;
use crate::error::RegistryError;
use crate::error::TypeMismatch;
use crate::expression::DiagnosticText;
use crate::expression::GenericDefinitionDescriptor;
use crate::identity::FragmentIdentity;
use crate::identity::MemberId;
use crate::invoke::InvocationArg;
use crate::invoke::InvocationBinding;
use crate::value::DynamicOwned;
use crate::value::Local;

static EMPTY_GENERIC: LazyLock<GenericDefinitionDescriptor> = LazyLock::new(|| GenericDefinitionDescriptor {
    parameters: Box::new([]),
    predicates: Box::new([]),
    diagnostic: DiagnosticText::default(),
});

fn fragment(fingerprint: u64) -> FragmentIdentity {
    FragmentIdentity::new("crate", "crate::module", 10, 4, "field", fingerprint)
}

#[test]
fn test_small_identity_and_error_accessors_preserve_input_facts() {
    let identity = fragment(17);
    assert_eq!(identity.content_fingerprint(), 17);
    let member = MemberId::new("Type", "field", 1, identity.clone());
    assert_eq!(member.fragment(), &identity);

    let direct = FieldIdentity::new(TypeId::of::<u8>(), "u8", 0, Some("value"));
    assert_eq!(direct.rust_name(), Some("value"));
    let variant = FieldIdentity::new_variant(TypeId::of::<u8>(), "u8", 0, None, 2, "Ready");
    assert_eq!(variant.variant_rust_name(), Some("Ready"));

    let mismatch = TypeMismatch::new(TypeId::of::<u8>(), TypeId::of::<u16>()).with_diagnostic_names("u8", "u16");
    assert_eq!(mismatch.expected_name(), Some("u8"));

    let conflict = RegistryError::duplicate_fragment(fragment(1), fragment(2));
    assert!(conflict.conflicting_fragments().is_some());
}

#[test]
fn test_small_generic_and_invocation_accessors_preserve_input_facts() {
    let diagnostic = DiagnosticText::from(String::from("source"));
    assert_eq!(diagnostic.0.as_deref(), Some("source"));

    let generic = ConcreteGenericDescriptor::new(&EMPTY_GENERIC, &[]);
    assert!(std::ptr::eq(generic.definition(), &*EMPTY_GENERIC));
    assert!(generic.arguments().is_empty());

    let binding = InvocationBinding::<Local>::named("value", InvocationArg::Owned(DynamicOwned::<Local>::new(3_u8)));
    assert_eq!(binding.name(), Some("value"));
    assert!(matches!(binding.argument(), InvocationArg::Owned(_)));
}
