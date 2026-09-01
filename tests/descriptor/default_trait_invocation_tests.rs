// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Regression coverage for default trait method invocation parity.

use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

use qubit_reflect as reflect;
use qubit_reflect::Reflect;
use qubit_reflect::descriptor::CatchingAvailability;
use qubit_reflect::descriptor::MethodQualifier;
use qubit_reflect::invoke::Invocation;
use qubit_reflect::invoke::InvocationOutput;
use qubit_reflect::reflect;
use qubit_reflect::reflect_impl;
use qubit_reflect::registry::ReflectRegistry;
use qubit_reflect::value::DynamicOwned;

static DEFAULT_RAW_VALUE: u8 = 47;

#[allow(dead_code)]
fn default_increment(value: u8) -> u8 {
    value + 1
}

#[derive(Reflect)]
#[reflect(opaque)]
struct DefaultInvocationSample;

#[reflect]
#[allow(dead_code)]
trait DefaultInvocationMatrix {
    fn default_tuple_output() -> (u8, u16) {
        (3, 5)
    }

    fn default_array_output() -> [u8; 2] {
        [7, 11]
    }

    fn default_raw_pointer_output() -> *const u8 {
        &DEFAULT_RAW_VALUE
    }

    fn default_function_pointer_output() -> fn(u8) -> u8 {
        default_increment
    }

    fn default_shared_str(value: &str) -> &str {
        value
    }

    #[allow(clippy::boxed_local)]
    fn default_boxed_receiver(self: Box<Self>) -> u8 {
        29
    }

    fn default_rc_receiver(self: Rc<Self>) -> u8 {
        31
    }

    fn default_arc_receiver(self: Arc<Self>) -> u8 {
        37
    }

    fn default_pinned_box_receiver(self: Pin<Box<Self>>) -> u8 {
        41
    }

    fn default_pinned_ref_receiver(self: Pin<&Self>) -> u8 {
        43
    }

    fn default_pinned_mut_receiver(self: Pin<&mut Self>) -> u8 {
        47
    }

    #[reflect(thread_safe)]
    fn default_thread_safe(value: u8) -> u8 {
        value + 2
    }

    #[reflect(catch_unwind)]
    fn default_catching() -> u8 {
        panic!("default catching payload")
    }

    #[reflect(thread_safe, catch_unwind)]
    fn default_thread_safe_catching() -> u8 {
        panic!("default thread-safe catching payload")
    }

    #[allow(dead_code)]
    fn default_never_output() -> ! {
        panic!("never method entered user code")
    }

    #[allow(dead_code)]
    fn default_slice_parameter(_: &[u8]) -> usize {
        0
    }

    #[allow(dead_code)]
    fn default_trait_object_parameter(_: &dyn std::fmt::Debug) -> usize {
        0
    }

    #[allow(dead_code)]
    fn default_opaque_output() -> impl Iterator<Item = u8> {
        [13, 17].into_iter()
    }

    #[allow(dead_code)]
    fn default_unsupported_mutable_borrow(value: &mut u8) -> &mut u8 {
        value
    }

    #[allow(dead_code, improper_ctypes_definitions)]
    unsafe extern "C" fn default_all_blockers<T>((left, right): (T, T), _: &[u8]) -> impl Iterator<Item = T> {
        [left, right].into_iter()
    }
}

#[reflect_impl]
impl DefaultInvocationMatrix for DefaultInvocationSample {}

#[derive(Reflect)]
#[reflect(opaque)]
struct PatternReasonSample;

#[reflect_impl]
impl PatternReasonSample {
    #[allow(dead_code)]
    unsafe fn wildcard_pattern(_: u8) -> u8 {
        19
    }

    #[allow(dead_code)]
    unsafe fn destructure_pattern((left, right): (u8, u8)) -> u8 {
        left + right
    }
}

fn default_method<'a>(
    registry: &'a ReflectRegistry,
    method_name: &str,
) -> &'a reflect::descriptor::MethodInstanceDescriptor {
    let implementations = registry.implementations(DefaultInvocationSample::type_descriptor().type_id());
    let reflected_trait = implementations
        .iter()
        .find_map(|implementation| {
            implementation
                .implemented_trait()
                .filter(|descriptor| descriptor.definition().rust_name() == "DefaultInvocationMatrix")
        })
        .expect("default invocation trait implementation must be registered");
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Trait(reflected_trait),
        method_name,
    ) else {
        panic!("default trait method must remain discoverable");
    };
    instance
}

fn invoke_default_owned(registry: &ReflectRegistry, method_name: &str) -> DynamicOwned<reflect::value::Local> {
    let output = default_method(registry, method_name)
        .adapter()
        .expect("safe default output shape needs an adapter")
        .invoke_local(Invocation::associated([]))
        .expect("local default adapter must be present")
        .expect("default invocation must validate");
    let InvocationOutput::Owned(value) = output else {
        panic!("safe concrete default output must be owned");
    };
    value
}

#[test]
fn test_default_trait_adapter_supports_safe_owned_non_path_outputs_and_never() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");

    let tuple = invoke_default_owned(registry, "default_tuple_output");
    let Ok(tuple) = DynamicOwned::<reflect::value::Local>::downcast::<(u8, u16)>(tuple) else {
        panic!("tuple output type must be retained");
    };
    assert_eq!(tuple, (3, 5));

    let array = invoke_default_owned(registry, "default_array_output");
    let Ok(array) = DynamicOwned::<reflect::value::Local>::downcast::<[u8; 2]>(array) else {
        panic!("array output type must be retained");
    };
    assert_eq!(array, [7, 11]);

    let pointer = invoke_default_owned(registry, "default_raw_pointer_output");
    let Ok(pointer) = DynamicOwned::<reflect::value::Local>::downcast::<*const u8>(pointer) else {
        panic!("raw pointer output type must be retained");
    };
    assert_eq!(pointer, &DEFAULT_RAW_VALUE);

    let function = invoke_default_owned(registry, "default_function_pointer_output");
    let Ok(function) = DynamicOwned::<reflect::value::Local>::downcast::<fn(u8) -> u8>(function) else {
        panic!("function pointer output type must be retained");
    };
    assert_eq!(function(23), 24);

    let never = default_method(registry, "default_never_output");
    assert!(never.adapter().is_some());
    assert!(never.unavailable_reasons().is_empty());
}

#[test]
fn test_default_trait_adapter_preserves_the_dedicated_str_variant_and_origin() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let output = default_method(registry, "default_shared_str")
        .adapter()
        .expect("shared str default method needs an adapter")
        .invoke_local(Invocation::associated([reflect::invoke::InvocationArg::Ref(
            reflect::value::DynamicRef::<reflect::value::Local>::new_str("default"),
        )]))
        .expect("local default adapter must be present")
        .expect("shared str default invocation must validate");
    let InvocationOutput::Ref { value, origins } = output else {
        panic!("shared str default output must be borrowed");
    };
    let Ok(value) = value.into_str() else {
        panic!("shared str default output must retain the str variant");
    };
    assert_eq!(value, "default");
    assert_eq!(origins.as_ref(), [reflect::invoke::BorrowOrigin::Parameter(0)]);
}

#[test]
fn test_default_trait_adapter_supports_owned_smart_receivers() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let cases: [(&str, DynamicOwned<reflect::value::Local>, u8); 4] = [
        (
            "default_boxed_receiver",
            DynamicOwned::<reflect::value::Local>::new(Box::new(DefaultInvocationSample)),
            29,
        ),
        (
            "default_rc_receiver",
            DynamicOwned::<reflect::value::Local>::new(Rc::new(DefaultInvocationSample)),
            31,
        ),
        (
            "default_arc_receiver",
            DynamicOwned::<reflect::value::Local>::new(Arc::new(DefaultInvocationSample)),
            37,
        ),
        (
            "default_pinned_box_receiver",
            DynamicOwned::<reflect::value::Local>::new(Box::pin(DefaultInvocationSample)),
            41,
        ),
    ];

    for (method_name, receiver, expected) in cases {
        let instance = default_method(registry, method_name);
        assert!(instance.unavailable_reasons().is_empty(), "{method_name}");
        let output = instance
            .adapter()
            .expect("safe default smart receiver needs an adapter")
            .invoke_local(Invocation::owned(receiver, []))
            .expect("local default adapter must be present")
            .expect("default smart receiver invocation must validate");
        let InvocationOutput::Owned(value) = output else {
            panic!("default smart receiver output must be owned");
        };
        let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
            panic!("default smart receiver output type must be retained");
        };
        assert_eq!(value, expected);
    }
}

#[test]
fn test_default_trait_adapter_supports_pinned_borrow_receivers() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let shared_instance = default_method(registry, "default_pinned_ref_receiver");
    assert!(shared_instance.unavailable_reasons().is_empty());
    let shared_receiver = Box::pin(DefaultInvocationSample);
    let output = shared_instance
        .adapter()
        .expect("default pinned shared receiver needs an adapter")
        .invoke_pinned_ref_local(reflect::invoke::PinnedRefInvocation::new(shared_receiver.as_ref(), []))
        .expect("default pinned shared entry point must be present")
        .expect("default pinned shared invocation must validate");
    let InvocationOutput::Owned(value) = output else {
        panic!("default pinned shared output must be owned");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
        panic!("default pinned shared output type must be retained");
    };
    assert_eq!(value, 43);

    let mutable_instance = default_method(registry, "default_pinned_mut_receiver");
    assert!(mutable_instance.unavailable_reasons().is_empty());
    let mut mutable_receiver = Box::pin(DefaultInvocationSample);
    let output = mutable_instance
        .adapter()
        .expect("default pinned mutable receiver needs an adapter")
        .invoke_pinned_mut_local(reflect::invoke::PinnedMutInvocation::new(mutable_receiver.as_mut(), []))
        .expect("default pinned mutable entry point must be present")
        .expect("default pinned mutable invocation must validate");
    let InvocationOutput::Owned(value) = output else {
        panic!("default pinned mutable output must be owned");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
        panic!("default pinned mutable output type must be retained");
    };
    assert_eq!(value, 47);
}

#[test]
fn test_default_trait_adapter_supports_thread_safe_mode() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let instance = default_method(registry, "default_thread_safe");
    assert!(instance.unavailable_reasons().is_empty());
    let adapter = instance.adapter().expect("thread-safe default method needs an adapter");
    assert!(adapter.invoke_local(Invocation::associated([])).is_none());
    let output = adapter
        .invoke_thread_safe(Invocation::associated([reflect::invoke::InvocationArg::Owned(
            reflect::value::DynamicOwned::<reflect::value::ThreadSafe>::new(51_u8),
        )]))
        .expect("thread-safe default entry point must be present")
        .expect("thread-safe default invocation must validate");
    let InvocationOutput::Owned(value) = output else {
        panic!("thread-safe default output must be owned");
    };
    let Ok(value) = reflect::value::DynamicOwned::<reflect::value::ThreadSafe>::downcast::<u8>(value) else {
        panic!("thread-safe default output type must be retained");
    };
    assert_eq!(value, 53);
}

#[cfg(panic = "unwind")]
#[test]
fn test_default_trait_adapter_supports_catching_and_thread_safe_composition() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let local = default_method(registry, "default_catching");
    assert!(local.unavailable_reasons().is_empty());
    let local = local.adapter().expect("catching default method needs an adapter");
    assert_eq!(local.catching_availability(), CatchingAvailability::Available);
    let panic = match local
        .invoke_catching_local(Invocation::associated([]))
        .expect("local catching default entry point must be present")
        .expect("local catching default invocation must validate")
    {
        Ok(_) => panic!("the default panic must be captured"),
        Err(panic) => panic,
    };
    assert_eq!(
        panic.payload().downcast_ref::<&str>(),
        Some(&"default catching payload")
    );

    let combined = default_method(registry, "default_thread_safe_catching");
    assert!(combined.unavailable_reasons().is_empty());
    let combined = combined
        .adapter()
        .expect("thread-safe catching default method needs an adapter");
    assert_eq!(combined.catching_availability(), CatchingAvailability::Available);
    assert!(combined.invoke_catching_local(Invocation::associated([])).is_none());
    let panic = match combined
        .invoke_catching_thread_safe(Invocation::associated([]))
        .expect("thread-safe catching default entry point must be present")
        .expect("thread-safe catching default invocation must validate")
    {
        Ok(_) => panic!("the thread-safe default panic must be captured"),
        Err(panic) => panic,
    };
    assert_eq!(
        panic.payload().downcast_ref::<&str>(),
        Some(&"default thread-safe catching payload")
    );
}

#[test]
fn test_default_trait_adapter_reports_precise_complete_unavailable_reasons() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    for (method_name, reasons) in [
        (
            "default_slice_parameter",
            &[reflect::descriptor::InvocationUnavailableReason::UnsupportedUnsizedValue][..],
        ),
        (
            "default_trait_object_parameter",
            &[reflect::descriptor::InvocationUnavailableReason::UnsupportedUnsizedValue][..],
        ),
        (
            "default_opaque_output",
            &[reflect::descriptor::InvocationUnavailableReason::OpaqueReturn][..],
        ),
        (
            "default_unsupported_mutable_borrow",
            &[reflect::descriptor::InvocationUnavailableReason::UnsupportedBorrowedReturn][..],
        ),
        (
            "default_all_blockers",
            &[
                reflect::descriptor::InvocationUnavailableReason::UnspecializedGeneric,
                reflect::descriptor::InvocationUnavailableReason::UnsafeMethod,
                reflect::descriptor::InvocationUnavailableReason::UnsupportedAbi,
                reflect::descriptor::InvocationUnavailableReason::OpaqueReturn,
                reflect::descriptor::InvocationUnavailableReason::UnsupportedUnsizedValue,
            ][..],
        ),
    ] {
        let instance = default_method(registry, method_name);
        assert!(instance.adapter().is_none());
        assert_eq!(instance.unavailable_reasons(), reasons, "{method_name}");
    }
}

#[test]
fn test_positional_patterns_are_not_invocation_blockers() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(PatternReasonSample::type_descriptor().type_id());
    for method_name in ["wildcard_pattern", "destructure_pattern"] {
        let reflect::descriptor::MethodLookup::Unique(instance) =
            reflect::descriptor::ImplDescriptor::lookup_method(implementations, MethodQualifier::Inherent, method_name)
        else {
            panic!("pattern method must remain discoverable");
        };
        assert!(instance.adapter().is_none());
        assert_eq!(
            instance.unavailable_reasons(),
            [reflect::descriptor::InvocationUnavailableReason::UnsafeMethod],
            "{method_name}",
        );
    }
}
