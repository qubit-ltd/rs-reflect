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
use qubit_reflect::descriptor::FieldSetError;
use qubit_reflect::descriptor::Visibility;
use qubit_reflect::value::DynamicMut;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::DynamicRef;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

#[Reflect]
struct OpaqueValue(&'static str);

struct UnreflectedValue(&'static str);

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

#[Reflect]
struct OptionalOwner {
    pub value: Option<u32>,
}

#[Reflect]
struct AccessMiddle {
    pub count: u32,
}

#[Reflect]
struct OptionalCustomOwner {
    pub value: Option<AccessMiddle>,
}

#[Reflect]
struct NestedCustomOwner {
    pub middle: AccessMiddle,
}

#[Reflect]
struct ArrayOwner {
    items: [String; 2],
}

#[Reflect]
struct SkipOwner {
    #[reflect(skip)]
    pub omitted: UnreflectedValue,
    pub count: u32,
}

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
    let opaque_type = opaque
        .find_concrete_type()
        .expect("the reflected custom field type has a concrete descriptor");
    assert!(std::ptr::eq(opaque.type_expression().context(), &context));
    assert!(std::ptr::eq(opaque_type.context(), &context));
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
    assert!(opaque.access().readable);
    assert!(opaque.access().mutable);
    assert!(opaque.access().replaceable);
    assert!(opaque.access().constructible);

    let target = owner();
    assert_eq!(target.opaque.0, "opaque");
    assert_eq!(
        opaque
            .get_dynamic(DynamicRef::<Local>::new(&target))
            .expect("a reflected custom field supports dynamic reads")
            .downcast_ref::<OpaqueValue>()
            .map(|value| value.0),
        Some("opaque")
    );
    assert!(matches!(
        private.get_dynamic(DynamicRef::<Local>::new(&target)),
        Err(FieldAccessError::NotPublic)
    ));
}

#[test]
fn dynamic_unsafe_field_read_bypasses_visibility_but_keeps_adapter_checks() {
    let context = ReflectionContext::builder()
        .include_type::<FieldOwner>()
        .build()
        .expect("the owner is registered in this context");
    let descriptor = TypeDescriptor::of_in::<FieldOwner>(&context);
    let private = descriptor.field("private_label").expect("private field exists");
    let public = descriptor.field("count").expect("public field exists");
    let target = owner();

    assert!(matches!(
        private.get_dynamic(DynamicRef::<Local>::new(&target)),
        Err(FieldAccessError::NotPublic)
    ));
    let private_value = unsafe { private.get_dynamic_unsafe::<Local>(DynamicRef::<Local>::new(&target)) }
        .expect("the caller authorizes reading this private field");
    assert_eq!(private_value.downcast_ref::<String>(), Some(&target.private_label));

    let public_value = public
        .get_dynamic(DynamicRef::<Local>::new(&target))
        .expect("public fields remain available through the safe API");
    assert_eq!(public_value.downcast_ref::<u32>(), Some(&target.count));
}

#[test]
fn derived_array_field_supports_dynamic_reads_with_visibility_checks() {
    let context = ReflectionContext::builder()
        .include_type::<ArrayOwner>()
        .include_type::<[String; 2]>()
        .include_type::<String>()
        .build()
        .expect("context contains the owner and reflected array field type");
    let descriptor = TypeDescriptor::of_in::<ArrayOwner>(&context);
    let field = descriptor.field("items").expect("array field exists");
    let target = ArrayOwner {
        items: [String::from("first"), String::from("second")],
    };

    assert!(matches!(
        field.get_dynamic(DynamicRef::<Local>::new(&target)),
        Err(FieldAccessError::NotPublic)
    ));
    let value = unsafe { field.get_dynamic_unsafe::<Local>(DynamicRef::<Local>::new(&target)) }
        .expect("the array field has a local dynamic adapter");

    assert_eq!(value.downcast_ref::<[String; 2]>(), Some(&target.items));
    assert!(std::ptr::eq(field.context(), &context));
}

#[test]
fn derived_option_field_supports_dynamic_reads_for_some_and_none() {
    let field = TypeDescriptor::of::<OptionalOwner>()
        .field("value")
        .expect("optional field exists");
    let some_owner = OptionalOwner { value: Some(27) };
    let some = field
        .get_dynamic(DynamicRef::<Local>::new(&some_owner))
        .expect("Option<u32> gets a dynamic field adapter");
    assert_eq!(some.downcast_ref::<Option<u32>>(), Some(&Some(27)));
    assert_eq!(
        some.as_any().and_then(|value| value.downcast_ref::<Option<u32>>()),
        Some(&Some(27))
    );

    let none_owner = OptionalOwner { value: None };
    let none = field
        .get_dynamic(DynamicRef::<Local>::new(&none_owner))
        .expect("the same adapter reads an absent optional value");
    assert_eq!(none.downcast_ref::<Option<u32>>(), Some(&None));
}

#[test]
fn derived_option_field_with_custom_reflect_inner_supports_dynamic_reads() {
    let field = TypeDescriptor::of::<OptionalCustomOwner>()
        .field("value")
        .expect("optional custom field exists");
    let some_owner = OptionalCustomOwner {
        value: Some(AccessMiddle { count: 43 }),
    };
    let some = field
        .get_dynamic(DynamicRef::<Local>::new(&some_owner))
        .expect("Option<AccessMiddle> gets a dynamic field adapter");
    assert_eq!(
        some.downcast_ref::<Option<AccessMiddle>>()
            .and_then(Option::as_ref)
            .map(|value| value.count),
        Some(43)
    );

    let none_owner = OptionalCustomOwner { value: None };
    let none = field
        .get_dynamic(DynamicRef::<Local>::new(&none_owner))
        .expect("the same adapter reads None");
    assert!(matches!(none.downcast_ref::<Option<AccessMiddle>>(), Some(None)));
}

#[test]
fn dynamic_custom_field_supports_dynamic_read_and_write() {
    let field = TypeDescriptor::of::<NestedCustomOwner>()
        .field("middle")
        .expect("nested custom field exists");
    let mut owner = NestedCustomOwner {
        middle: AccessMiddle { count: 17 },
    };

    {
        let value = field
            .get_dynamic(DynamicRef::<Local>::new(&owner))
            .expect("a reflected custom field gets a dynamic adapter");
        assert_eq!(value.downcast_ref::<AccessMiddle>().map(|value| value.count), Some(17));
    }

    field
        .set_dynamic(
            DynamicMut::<Local>::new(&mut owner),
            DynamicOwned::<Local>::new(AccessMiddle { count: 29 }),
        )
        .expect("a reflected custom field can be replaced dynamically");
    assert_eq!(owner.middle.count, 29);
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
fn reflect_skip_explicitly_excludes_an_unreflected_field() {
    let descriptor = TypeDescriptor::of::<SkipOwner>();
    assert!(descriptor.field("omitted").is_err());
    assert!(descriptor.field("count").is_ok());
    let owner = SkipOwner {
        omitted: UnreflectedValue("excluded"),
        count: 5,
    };
    assert_eq!(owner.omitted.0, "excluded");
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
