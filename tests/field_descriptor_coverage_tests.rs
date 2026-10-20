// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use std::any::TypeId;

use qubit_reflect::Reflect;
use qubit_reflect::ReflectionContext;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::FieldAccessError;
use qubit_reflect::descriptor::FieldAccessUnavailableReason;
use qubit_reflect::descriptor::FieldSetError;
use qubit_reflect::descriptor::Visibility;
use qubit_reflect::error::ReflectionError;
use qubit_reflect::value::DynamicMut;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::DynamicRef;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

struct OpaqueValue(&'static str);

#[Reflect]
struct FieldOwner {
    pub count: u32,
    pub opaque: OpaqueValue,
    private_label: String,
}

#[Reflect]
struct ThreadSafeOwner {
    pub count: u32,
}

#[Reflect]
struct WrongOwner;

fn owner() -> FieldOwner {
    FieldOwner {
        count: 11,
        opaque: OpaqueValue("opaque"),
        private_label: String::from("hidden"),
    }
}

#[test]
fn field_descriptor_exposes_context_type_expression_and_concrete_lookup() {
    let context = ReflectionContext::builder()
        .include_type::<FieldOwner>()
        .include_type::<u32>()
        .build()
        .expect("the context contains the owner and reflected field type");
    let descriptor = TypeDescriptor::of_in::<FieldOwner>(&context);
    let count = descriptor.field("count").expect("public field exists");
    let opaque = descriptor.field("opaque").expect("opaque field is retained");

    assert!(std::ptr::eq(count.context(), &context));
    assert_eq!(count.declaring_type().type_id(), TypeId::of::<FieldOwner>());
    assert_eq!(count.type_expression().display().to_string(), "u32");
    assert_eq!(count.find_concrete_type().unwrap().type_id(), TypeId::of::<u32>());
    assert_eq!(count.concrete_type().unwrap().type_id(), TypeId::of::<u32>());
    assert!(opaque.find_concrete_type().is_none());
    assert!(matches!(opaque.concrete_type(), Err(ReflectionError::Lookup(_))));
    assert!(opaque.type_expression().display().to_string().contains("OpaqueValue"));
}

#[test]
fn field_visibility_and_dynamic_access_capabilities_match_metadata() {
    let descriptor = TypeDescriptor::of::<FieldOwner>();
    let public = descriptor.field("count").expect("public field exists");
    let private = descriptor.field("private_label").expect("private field exists");
    let opaque = descriptor.field("opaque").expect("opaque field exists");

    assert_eq!(public.visibility(), Visibility::Public);
    assert_eq!(private.visibility(), Visibility::Private);
    assert_eq!(opaque.visibility(), Visibility::Public);
    assert!(public.access().readable);
    assert!(public.access().mutable);
    assert!(public.access().replaceable);
    assert!(public.access().constructible);
    assert!(!opaque.access().readable);
    assert!(!opaque.access().mutable);
    assert!(!opaque.access().replaceable);
    assert!(!opaque.access().constructible);

    let target = owner();
    assert_eq!(target.opaque.0, "opaque");
    assert!(matches!(
        private.get_dynamic(DynamicRef::<Local>::new(&target)),
        Err(FieldAccessError::NotPublic)
    ));
}

#[test]
fn dynamic_field_adapter_errors_preserve_the_rejected_value() {
    let field = TypeDescriptor::of::<FieldOwner>()
        .field("count")
        .expect("public field exists");
    let wrong_owner = WrongOwner;

    assert!(matches!(
        field.get_dynamic(DynamicRef::<Local>::new(&wrong_owner)),
        Err(FieldAccessError::TargetTypeMismatch(_))
    ));
    assert!(matches!(
        field.get_dynamic_mut(DynamicMut::<Local>::new(&mut { WrongOwner })),
        Err(FieldAccessError::TargetTypeMismatch(_))
    ));

    let failure = field
        .set_dynamic(
            DynamicMut::<Local>::new(&mut { WrongOwner }),
            DynamicOwned::<Local>::new(17_u32),
        )
        .expect_err("dynamic writes validate the declaring type");
    assert!(matches!(failure.error(), FieldSetError::TargetTypeMismatch(_)));
    assert_eq!(failure.replacement().downcast_ref::<u32>(), Some(&17));

    let mut target = owner();
    let failure = field
        .set_dynamic(
            DynamicMut::<Local>::new(&mut target),
            DynamicOwned::<Local>::new(String::from("wrong field type")),
        )
        .expect_err("dynamic writes validate the field type");
    assert!(matches!(failure.error(), FieldSetError::ValueTypeMismatch(_)));
    assert_eq!(
        failure.replacement().downcast_ref::<String>().map(String::as_str),
        Some("wrong field type")
    );
    assert_eq!(target.count, 11);
}

#[test]
fn thread_safe_dynamic_field_supports_mutation_and_replacement() {
    let field = TypeDescriptor::of::<ThreadSafeOwner>()
        .field("count")
        .expect("thread-safe owner field exists");
    let mut target = ThreadSafeOwner { count: 13 };

    *field
        .get_dynamic_mut(DynamicMut::<ThreadSafe>::new(&mut target))
        .expect("thread-safe adapter supports mutable access")
        .downcast_mut::<u32>()
        .expect("the field has type u32") = 14;
    field
        .set_dynamic(
            DynamicMut::<ThreadSafe>::new(&mut target),
            DynamicOwned::<ThreadSafe>::new(15_u32),
        )
        .expect("thread-safe adapter supports replacement");
    assert_eq!(target.count, 15);
}

#[test]
fn missing_dynamic_adapter_reports_reason_and_returns_replacement() {
    let field = TypeDescriptor::of::<FieldOwner>()
        .field("opaque")
        .expect("opaque field exists");
    let mut target = owner();

    assert!(matches!(
        field.get_dynamic(DynamicRef::<Local>::new(&target)),
        Err(FieldAccessError::Unavailable(
            FieldAccessUnavailableReason::AdapterUnavailable
        ))
    ));
    assert!(matches!(
        field.get_dynamic_mut(DynamicMut::<Local>::new(&mut target)),
        Err(FieldAccessError::Unavailable(
            FieldAccessUnavailableReason::AdapterUnavailable
        ))
    ));
    let failure = field
        .set_dynamic(
            DynamicMut::<Local>::new(&mut target),
            DynamicOwned::<Local>::new(String::from("recovered")),
        )
        .expect_err("an opaque field has no dynamic adapter");
    assert!(matches!(
        failure.error(),
        FieldSetError::Unavailable(FieldAccessUnavailableReason::AdapterUnavailable)
    ));
    let (_, recovered) = failure.into_parts();
    assert_eq!(recovered.downcast::<String>().unwrap(), "recovered");
    assert_eq!(target.opaque.0, "opaque");
}

#[test]
fn typed_access_rejects_value_mismatches_and_thread_safe_dynamic_access_works() {
    let field = TypeDescriptor::of::<FieldOwner>()
        .field("count")
        .expect("public field exists");
    let target = owner();
    let error = field
        .get::<_, String>(&target)
        .expect_err("typed reads require the declared field type");
    let FieldAccessError::ValueTypeMismatch(mismatch) = error else {
        panic!("the failure must identify a field value type mismatch");
    };
    assert_eq!(mismatch.expected(), TypeId::of::<u32>());
    assert_eq!(mismatch.actual(), TypeId::of::<String>());

    let error = field
        .get::<_, u32>(&WrongOwner)
        .expect_err("typed reads require the declaring owner type");
    assert!(matches!(error, FieldAccessError::TargetTypeMismatch(_)));

    let thread_safe_field = TypeDescriptor::of::<ThreadSafeOwner>()
        .field("count")
        .expect("thread-safe owner field exists");
    let thread_safe_target = ThreadSafeOwner { count: 13 };
    let borrowed = thread_safe_field
        .get_dynamic(DynamicRef::<ThreadSafe>::new(&thread_safe_target))
        .expect("thread-safe adapter supports shared access");
    assert_eq!(borrowed.downcast_ref::<u32>(), Some(&13));
}
