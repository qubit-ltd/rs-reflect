// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow explicit-imports
//! Integration coverage for implementation registration expansion.
use std::any::TypeId;
use std::future::Future;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::OnceLock;
use std::task::Context;
use std::task::Poll;

use qubit_reflect as reflect;
use qubit_reflect::Reflect;
use qubit_reflect::descriptor::MethodQualifier;
use qubit_reflect::descriptor::StructKind;
use qubit_reflect::invoke::Invocation;
use qubit_reflect::invoke::InvocationArg;
use qubit_reflect::invoke::InvocationOutput;
use qubit_reflect::reflect;
use qubit_reflect::reflect_impl;
use qubit_reflect::registry::ReflectRegistry;
use qubit_reflect::value::DynamicOwned;

struct Sample;

static REFLECTED_RAW_VALUE: u8 = 41;

fn reflected_increment(value: u8) -> u8 {
    value + 1
}

impl Reflect for Sample {
    fn type_descriptor() -> &'static reflect::TypeDescriptor {
        static DESCRIPTOR: OnceLock<reflect::TypeDescriptor> = OnceLock::new();
        DESCRIPTOR.get_or_init(|| {
            reflect::__private::codegen_v1::descriptor::struct_type::<Sample>("Sample", StructKind::Named, &[])
        })
    }
}

trait External {
    fn value(&self) -> u8;
}

#[reflect]
trait Reflected {
    fn reflected_value(&self) -> u8;

    fn default_value(&self) -> u8 {
        13
    }

    #[allow(dead_code)]
    unsafe fn unsafe_default_value(&self) -> u8 {
        23
    }

    #[allow(dead_code)]
    fn generic_default_value<T>(&self, value: T) -> T {
        value
    }
}

#[reflect_impl]
impl Reflected for Sample {
    fn reflected_value(&self) -> u8 {
        11
    }
}

#[reflect]
trait RenamedReflected {
    #[reflect(rename = "renamed_override_query")]
    fn renamed_override(&self) -> u8;

    #[reflect(rename = "renamed_default_query")]
    fn renamed_default(&self) -> u8 {
        37
    }
}

#[reflect_impl]
impl RenamedReflected for Sample {
    fn renamed_override(&self) -> u8 {
        31
    }
}

#[reflect_impl(external_trait_id = "example.external")]
impl External for Sample {
    fn value(&self) -> u8 {
        7
    }
}

#[reflect_impl]
impl Sample {
    fn private_method(&self) -> u8 {
        3
    }

    pub fn public_method() -> u8 {
        5
    }

    fn reflected_associated() -> u8 {
        17
    }

    const fn reflected_const_function(value: u8) -> u8 {
        value + 1
    }

    fn reflected_owned_argument(value: u8) -> u8 {
        value + 1
    }

    fn reflected_shared_argument(value: &u8) -> u8 {
        value + 2
    }

    fn reflected_mutable_argument(value: &mut u8) -> u8 {
        *value += 2;
        *value
    }

    fn reflected_shared_str(value: &str) -> &str {
        value
    }

    fn reflected_mutable_str(value: &mut str) {
        value.make_ascii_uppercase();
    }

    #[allow(dead_code)]
    fn reflected_slice(value: &[u8]) -> usize {
        value.len()
    }

    #[allow(dead_code)]
    fn reflected_dyn_debug(value: &dyn std::fmt::Debug) -> usize {
        format!("{value:?}").len()
    }

    fn reflected_tuple_output() -> (u8, u16) {
        (7, 11)
    }

    fn reflected_array_output() -> [u8; 2] {
        [13, 17]
    }

    fn reflected_raw_pointer_output() -> *const u8 {
        &REFLECTED_RAW_VALUE
    }

    fn reflected_function_pointer_output() -> fn(u8) -> u8 {
        reflected_increment
    }

    fn reflected_before_skipped() -> u8 {
        37
    }

    #[reflect(skip)]
    #[allow(dead_code)]
    fn reflected_skipped() -> u8 {
        41
    }

    fn reflected_after_skipped() -> u8 {
        43
    }

    #[allow(dead_code)]
    fn reflected_never_output() -> ! {
        panic!("never method entered user code")
    }

    #[allow(dead_code)]
    fn reflected_opaque_output() -> impl Iterator<Item = u8> {
        [19, 23].into_iter()
    }

    #[allow(dead_code)]
    fn reflected_unsupported_mutable_borrow(value: &mut u8) -> &mut u8 {
        value
    }

    #[allow(dead_code, improper_ctypes_definitions)]
    unsafe extern "C" fn reflected_all_blockers<T>((left, right): (T, T)) -> impl Iterator<Item = T> {
        [left, right].into_iter()
    }

    fn reflected_two_owned_arguments(first: u8, second: u8) -> u8 {
        first + second
    }

    #[reflect(specialize(T = u8))]
    #[allow(dead_code)]
    fn reflected_generic<T>(value: T) -> T {
        value
    }

    #[reflect(specialize(T = u8))]
    #[allow(dead_code)]
    fn reflected_nested_generic<T>(value: Vec<Option<T>>) -> Vec<Option<T>> {
        value
    }

    #[reflect(specialize(N = 5))]
    #[allow(dead_code)]
    fn reflected_const_generic<const N: usize>(value: usize) -> usize {
        value + N
    }

    async fn reflected_async_argument(value: u8) -> u8 {
        value + 3
    }

    #[reflect(thread_safe)]
    fn reflected_thread_safe_argument(value: u8) -> u8 {
        value + 4
    }

    #[reflect(catch_unwind)]
    fn reflected_panicking() -> u8 {
        panic!("caught panic")
    }

    fn reflected_shared(&self) -> u8 {
        19
    }

    fn reflected_borrowed_path(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed("borrowed")
    }

    extern "C" fn reflected_c_abi() -> u8 {
        29
    }
}

struct Counter(u8);

impl Reflect for Counter {
    fn type_descriptor() -> &'static reflect::TypeDescriptor {
        static DESCRIPTOR: OnceLock<reflect::TypeDescriptor> = OnceLock::new();
        DESCRIPTOR.get_or_init(|| {
            reflect::__private::codegen_v1::descriptor::struct_type::<Counter>("Counter", StructKind::Named, &[])
        })
    }
}

#[reflect_impl]
impl Counter {
    fn reflected_borrowed(&self) -> &u8 {
        &self.0
    }

    fn reflected_borrowed_mut(&mut self) -> &mut u8 {
        &mut self.0
    }

    fn reflected_mut(&mut self) -> u8 {
        self.0 += 1;
        self.0
    }

    fn reflected_owned(self) -> u8 {
        self.0
    }
}

struct SmartReceiver(u8);

struct PinnedOnlyReceiver {
    value: u8,
    _pin: PhantomPinned,
}

#[derive(reflect::Reflect)]
#[reflect(opaque)]
struct ExtensionReceiver(u8);

#[derive(reflect::Reflect)]
#[reflect(opaque)]
struct UnadaptedReceiver;

#[derive(reflect::Reflect)]
#[reflect(opaque)]
struct SpecializedGenericImpl<T>(std::marker::PhantomData<T>);

#[derive(reflect::Reflect)]
#[reflect(opaque)]
struct SpecializedConstGenericImpl<const N: usize>;

#[derive(reflect::Reflect)]
#[reflect(opaque)]
struct MultipleSpecializedGenericImpl<T>(std::marker::PhantomData<T>);

#[derive(reflect::Reflect)]
#[reflect(opaque)]
struct ConstrainedSpecializedGenericImpl<T>(std::marker::PhantomData<T>);

#[derive(reflect::Reflect)]
#[reflect(opaque)]
struct LifetimeSpecializedGenericImpl<'a, T>(std::marker::PhantomData<&'a T>);

struct GenericImplDefinitionOnly<'a, T, const N: usize>(std::marker::PhantomData<&'a T>);

#[allow(dead_code)]
struct GenericTraitImplDefinitionOnly<T>(std::marker::PhantomData<T>);

#[reflect]
#[allow(dead_code)]
trait GenericImplDefinitionTrait {
    fn generic_definition_value(&self) -> usize;
}

#[reflect_impl]
impl<T: Clone + Send> GenericImplDefinitionTrait for GenericTraitImplDefinitionOnly<T> {
    fn generic_definition_value(&self) -> usize {
        1
    }
}

#[allow(dead_code)]
trait ExternalBlanketDefinitionTrait {}

#[reflect_impl(external_trait_id = "integration.external.blanket_definition")]
impl<T> ExternalBlanketDefinitionTrait for T where T: Clone + Send {}

#[derive(Clone, reflect::Reflect)]
#[reflect(opaque)]
struct ConditionalDefaultSample;

#[reflect]
trait ConditionalDefault {
    #[allow(dead_code)]
    fn clone_value(&self) -> Self
    where
        Self: Clone,
    {
        self.clone()
    }
}

#[reflect_impl]
impl ConditionalDefault for ConditionalDefaultSample {}

#[derive(reflect::Reflect)]
#[reflect(opaque)]
struct AssociatedDefaultSample;

#[reflect]
trait AssociatedDefault {
    type Value;

    #[allow(dead_code)]
    fn associated_value(&self) -> Self::Value {
        panic!("the descriptor-only default method must not be invoked")
    }
}

#[reflect_impl]
impl AssociatedDefault for AssociatedDefaultSample {
    type Value = String;
}

#[derive(reflect::Reflect)]
#[reflect(opaque)]
struct NestedAssociatedDefaultSample;

#[reflect]
trait NestedAssociatedDefault<'a> {
    type Value;

    #[allow(dead_code)]
    fn nested_values(&'a self) -> Box<dyn Iterator<Item = Self::Value> + 'a> {
        Box::new(std::iter::empty())
    }
}

#[reflect_impl]
impl NestedAssociatedDefault<'static> for NestedAssociatedDefaultSample {
    type Value = String;
}

impl Reflect for SmartReceiver {
    fn type_descriptor() -> &'static reflect::TypeDescriptor {
        static DESCRIPTOR: OnceLock<reflect::TypeDescriptor> = OnceLock::new();
        DESCRIPTOR.get_or_init(|| {
            reflect::__private::codegen_v1::descriptor::struct_type::<SmartReceiver>(
                "SmartReceiver",
                StructKind::Named,
                &[],
            )
        })
    }
}

impl Reflect for PinnedOnlyReceiver {
    fn type_descriptor() -> &'static reflect::TypeDescriptor {
        static DESCRIPTOR: OnceLock<reflect::TypeDescriptor> = OnceLock::new();
        DESCRIPTOR.get_or_init(|| {
            reflect::__private::codegen_v1::descriptor::struct_type::<PinnedOnlyReceiver>(
                "PinnedOnlyReceiver",
                StructKind::Named,
                &[],
            )
        })
    }
}

#[reflect_impl]
impl SmartReceiver {
    #[allow(clippy::boxed_local)]
    fn boxed(self: Box<Self>) -> u8 {
        self.0
    }

    fn rc(self: Rc<Self>) -> u8 {
        self.0
    }

    fn arc(self: Arc<Self>) -> u8 {
        self.0
    }

    fn pinned_box(self: Pin<Box<Self>>) -> u8 {
        self.0
    }

    #[allow(dead_code)]
    fn pinned_ref(self: Pin<&Self>) -> u8 {
        self.0
    }

    #[allow(dead_code)]
    fn pinned_mut(self: Pin<&mut Self>) -> u8 {
        self.0
    }
}

#[reflect_impl]
impl PinnedOnlyReceiver {
    fn pinned_ref(self: Pin<&Self>) -> u8 {
        self.value
    }

    fn pinned_mut(self: Pin<&mut Self>) -> u8 {
        self.value
    }
}

#[reflect_impl]
impl ExtensionReceiver {
    fn extension(self: Pin<Rc<Self>>) -> u8 {
        self.0
    }

    fn extension_named(self: Pin<Rc<Self>>, first: u8, second: u16) -> u32 {
        u32::from(self.0) + u32::from(first) * 1_000 + u32::from(second)
    }
}

#[reflect_impl]
impl UnadaptedReceiver {
    fn extension(self: Pin<Rc<Self>>) {}
}

fn extension_receiver_adapter<'call>(
    receiver: reflect::invoke::InvocationReceiver<'call, reflect::value::Local>,
) -> Result<Pin<Rc<ExtensionReceiver>>, reflect::invoke::InvocationReceiver<'call, reflect::value::Local>> {
    match receiver {
        reflect::invoke::InvocationReceiver::Owned(value) => {
            reflect::value::DynamicOwned::<reflect::value::Local>::downcast::<Pin<Rc<ExtensionReceiver>>>(value)
                .map_err(reflect::invoke::InvocationReceiver::Owned)
        }
        receiver => Err(receiver),
    }
}

reflect::register_type_capabilities!(ExtensionReceiver: [
    reflect::invoke::receiver_adapter_key::<Pin<Rc<ExtensionReceiver>>, reflect::value::Local>()
        => extension_receiver_adapter
]);

#[reflect_impl(specialize(T = u8))]
impl<T> SpecializedGenericImpl<T> {
    fn specialization_value() -> u8 {
        67
    }

    fn round_trip(value: T) -> T {
        value
    }
}

#[reflect_impl(specialize(N = 3))]
impl<const N: usize> SpecializedConstGenericImpl<N> {
    fn specialization_value() -> usize {
        N
    }
}

#[reflect_impl(specialize(T = u8), specialize(T = u16))]
impl<T> MultipleSpecializedGenericImpl<T> {
    fn type_size() -> usize {
        ::std::mem::size_of::<T>()
    }
}

#[reflect_impl(specialize(T = String))]
impl<T: Clone + Send> ConstrainedSpecializedGenericImpl<T> where T: Sync {}

#[reflect_impl(specialize(T = u8))]
impl<'a, T: 'a> LifetimeSpecializedGenericImpl<'a, T> {}

#[reflect_impl]
#[allow(dead_code)]
impl<'a, T: Clone + 'a, const N: usize> GenericImplDefinitionOnly<'a, T, N>
where
    T: Send,
{
    fn definition_only(value: &'a T) -> usize {
        let _ = value;
        N
    }

    fn generic_definition_method<U: Clone>(value: U) -> U {
        value
    }

    #[reflect(skip)]
    fn skipped_definition_method() {}
}

#[test]
fn test_reflect_impl_generates_callable_adapter_for_shared_receiver() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_shared",
    ) else {
        panic!("generated shared method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("safe shared method needs adapter");
    let sample = Sample;
    let output = adapter
        .invoke_local(Invocation::borrowed(
            reflect::value::DynamicRef::<reflect::value::Local>::new(&sample),
            [],
        ))
        .expect("local adapter must be present")
        .expect("validated invocation must call method");
    let InvocationOutput::Owned(value) = output else {
        panic!("shared method must return an owned value");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
        panic!("generated value must retain type");
    };
    assert_eq!(value, 19);
}

#[test]
fn test_reflect_impl_generated_str_adapter_preserves_parameter_borrow_origin() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_shared_str",
    ) else {
        panic!("generated shared str method instance must be discoverable");
    };
    let output = instance
        .adapter()
        .expect("shared str method needs a dedicated adapter")
        .invoke_local(Invocation::associated([InvocationArg::Ref(
            reflect::value::DynamicRef::<reflect::value::Local>::new_str("hello"),
        )]))
        .expect("local adapter must be present")
        .expect("shared str invocation must validate");
    let InvocationOutput::Ref { value, origins } = output else {
        panic!("shared str method must return a shared borrow");
    };
    let Ok(value) = value.into_str() else {
        panic!("output must retain the str variant");
    };
    assert_eq!(value, "hello");
    assert_eq!(origins.as_ref(), [reflect::invoke::BorrowOrigin::Parameter(0)]);
}

#[test]
fn test_reflect_impl_generated_mut_str_adapter_uses_dedicated_dynamic_variant() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_mutable_str",
    ) else {
        panic!("generated mutable str method instance must be discoverable");
    };
    let mut text = String::from("hello");
    let output = instance
        .adapter()
        .expect("mutable str method needs a dedicated adapter")
        .invoke_local(Invocation::associated([InvocationArg::Mut(
            reflect::value::DynamicMut::<reflect::value::Local>::new_str_mut(text.as_mut_str()),
        )]))
        .expect("local adapter must be present")
        .expect("mutable str invocation must validate");
    assert!(matches!(output, InvocationOutput::Unit));
    drop(output);
    assert_eq!(text, "HELLO");
}

#[test]
fn test_reflect_impl_only_describes_unsized_slice_and_trait_object_parameters() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    for method_name in ["reflected_slice", "reflected_dyn_debug"] {
        let reflect::descriptor::MethodLookup::Unique(instance) =
            reflect::descriptor::ImplDescriptor::lookup_method(implementations, MethodQualifier::Inherent, method_name)
        else {
            panic!("unsized parameter method must remain discoverable");
        };
        assert!(instance.adapter().is_none());
        assert_eq!(
            instance.unavailable_reasons(),
            [reflect::descriptor::InvocationUnavailableReason::UnsupportedUnsizedValue]
        );
    }
}

#[test]
fn test_reflect_impl_invokes_safe_owned_non_path_output_shapes() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let invoke = |method_name| {
        let reflect::descriptor::MethodLookup::Unique(instance) =
            reflect::descriptor::ImplDescriptor::lookup_method(implementations, MethodQualifier::Inherent, method_name)
        else {
            panic!("owned output method must remain discoverable");
        };
        instance
            .adapter()
            .expect("safe owned output shape needs an adapter")
            .invoke_local(Invocation::associated([]))
            .expect("local adapter must be present")
            .expect("owned output invocation must validate")
    };

    let InvocationOutput::Owned(tuple) = invoke("reflected_tuple_output") else {
        panic!("tuple output must be owned");
    };
    let Ok(tuple) = DynamicOwned::<reflect::value::Local>::downcast::<(u8, u16)>(tuple) else {
        panic!("tuple output type must be retained");
    };
    assert_eq!(tuple, (7, 11));

    let InvocationOutput::Owned(array) = invoke("reflected_array_output") else {
        panic!("array output must be owned");
    };
    let Ok(array) = DynamicOwned::<reflect::value::Local>::downcast::<[u8; 2]>(array) else {
        panic!("array output type must be retained");
    };
    assert_eq!(array, [13, 17]);

    let InvocationOutput::Owned(pointer) = invoke("reflected_raw_pointer_output") else {
        panic!("raw pointer output must be owned");
    };
    let Ok(pointer) = DynamicOwned::<reflect::value::Local>::downcast::<*const u8>(pointer) else {
        panic!("raw pointer output type must be retained");
    };
    assert_eq!(pointer, &REFLECTED_RAW_VALUE);

    let InvocationOutput::Owned(function) = invoke("reflected_function_pointer_output") else {
        panic!("function pointer output must be owned");
    };
    let Ok(function) = DynamicOwned::<reflect::value::Local>::downcast::<fn(u8) -> u8>(function) else {
        panic!("function pointer output type must be retained");
    };
    assert_eq!(function(29), 30);
}

#[test]
fn test_reflect_impl_never_method_keeps_an_unreachable_adapter() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_never_output",
    ) else {
        panic!("never method must remain discoverable");
    };
    assert!(instance.adapter().is_some());
    assert!(instance.unavailable_reasons().is_empty());
}

#[test]
fn test_reflect_impl_reports_precise_opaque_and_borrowed_return_reasons() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    for (method_name, reason) in [
        (
            "reflected_opaque_output",
            reflect::descriptor::InvocationUnavailableReason::OpaqueReturn,
        ),
        (
            "reflected_unsupported_mutable_borrow",
            reflect::descriptor::InvocationUnavailableReason::UnsupportedBorrowedReturn,
        ),
    ] {
        let reflect::descriptor::MethodLookup::Unique(instance) =
            reflect::descriptor::ImplDescriptor::lookup_method(implementations, MethodQualifier::Inherent, method_name)
        else {
            panic!("descriptor-only return method must remain discoverable");
        };
        assert!(instance.adapter().is_none());
        assert_eq!(instance.unavailable_reasons(), [reason]);
    }
}

#[test]
fn test_reflect_impl_skip_preserves_descriptor_source_index_and_order() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let implementation = implementations
        .iter()
        .find(|implementation| {
            implementation
                .methods()
                .iter()
                .any(|method| method.rust_name() == "reflected_skipped")
        })
        .expect("skipped method must remain in its concrete impl descriptor");
    let methods = implementation.methods();
    let skipped_index = methods
        .iter()
        .position(|method| method.rust_name() == "reflected_skipped")
        .expect("skipped method must retain source order");
    assert_eq!(methods[skipped_index - 1].rust_name(), "reflected_before_skipped");
    assert_eq!(methods[skipped_index + 1].rust_name(), "reflected_after_skipped");
    assert_eq!(
        methods[skipped_index - 1].identity().index() + 1,
        methods[skipped_index].identity().index()
    );
    assert_eq!(
        methods[skipped_index].identity().index() + 1,
        methods[skipped_index + 1].identity().index()
    );

    let instance = &implementation.method_instances()[skipped_index];
    assert_eq!(instance.effective_method().rust_name(), "reflected_skipped");
    assert_eq!(
        instance.implementation_source(),
        reflect::descriptor::MethodImplementationSource::Declared
    );
    assert!(instance.adapter().is_none());
    assert_eq!(
        instance.unavailable_reasons(),
        [reflect::descriptor::InvocationUnavailableReason::DisabledByPolicy]
    );
}

#[test]
fn test_reflect_impl_collects_all_unavailable_reasons_in_stable_order() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_all_blockers",
    ) else {
        panic!("multiply blocked method must remain discoverable");
    };
    assert!(instance.adapter().is_none());
    assert_eq!(
        instance.unavailable_reasons(),
        [
            reflect::descriptor::InvocationUnavailableReason::UnspecializedGeneric,
            reflect::descriptor::InvocationUnavailableReason::UnsafeMethod,
            reflect::descriptor::InvocationUnavailableReason::UnsupportedAbi,
            reflect::descriptor::InvocationUnavailableReason::OpaqueReturn,
        ]
    );
}

#[test]
fn test_reflect_impl_registers_explicit_generic_method_specialization() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_generic",
    ) else {
        panic!("registered generic specialization must be discoverable");
    };
    assert_eq!(instance.arguments().len(), 1);
    let adapter = instance
        .adapter()
        .expect("simple registered specialization needs an adapter");
    let output = adapter
        .invoke_local(Invocation::associated([InvocationArg::Owned(DynamicOwned::<
            reflect::value::Local,
        >::new(31_u8))]))
        .expect("local adapter must be present")
        .expect("specialized invocation must validate");
    let InvocationOutput::Owned(value) = output else {
        panic!("specialized method must return an owned value");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
        panic!("specialized output must retain its concrete type");
    };
    assert_eq!(value, 31);
}

#[test]
fn test_reflect_impl_recursively_substitutes_nested_generic_method_specialization() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_nested_generic",
    ) else {
        panic!("nested generic method specialization must be discoverable");
    };
    let output = instance
        .adapter()
        .expect("nested generic method specialization needs an adapter")
        .invoke_local(Invocation::associated([InvocationArg::Owned(DynamicOwned::<
            reflect::value::Local,
        >::new(vec![
            Some(31_u8),
            None,
        ]))]))
        .expect("local adapter must be present")
        .expect("nested generic invocation must validate");
    let InvocationOutput::Owned(value) = output else {
        panic!("nested generic method must return an owned value");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<Vec<Option<u8>>>(value) else {
        panic!("nested generic output must use the concrete specialized type");
    };
    assert_eq!(value, vec![Some(31), None]);
}

#[test]
fn test_reflect_impl_invokes_explicit_const_generic_method_specialization() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_const_generic",
    ) else {
        panic!("registered const specialization must be discoverable");
    };
    assert_eq!(instance.arguments().len(), 1);
    let adapter = instance
        .adapter()
        .expect("registered const specialization needs an adapter");
    let output = adapter
        .invoke_local(Invocation::associated([InvocationArg::Owned(DynamicOwned::<
            reflect::value::Local,
        >::new(7_usize))]))
        .expect("local adapter must be present")
        .expect("specialized invocation must validate");
    let InvocationOutput::Owned(value) = output else {
        panic!("specialized method must return an owned value");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<usize>(value) else {
        panic!("specialized output must retain its concrete type");
    };
    assert_eq!(value, 12);
}

#[test]
fn test_reflect_impl_records_and_invokes_const_function_at_runtime() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_const_function",
    ) else {
        panic!("const function must be discoverable");
    };
    assert!(instance.effective_method().qualifiers().is_const());
    let output = instance
        .adapter()
        .expect("const function needs an ordinary runtime adapter")
        .invoke_local(Invocation::associated([InvocationArg::Owned(DynamicOwned::<
            reflect::value::Local,
        >::new(41_u8))]))
        .expect("local adapter must be present")
        .expect("const function invocation must validate");
    let InvocationOutput::Owned(value) = output else {
        panic!("const function must return an owned value");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
        panic!("const function output must retain its exact type");
    };
    assert_eq!(value, 42);
}

#[test]
fn test_reflect_impl_generates_callable_adapter_for_mutable_receiver() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Counter::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) =
        reflect::descriptor::ImplDescriptor::lookup_method(implementations, MethodQualifier::Inherent, "reflected_mut")
    else {
        panic!("generated mutable method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("safe mutable method needs adapter");
    let mut counter = Counter(3);
    let value = {
        let output = adapter
            .invoke_local(Invocation::borrowed_mut(
                reflect::value::DynamicMut::<reflect::value::Local>::new(&mut counter),
                [],
            ))
            .expect("local adapter must be present")
            .expect("validated invocation must call method");
        let InvocationOutput::Owned(value) = output else {
            panic!("mutable method must return an owned value");
        };
        let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
            panic!("generated value must retain type");
        };
        value
    };
    assert_eq!(value, 4);
    assert_eq!(counter.0, 4);
}

#[test]
fn test_reflect_impl_generates_callable_adapter_for_owned_receiver() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Counter::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_owned",
    ) else {
        panic!("generated owned method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("safe owned method needs adapter");
    let output = adapter
        .invoke_local(Invocation::owned(
            DynamicOwned::<reflect::value::Local>::new(Counter(23)),
            [],
        ))
        .expect("local adapter must be present")
        .expect("validated invocation must call method");
    let InvocationOutput::Owned(value) = output else {
        panic!("owned method must return an owned value");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
        panic!("generated value must retain type");
    };
    assert_eq!(value, 23);
}

#[test]
fn test_reflect_impl_generates_callable_adapters_for_owned_smart_receivers() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(SmartReceiver::type_descriptor().type_id());
    let cases: [(&str, DynamicOwned<reflect::value::Local>); 4] = [
        (
            "boxed",
            DynamicOwned::<reflect::value::Local>::new(Box::new(SmartReceiver(11))),
        ),
        (
            "rc",
            DynamicOwned::<reflect::value::Local>::new(Rc::new(SmartReceiver(12))),
        ),
        (
            "arc",
            DynamicOwned::<reflect::value::Local>::new(Arc::new(SmartReceiver(13))),
        ),
        (
            "pinned_box",
            DynamicOwned::<reflect::value::Local>::new(Box::pin(SmartReceiver(14))),
        ),
    ];

    for (expected, (name, receiver)) in cases.into_iter().enumerate() {
        let reflect::descriptor::MethodLookup::Unique(instance) =
            reflect::descriptor::ImplDescriptor::lookup_method(implementations, MethodQualifier::Inherent, name)
        else {
            panic!("smart receiver method must be discoverable");
        };
        let adapter = instance.adapter().expect("supported smart receiver needs an adapter");
        let output = adapter
            .invoke_local(Invocation::owned(receiver, []))
            .expect("local adapter must be present")
            .expect("validated invocation must call method");
        let InvocationOutput::Owned(value) = output else {
            panic!("smart receiver method must return an owned value");
        };
        let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
            panic!("generated value must retain type");
        };
        assert_eq!(value, (expected as u8) + 11);
    }
}

#[test]
fn test_reflect_impl_invokes_pinned_borrow_receivers_without_erasing_pin() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(PinnedOnlyReceiver::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(shared_instance) =
        reflect::descriptor::ImplDescriptor::lookup_method(implementations, MethodQualifier::Inherent, "pinned_ref")
    else {
        panic!("pinned receiver method must be discoverable");
    };
    let shared_adapter = shared_instance.adapter().expect("pinned shared receiver needs adapter");
    let shared_receiver = Box::pin(PinnedOnlyReceiver {
        value: 41,
        _pin: PhantomPinned,
    });
    let shared_output = shared_adapter
        .invoke_pinned_ref_local(reflect::invoke::PinnedRefInvocation::new(shared_receiver.as_ref(), []))
        .expect("exact typed pinned adapter must be present")
        .expect("pinned shared invocation must validate");
    let InvocationOutput::Owned(shared_value) = shared_output else {
        panic!("pinned shared method must return an owned value");
    };
    let Ok(shared_value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(shared_value) else {
        panic!("pinned shared output must retain type");
    };
    assert_eq!(shared_value, 41);

    let reflect::descriptor::MethodLookup::Unique(mutable_instance) =
        reflect::descriptor::ImplDescriptor::lookup_method(implementations, MethodQualifier::Inherent, "pinned_mut")
    else {
        panic!("pinned mutable receiver method must be discoverable");
    };
    let mutable_adapter = mutable_instance
        .adapter()
        .expect("pinned mutable receiver needs adapter");
    let mut mutable_receiver = Box::pin(PinnedOnlyReceiver {
        value: 42,
        _pin: PhantomPinned,
    });
    let mutable_output = mutable_adapter
        .invoke_pinned_mut_local(reflect::invoke::PinnedMutInvocation::new(mutable_receiver.as_mut(), []))
        .expect("exact typed pinned adapter must be present")
        .expect("pinned mutable invocation must validate");
    let InvocationOutput::Owned(mutable_value) = mutable_output else {
        panic!("pinned mutable method must return an owned value");
    };
    let Ok(mutable_value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(mutable_value) else {
        panic!("pinned mutable output must retain type");
    };
    assert_eq!(mutable_value, 42);
}

#[test]
fn test_reflect_impl_invokes_an_explicit_receiver_through_a_registered_adapter() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(ExtensionReceiver::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) =
        reflect::descriptor::ImplDescriptor::lookup_method(implementations, MethodQualifier::Inherent, "extension")
    else {
        panic!("registered extension receiver method must be discoverable");
    };
    let adapter = instance
        .adapter()
        .expect("registered receiver adapter must enable invocation");
    let output = adapter
        .invoke_local(Invocation::owned(
            DynamicOwned::<reflect::value::Local>::new(Pin::new(Rc::new(ExtensionReceiver(53)))),
            [],
        ))
        .expect("local adapter must be present")
        .expect("receiver adapter must accept the exact container");
    let InvocationOutput::Owned(value) = output else {
        panic!("extension receiver must return an owned value");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
        panic!("extension receiver output must retain its concrete type");
    };
    assert_eq!(value, 53);
}

#[test]
fn test_explicit_receiver_rejection_recovers_named_arguments_in_caller_order() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(ExtensionReceiver::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "extension_named",
    ) else {
        panic!("registered extension receiver method must be discoverable");
    };
    let invocation = Invocation::from_bindings(
        Some(reflect::invoke::InvocationReceiver::Owned(DynamicOwned::<
            reflect::value::Local,
        >::new(String::from(
            "wrong receiver",
        )))),
        [
            reflect::invoke::InvocationBinding::named(
                "second",
                InvocationArg::Owned(DynamicOwned::<reflect::value::Local>::new(22_u16)),
            ),
            reflect::invoke::InvocationBinding::positional(InvocationArg::Owned(
                DynamicOwned::<reflect::value::Local>::new(11_u8),
            )),
        ],
    );

    let Some(Err(failure)) = instance.invoke_local(invocation) else {
        panic!("the explicit receiver adapter must reject an incompatible container")
    };
    assert!(matches!(
        failure.error().kind(),
        reflect::invoke::InvocationErrorKind::ReceiverAdapterRejected { .. }
    ));
    assert_eq!(failure.recovery().argument_name(0), Some("second"));
    assert_eq!(failure.recovery().argument_name(1), None);
    let (receiver, arguments) = failure.into_recovery().into_parts();
    let Some(reflect::invoke::InvocationReceiver::Owned(receiver)) = receiver else {
        panic!("recovery must retain the incompatible owned receiver")
    };
    assert!(DynamicOwned::<reflect::value::Local>::downcast::<String>(receiver).is_ok());
    let mut arguments = arguments.into_vec().into_iter();
    let Some(InvocationArg::Owned(second)) = arguments.next() else {
        panic!("first caller binding must remain owned")
    };
    let Some(InvocationArg::Owned(first)) = arguments.next() else {
        panic!("second caller binding must remain owned")
    };
    let Ok(second) = DynamicOwned::<reflect::value::Local>::downcast::<u16>(second) else {
        panic!("named value must remain intact")
    };
    let Ok(first) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(first) else {
        panic!("positional value must remain intact")
    };
    assert_eq!(second, 22);
    assert_eq!(first, 11);
}

#[test]
fn test_reflect_impl_only_describes_an_explicit_receiver_without_an_adapter() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(UnadaptedReceiver::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) =
        reflect::descriptor::ImplDescriptor::lookup_method(implementations, MethodQualifier::Inherent, "extension")
    else {
        panic!("unadapted explicit receiver method must remain discoverable");
    };
    assert!(instance.adapter().is_none());
    assert!(
        instance
            .unavailable_reasons()
            .contains(&reflect::descriptor::InvocationUnavailableReason::UnsupportedReceiver)
    );
}

#[test]
fn test_reflect_impl_registers_explicit_generic_impl_specialization() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(SpecializedGenericImpl::<u8>::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "specialization_value",
    ) else {
        panic!("explicit generic impl specialization must register its method");
    };
    let implementation = implementations
        .iter()
        .find(|implementation| {
            implementation
                .method_instances()
                .iter()
                .any(|candidate| std::ptr::eq(candidate, instance))
        })
        .expect("method instance must belong to one registered generic impl");
    let registered_definition = registry
        .impl_definitions()
        .iter()
        .copied()
        .find(|definition| std::ptr::eq(*definition, implementation.definition()))
        .expect("the concrete specialization must share its registered definition");
    assert!(std::ptr::eq(registered_definition, implementation.definition()));
    assert_eq!(implementation.definition().generic_definition().parameters().len(), 1);
    assert_eq!(implementation.arguments().len(), 1);
    let adapter = instance
        .adapter()
        .expect("concrete generic impl method needs an adapter");
    let output = adapter
        .invoke_local(Invocation::associated([]))
        .expect("local adapter must be present")
        .expect("specialized generic impl invocation must validate");
    let InvocationOutput::Owned(value) = output else {
        panic!("specialized generic impl method must return an owned value");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
        panic!("specialized generic impl output must retain its concrete type");
    };
    assert_eq!(value, 67);

    let reflect::descriptor::MethodLookup::Unique(instance) =
        reflect::descriptor::ImplDescriptor::lookup_method(implementations, MethodQualifier::Inherent, "round_trip")
    else {
        panic!("generic impl method must register for its concrete specialization");
    };
    let adapter = instance.adapter().expect("concrete generic method needs an adapter");
    let output = adapter
        .invoke_local(Invocation::associated([InvocationArg::Owned(DynamicOwned::<
            reflect::value::Local,
        >::new(19_u8))]))
        .expect("local adapter must be present")
        .expect("concrete generic impl invocation must validate");
    let InvocationOutput::Owned(value) = output else {
        panic!("concrete generic impl method must return an owned value");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
        panic!("concrete generic impl method output must retain its concrete type");
    };
    assert_eq!(value, 19);
}

#[test]
fn test_reflect_impl_registers_explicit_const_generic_impl_specialization() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(SpecializedConstGenericImpl::<3>::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "specialization_value",
    ) else {
        panic!("explicit const generic impl specialization must register its method");
    };
    let implementation = implementations
        .iter()
        .find(|implementation| {
            implementation
                .method_instances()
                .iter()
                .any(|candidate| std::ptr::eq(candidate, instance))
        })
        .expect("method instance must belong to one registered const generic impl");
    assert_eq!(implementation.definition().generic_definition().parameters().len(), 1);
    assert_eq!(implementation.arguments().len(), 1);
    let adapter = instance
        .adapter()
        .expect("concrete const generic impl method needs an adapter");
    let output = adapter
        .invoke_local(Invocation::associated([]))
        .expect("local adapter must be present")
        .expect("specialized const generic impl invocation must validate");
    let InvocationOutput::Owned(value) = output else {
        panic!("specialized const generic impl method must return an owned value");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<usize>(value) else {
        panic!("specialized const generic impl output must retain its concrete type");
    };
    assert_eq!(value, 3);
}

#[test]
fn test_reflect_impl_shares_one_definition_across_multiple_specializations() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let u8_impls = registry.implementations(MultipleSpecializedGenericImpl::<u8>::type_descriptor().type_id());
    let u16_impls = registry.implementations(MultipleSpecializedGenericImpl::<u16>::type_descriptor().type_id());

    assert_eq!(u8_impls.len(), 1);
    assert_eq!(u16_impls.len(), 1);
    assert!(std::ptr::eq(u8_impls[0].definition(), u16_impls[0].definition()));
    assert_eq!(
        registry
            .impl_definitions()
            .iter()
            .filter(|definition| std::ptr::eq(**definition, u8_impls[0].definition()))
            .count(),
        1,
    );
}

#[test]
fn test_reflect_impl_validates_bounds_and_static_lifetime_specialization() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let constrained =
        registry.implementations(ConstrainedSpecializedGenericImpl::<String>::type_descriptor().type_id());
    let lifetime = registry.implementations(LifetimeSpecializedGenericImpl::<'static, u8>::type_descriptor().type_id());

    assert_eq!(constrained.len(), 1);
    assert_eq!(lifetime.len(), 1);
    assert_eq!(constrained[0].arguments().len(), 1);
    assert_eq!(lifetime[0].arguments().len(), 1);
}

#[test]
fn test_reflect_impl_registers_generic_definition_without_concrete_instance() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let definitions: Vec<_> = registry
        .impl_definitions()
        .iter()
        .copied()
        .filter(|definition| {
            matches!(
                definition.target_type(),
                reflect::expression::TypeExpression::Concrete(target)
                    if target.path().last().is_some_and(|segment| segment.as_ref() == "GenericImplDefinitionOnly")
            )
        })
        .collect();

    assert_eq!(definitions.len(), 1);
    let definition = definitions[0];
    assert_eq!(definition.generic_definition().parameters().len(), 3);
    assert!(!definition.generic_definition().predicates().is_empty());
    assert_eq!(definition.methods().len(), 3);
    assert_eq!(definition.methods()[1].generic_definition().parameters().len(), 1);
    assert_eq!(definition.methods()[2].rust_name(), "skipped_definition_method");
    let candidates = registry.find_impl_definitions_by_target(definition.target_type());
    assert_eq!(candidates.len(), 1);
    assert!(!candidates.is_empty());
    assert!(std::ptr::eq(
        candidates.iter().next().expect("the exact target must match"),
        definition,
    ));
    assert_eq!(
        registry
            .find_impl_definitions_by_target(definition.target_type())
            .into_iter()
            .count(),
        1,
    );
    assert!(
        registry
            .implementations(TypeId::of::<GenericImplDefinitionOnly<'static, u8, 7>>())
            .is_empty()
    );
}

#[test]
fn test_reflect_impl_registers_trait_and_blanket_definitions_without_instances() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let generic_trait = registry
        .impl_definitions()
        .iter()
        .copied()
        .find(|definition| {
            matches!(
                definition.target_type(),
                reflect::expression::TypeExpression::Concrete(target)
                    if target.path().last().is_some_and(|segment| segment.as_ref() == "GenericTraitImplDefinitionOnly")
            )
        })
        .expect("generic trait impl definition must be registered");
    assert_eq!(generic_trait.kind(), reflect::descriptor::ImplKind::Trait);
    assert_eq!(generic_trait.generic_definition().parameters().len(), 1);
    assert_eq!(
        generic_trait
            .implemented_trait()
            .expect("reflected trait definition must be resolved")
            .rust_name(),
        "GenericImplDefinitionTrait",
    );

    let blanket = registry
        .impl_definitions()
        .iter()
        .copied()
        .find(|definition| {
            matches!(definition.target_type(), reflect::expression::TypeExpression::Parameter(name) if name.as_ref() == "T")
        })
        .expect("external blanket impl definition must be registered");
    assert_eq!(blanket.generic_definition().parameters().len(), 1);
    assert!(!blanket.generic_definition().predicates().is_empty());
    assert_eq!(
        blanket
            .implemented_trait()
            .expect("external blanket trait definition must be resolved")
            .rust_path(),
        "ExternalBlanketDefinitionTrait",
    );
}

#[test]
fn test_reflect_impl_generates_callable_adapter_for_shared_borrowed_output() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Counter::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_borrowed",
    ) else {
        panic!("generated borrowed-output method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("shared borrowed output needs adapter");
    let counter = Counter(23);
    let output = adapter
        .invoke_local(Invocation::borrowed(
            reflect::value::DynamicRef::<reflect::value::Local>::new(&counter),
            [],
        ))
        .expect("local adapter must be present")
        .expect("validated invocation must call method");
    let InvocationOutput::Ref { value, origins } = output else {
        panic!("shared borrowed output must be represented as a dynamic reference");
    };
    assert_eq!(value.downcast_ref::<u8>(), Some(&23));
    assert_eq!(origins.as_ref(), &[reflect::invoke::BorrowOrigin::Receiver]);
}

#[test]
fn test_reflect_impl_generates_callable_adapter_for_mutable_borrowed_output() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Counter::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_borrowed_mut",
    ) else {
        panic!("generated mutable borrowed-output method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("mutable borrowed output needs adapter");
    let mut counter = Counter(23);
    {
        let output = adapter
            .invoke_local(Invocation::borrowed_mut(
                reflect::value::DynamicMut::<reflect::value::Local>::new(&mut counter),
                [],
            ))
            .expect("local adapter must be present")
            .expect("validated invocation must call method");
        let InvocationOutput::Mut { mut value, origin } = output else {
            panic!("mutable borrowed output must be represented as a dynamic mutable reference");
        };
        assert_eq!(origin, reflect::invoke::BorrowOrigin::Receiver);
        *value
            .downcast_mut::<u8>()
            .expect("mutable output must retain the exact field type") = 42;
    }
    assert_eq!(counter.0, 42);
}

#[test]
fn test_reflect_impl_does_not_generate_adapter_for_non_rust_abi() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_c_abi",
    ) else {
        panic!("non-Rust ABI method instance must remain describable");
    };
    assert!(instance.adapter().is_none());
    assert!(
        instance
            .unavailable_reasons()
            .contains(&reflect::descriptor::InvocationUnavailableReason::UnsupportedAbi)
    );
}

#[test]
fn test_reflect_impl_registers_inherent_and_external_fragments() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    assert!(registry.get(Sample::type_descriptor().type_id()).is_none());
    assert_eq!(Sample::public_method(), 5);
    assert_eq!(Sample.value(), 7);
    assert_eq!(Sample.reflected_value(), 11);
    assert_eq!(Sample.default_value(), 13);
    assert_eq!(Sample.private_method(), 3);
    assert_eq!(Sample::reflected_c_abi(), 29);
    assert_eq!(Sample.reflected_borrowed_path(), "borrowed");
}

#[test]
fn test_reflect_impl_invokes_overridden_trait_method_through_descriptor_adapter() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflected_trait = implementations
        .iter()
        .find_map(|implementation| {
            implementation
                .implemented_trait()
                .filter(|descriptor| descriptor.definition().rust_name() == "Reflected")
        })
        .expect("reflected trait implementation must be registered");
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Trait(reflected_trait),
        "reflected_value",
    ) else {
        panic!("overridden trait method instance must be discoverable");
    };
    assert_eq!(
        instance.implementation_source(),
        reflect::descriptor::MethodImplementationSource::Overridden
    );
    let adapter = instance
        .adapter()
        .expect("supported overridden trait method needs an adapter");
    let sample = Sample;
    let output = adapter
        .invoke_local(Invocation::borrowed(
            reflect::value::DynamicRef::<reflect::value::Local>::new(&sample),
            [],
        ))
        .expect("local adapter must be present")
        .expect("validated invocation must call the overridden trait method");
    let InvocationOutput::Owned(value) = output else {
        panic!("overridden trait method must return an owned value");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
        panic!("overridden trait method output must retain its concrete type");
    };
    assert_eq!(value, 11);
}

#[test]
fn test_reflect_impl_invokes_defaulted_trait_method_through_descriptor_adapter() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflected_trait = implementations
        .iter()
        .find_map(|implementation| {
            implementation
                .implemented_trait()
                .filter(|descriptor| descriptor.definition().rust_name() == "Reflected")
        })
        .expect("reflected trait implementation must be registered");
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Trait(reflected_trait),
        "default_value",
    ) else {
        panic!("defaulted trait method instance must be discoverable");
    };
    assert_eq!(
        instance.implementation_source(),
        reflect::descriptor::MethodImplementationSource::Defaulted
    );
    let adapter = instance
        .adapter()
        .expect("supported defaulted trait method needs an adapter");
    let sample = Sample;
    let output = adapter
        .invoke_local(Invocation::borrowed(
            reflect::value::DynamicRef::<reflect::value::Local>::new(&sample),
            [],
        ))
        .expect("local adapter must be present")
        .expect("validated invocation must call the defaulted trait method");
    let InvocationOutput::Owned(value) = output else {
        panic!("defaulted trait method must return an owned value");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
        panic!("defaulted trait method output must retain its concrete type");
    };
    assert_eq!(value, 13);
}

#[test]
fn test_reflect_impl_matches_renamed_trait_override_by_rust_identity() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflected_trait = implementations
        .iter()
        .find_map(|implementation| {
            implementation
                .implemented_trait()
                .filter(|descriptor| descriptor.definition().rust_name() == "RenamedReflected")
        })
        .expect("renamed reflected trait implementation must be registered");
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Trait(reflected_trait),
        "renamed_override_query",
    ) else {
        panic!("renamed override must be discoverable by its trait query name");
    };
    assert_eq!(
        instance.implementation_source(),
        reflect::descriptor::MethodImplementationSource::Overridden
    );
    assert_eq!(instance.effective_method().rust_name(), "renamed_override");
    let adapter = instance
        .adapter()
        .expect("renaming a declaration must not disable its override adapter");
    let sample = Sample;
    let output = adapter
        .invoke_local(Invocation::borrowed(
            reflect::value::DynamicRef::<reflect::value::Local>::new(&sample),
            [],
        ))
        .expect("local adapter must be present")
        .expect("renamed override invocation must validate");
    let InvocationOutput::Owned(value) = output else {
        panic!("renamed override must return an owned value");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
        panic!("renamed override output must retain its concrete type");
    };
    assert_eq!(value, 31);
}

#[test]
fn test_reflect_impl_keeps_renamed_trait_default_by_rust_identity() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflected_trait = implementations
        .iter()
        .find_map(|implementation| {
            implementation
                .implemented_trait()
                .filter(|descriptor| descriptor.definition().rust_name() == "RenamedReflected")
        })
        .expect("renamed reflected trait implementation must be registered");
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Trait(reflected_trait),
        "renamed_default_query",
    ) else {
        panic!("renamed default must be discoverable by its trait query name");
    };
    assert_eq!(
        instance.implementation_source(),
        reflect::descriptor::MethodImplementationSource::Defaulted
    );
    assert_eq!(instance.effective_method().rust_name(), "renamed_default");
    let adapter = instance
        .adapter()
        .expect("renaming a declaration must not disable its default adapter");
    let sample = Sample;
    let output = adapter
        .invoke_local(Invocation::borrowed(
            reflect::value::DynamicRef::<reflect::value::Local>::new(&sample),
            [],
        ))
        .expect("local adapter must be present")
        .expect("renamed default invocation must validate");
    let InvocationOutput::Owned(value) = output else {
        panic!("renamed default must return an owned value");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
        panic!("renamed default output must retain its concrete type");
    };
    assert_eq!(value, 37);
}

#[test]
fn test_reflect_impl_preserves_structured_reasons_for_unavailable_trait_methods() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflected_trait = implementations
        .iter()
        .find_map(|implementation| {
            implementation
                .implemented_trait()
                .filter(|descriptor| descriptor.definition().rust_name() == "Reflected")
        })
        .expect("reflected trait implementation must be registered");

    for (method, expected_reason) in [
        (
            "unsafe_default_value",
            reflect::descriptor::InvocationUnavailableReason::UnsafeMethod,
        ),
        (
            "generic_default_value",
            reflect::descriptor::InvocationUnavailableReason::UnspecializedGeneric,
        ),
    ] {
        let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
            implementations,
            MethodQualifier::Trait(reflected_trait),
            method,
        ) else {
            panic!("unavailable trait method instance must remain discoverable");
        };
        assert!(instance.adapter().is_none());
        assert_eq!(instance.unavailable_reasons(), [expected_reason]);
    }
}

#[test]
fn test_reflect_impl_only_describes_defaults_with_unproven_method_bounds_or_associated_types() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    for (target, trait_name, method_name) in [
        (
            ConditionalDefaultSample::type_descriptor().type_id(),
            "ConditionalDefault",
            "clone_value",
        ),
        (
            AssociatedDefaultSample::type_descriptor().type_id(),
            "AssociatedDefault",
            "associated_value",
        ),
    ] {
        let implementations = registry.implementations(target);
        let reflected_trait = implementations
            .iter()
            .find_map(|implementation| {
                implementation
                    .implemented_trait()
                    .filter(|descriptor| descriptor.definition().rust_name() == trait_name)
            })
            .expect("reflected trait implementation must remain registered");
        let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
            implementations,
            MethodQualifier::Trait(reflected_trait),
            method_name,
        ) else {
            panic!("descriptor-only default method must remain discoverable");
        };
        assert_eq!(
            instance.implementation_source(),
            reflect::descriptor::MethodImplementationSource::Defaulted
        );
        assert!(instance.adapter().is_none());
        assert_eq!(
            instance.unavailable_reasons(),
            [reflect::descriptor::InvocationUnavailableReason::DisabledByPolicy]
        );
    }
}

#[test]
fn test_reflect_impl_only_describes_nested_trait_object_associated_type_and_lifetime() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(NestedAssociatedDefaultSample::type_descriptor().type_id());
    let reflected_trait = implementations
        .iter()
        .find_map(|implementation| {
            implementation
                .implemented_trait()
                .filter(|descriptor| descriptor.definition().rust_name() == "NestedAssociatedDefault")
        })
        .expect("nested associated reflected trait implementation must remain registered");
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Trait(reflected_trait),
        "nested_values",
    ) else {
        panic!("nested associated default method must remain discoverable");
    };
    assert_eq!(
        instance.implementation_source(),
        reflect::descriptor::MethodImplementationSource::Defaulted
    );
    assert!(instance.adapter().is_none());
    assert_eq!(
        instance.unavailable_reasons(),
        [reflect::descriptor::InvocationUnavailableReason::UnsupportedBorrowedReturn]
    );
}

#[test]
fn test_reflect_impl_generates_callable_adapter_for_safe_associated_function() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_associated",
    ) else {
        panic!("generated associated method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("safe associated method needs adapter");
    let output = adapter
        .invoke_local(Invocation::associated([]))
        .expect("local adapter must be present")
        .expect("validated invocation must call method");
    let InvocationOutput::Owned(value) = output else {
        panic!("associated method must return an owned value");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
        panic!("generated value must retain type");
    };
    assert_eq!(value, 17);
}

#[test]
fn test_reflect_impl_generates_callable_adapter_for_owned_argument() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_owned_argument",
    ) else {
        panic!("generated associated method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("owned argument method needs adapter");
    let output = adapter
        .invoke_local(Invocation::associated([InvocationArg::Owned(DynamicOwned::<
            reflect::value::Local,
        >::new(41_u8))]))
        .expect("local adapter must be present")
        .expect("validated invocation must call method");
    let InvocationOutput::Owned(value) = output else {
        panic!("associated method must return an owned value");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
        panic!("generated value must retain type");
    };
    assert_eq!(value, 42);
}

#[test]
fn test_reflect_impl_generates_callable_adapter_for_shared_argument() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_shared_argument",
    ) else {
        panic!("generated associated method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("shared argument method needs adapter");
    let input = 40_u8;
    let output = adapter
        .invoke_local(Invocation::associated([InvocationArg::Ref(
            reflect::value::DynamicRef::<reflect::value::Local>::new(&input),
        )]))
        .expect("local adapter must be present")
        .expect("validated invocation must call method");
    let InvocationOutput::Owned(value) = output else {
        panic!("associated method must return an owned value");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
        panic!("generated value must retain type");
    };
    assert_eq!(value, 42);
}

#[test]
fn test_reflect_impl_generates_callable_adapter_for_mutable_argument() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_mutable_argument",
    ) else {
        panic!("generated associated method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("mutable argument method needs adapter");
    let mut input = 40_u8;
    let value = {
        let output = adapter
            .invoke_local(Invocation::associated([InvocationArg::Mut(
                reflect::value::DynamicMut::<reflect::value::Local>::new(&mut input),
            )]))
            .expect("local adapter must be present")
            .expect("validated invocation must call method");
        let InvocationOutput::Owned(value) = output else {
            panic!("associated method must return an owned value");
        };
        let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
            panic!("generated value must retain type");
        };
        value
    };
    assert_eq!(value, 42);
    assert_eq!(input, 42);
}

#[test]
fn test_reflect_impl_preserves_all_owned_arguments_after_validation_failure() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_two_owned_arguments",
    ) else {
        panic!("two-argument method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("safe owned-argument method needs adapter");
    let result = adapter
        .invoke_local(Invocation::associated([
            InvocationArg::Owned(DynamicOwned::<reflect::value::Local>::new(7_u8)),
            InvocationArg::Owned(DynamicOwned::<reflect::value::Local>::new(String::from("wrong"))),
        ]))
        .expect("local adapter must be present");
    let Err(failure) = result else {
        panic!("the second argument must fail exact type validation");
    };
    assert!(matches!(
        failure.error().kind(),
        reflect::invoke::InvocationErrorKind::ArgumentTypeMismatch { index: 1, .. }
    ));
    let (_, arguments) = failure.into_recovery().into_parts();
    let mut arguments = arguments.into_vec().into_iter();
    let (Some(InvocationArg::Owned(first)), Some(InvocationArg::Owned(second)), None) =
        (arguments.next(), arguments.next(), arguments.next())
    else {
        panic!("validation failure must preserve both owned arguments in source order");
    };
    let Ok(first) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(first) else {
        panic!("first value must remain intact");
    };
    assert_eq!(first, 7);
    assert!(DynamicOwned::<reflect::value::Local>::downcast::<String>(second).is_ok());
}

#[test]
fn test_reflect_impl_generates_callable_adapter_for_async_method() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_async_argument",
    ) else {
        panic!("generated async method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("safe async method needs adapter");
    let output = adapter
        .invoke_local(Invocation::associated([InvocationArg::Owned(DynamicOwned::<
            reflect::value::Local,
        >::new(39_u8))]))
        .expect("local adapter must be present")
        .expect("validated invocation must start method");
    let InvocationOutput::Future(mut future) = output else {
        panic!("async method must return a reflected future");
    };
    let Poll::Ready(InvocationOutput::Owned(value)) = poll_once(&mut future) else {
        panic!("simple async method must complete when polled");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
        panic!("async output must retain its owned value");
    };
    assert_eq!(value, 42);
}

#[test]
fn test_reflect_impl_generates_explicit_thread_safe_adapter() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_thread_safe_argument",
    ) else {
        panic!("generated thread-safe method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("explicit thread-safe method needs adapter");
    assert!(adapter.invoke_local(Invocation::associated([])).is_none());
    let output = adapter
        .invoke_thread_safe(Invocation::associated([InvocationArg::Owned(
            reflect::value::DynamicOwned::<reflect::value::ThreadSafe>::new(38_u8),
        )]))
        .expect("thread-safe adapter must be present")
        .expect("validated invocation must call method");
    let InvocationOutput::Owned(value) = output else {
        panic!("thread-safe method must return an owned output");
    };
    let Ok(value) = reflect::value::DynamicOwned::<reflect::value::ThreadSafe>::downcast::<u8>(value) else {
        panic!("thread-safe output must retain its exact type");
    };
    assert_eq!(value, 42);
}

#[test]
fn test_reflect_impl_generates_explicit_catching_adapter() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_panicking",
    ) else {
        panic!("generated catching method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("catching method needs adapter");
    let caught = match adapter
        .invoke_catching_local(Invocation::associated([]))
        .expect("catching adapter must be present")
        .expect("validated invocation must begin")
    {
        Ok(_) => panic!("the panic must be reported separately from validation failure"),
        Err(caught) => caught,
    };
    assert_eq!(caught.payload().downcast_ref::<&str>(), Some(&"caught panic"));
}

fn poll_once<F: Future + Unpin>(future: &mut F) -> Poll<F::Output> {
    let waker = std::task::Waker::noop();
    let mut context = Context::from_waker(waker);
    Pin::new(future).poll(&mut context)
}
