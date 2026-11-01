// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use std::pin::Pin;
use std::sync::OnceLock;

use qubit_reflect::__private::codegen_v4::invoke::EffectiveMethod;
use qubit_reflect::__private::codegen_v4::registration::MethodBinding;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;
use qubit_reflect::bind_method;

struct SpecializationFixture;

impl qubit_reflect::__private::codegen_v4::descriptor::Reflect for SpecializationFixture {
    fn __reflect_registration() -> &'static qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<Self> {
        static REGISTRATION: qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<SpecializationFixture> =
            qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration::new();
        &REGISTRATION
    }
}

#[Reflect]
trait DefaultMethodFixture<T: qubit_reflect::__private::codegen_v4::descriptor::Reflect> {
    fn ordinary_default(&self, value: T) -> T {
        value
    }

    fn pinned_default(self: Pin<&mut Self>, value: T) -> T {
        value
    }

    fn generic_default<U>(&self, value: U) -> U {
        value
    }
}

#[ReflectImpl(reflected_trait = DefaultMethodFixture::<u32>)]
impl DefaultMethodFixture<u32> for SpecializationFixture {}

#[ReflectImpl]
impl SpecializationFixture {
    fn map<T>(&self, value: T) -> T {
        value
    }

    fn const_value<const N: usize>(&self) -> usize {
        N
    }
}

fn type_binding() -> MethodBinding<SpecializationFixture> {
    bind_method!(SpecializationFixture::map::<u32> as fn(&SpecializationFixture, u32) -> u32)
}

fn typed_const_binding() -> MethodBinding<SpecializationFixture> {
    bind_method!(SpecializationFixture::const_value::<3usize> as fn(&SpecializationFixture) -> usize)
}

fn inferred_const_binding() -> MethodBinding<SpecializationFixture> {
    bind_method!(SpecializationFixture::const_value::<3> as fn(&SpecializationFixture) -> usize)
}

fn thread_safe_type_binding() -> MethodBinding<SpecializationFixture> {
    bind_method!(
        SpecializationFixture::map::<u32>
            as thread_safe fn(&SpecializationFixture, u32) -> u32
    )
}

fn thread_safe_default_binding() -> MethodBinding<SpecializationFixture> {
    bind_method!(
        <SpecializationFixture as DefaultMethodFixture<u32>>::ordinary_default
            as thread_safe fn(&SpecializationFixture, u32) -> u32
    )
}

fn thread_safe_pinned_default_binding() -> MethodBinding<SpecializationFixture> {
    bind_method!(
        <SpecializationFixture as DefaultMethodFixture<u32>>::pinned_default
            as thread_safe fn(Pin<&mut SpecializationFixture>, u32) -> u32
    )
}

fn registered_method_catalog() -> &'static [EffectiveMethod] {
    static METHODS: OnceLock<Box<[EffectiveMethod]>> = OnceLock::new();
    METHODS.get_or_init(|| {
        let binding = bind_method!(SpecializationFixture::map::<u32> as fn(&SpecializationFixture, u32) -> u32);
        vec![binding.effective()].into_boxed_slice()
    })
}

qubit_reflect::__private::codegen_v4::invoke::submit_method_catalog_provider!(
    SpecializationFixture,
    registered_method_catalog
);
qubit_reflect::__private::codegen_v4::registration::submit_type!(SpecializationFixture);

#[Reflect]
struct BuilderBindingFixture;

mod same_named_types {
    pub mod first {
        #[qubit_reflect::Reflect]
        pub struct Value;
    }

    pub mod second {
        #[qubit_reflect::Reflect]
        pub struct Value;
    }
}

#[ReflectImpl]
impl BuilderBindingFixture {
    pub fn map<T: qubit_reflect::__private::codegen_v4::descriptor::Reflect>(&self, value: T) -> T {
        value
    }

    pub fn mixed<T: qubit_reflect::__private::codegen_v4::descriptor::Reflect, const N: usize>(&self) -> usize {
        let _argument_type = std::any::TypeId::of::<T>();
        N
    }

    pub fn default_generic<T: qubit_reflect::__private::codegen_v4::descriptor::Reflect>(&self, value: T) -> T {
        value
    }
}

fn builder_type_binding() -> MethodBinding<BuilderBindingFixture> {
    bind_method!(BuilderBindingFixture::map::<u32> as fn(&BuilderBindingFixture, u32) -> u32)
}

fn same_named_type_binding() -> MethodBinding<BuilderBindingFixture> {
    bind_method!(
        BuilderBindingFixture::map::<same_named_types::first::Value>
            as fn(&BuilderBindingFixture, same_named_types::first::Value) -> same_named_types::first::Value
    )
}

#[Reflect]
trait ApplicationLabel<T> {
    fn label(&self) -> u32;
}

#[Reflect]
struct ApplicationLabelTarget;

#[ReflectImpl(reflected_trait = ApplicationLabel::<u8>)]
impl ApplicationLabel<u8> for ApplicationLabelTarget {
    #[reflect(skip)]
    fn label(&self) -> u32 {
        8
    }
}

#[ReflectImpl(reflected_trait = ApplicationLabel::<u64>)]
impl ApplicationLabel<u64> for ApplicationLabelTarget {
    #[reflect(skip)]
    fn label(&self) -> u32 {
        64
    }
}

fn application_label_u8_binding() -> MethodBinding<ApplicationLabelTarget> {
    bind_method!(<ApplicationLabelTarget as ApplicationLabel<u8>>::label as fn(&ApplicationLabelTarget) -> u32)
}

fn application_label_u64_binding() -> MethodBinding<ApplicationLabelTarget> {
    bind_method!(<ApplicationLabelTarget as ApplicationLabel<u64>>::label as fn(&ApplicationLabelTarget) -> u32)
}

fn application_label_impl<'ctx>(
    context: &'ctx ReflectionContext,
    argument_type: std::any::TypeId,
) -> qubit_reflect::descriptor::ImplDescriptor<'ctx> {
    context
        .type_of::<ApplicationLabelTarget>()
        .trait_impls()
        .expect("trait applications are available")
        .iter()
        .find(|implementation| {
            implementation
                .implemented_trait()
                .and_then(|trait_| trait_.arguments().get_at(0))
                .and_then(|argument| argument.type_argument().ok())
                .and_then(|argument| argument.concrete_type().ok())
                .is_some_and(|argument| argument.type_id() == argument_type)
        })
        .expect("the requested trait application is registered")
}

fn invoke_application_label(
    context: &ReflectionContext,
    argument_type: std::any::TypeId,
    target: &ApplicationLabelTarget,
) -> u32 {
    let method = application_label_impl(context, argument_type)
        .method("label")
        .expect("the trait application's method is navigable");
    let output = match method.invoke(qubit_reflect::Invocation::<qubit_reflect::value::Local>::on_ref(target)) {
        Ok(output) => output,
        Err(_) => panic!("the selected application method is callable"),
    };
    output
        .into_owned::<u32>()
        .unwrap_or_else(|_| panic!("the selected application method returns u32"))
}

fn builder_mixed_binding() -> MethodBinding<BuilderBindingFixture> {
    bind_method!(BuilderBindingFixture::mixed::<u32, 3usize> as fn(&BuilderBindingFixture) -> usize)
}

#[test]
fn generic_method_bindings_expand_to_typed_runtime_specs() {
    assert_eq!(
        DefaultMethodFixture::<u32>::generic_default(&SpecializationFixture, 7),
        7
    );
    let _ = (
        type_binding().effective(),
        typed_const_binding().effective(),
        inferred_const_binding().effective(),
        thread_safe_type_binding().effective(),
        thread_safe_default_binding().effective(),
        thread_safe_pinned_default_binding().effective(),
    );
    assert_eq!(registered_method_catalog().len(), 1);
}

#[test]
fn builder_binding_joins_the_target_method_catalog() {
    assert_eq!(BuilderBindingFixture.default_generic(11_u8), 11);
    let context = ReflectionContext::builder()
        .include_linked()
        .bind(builder_type_binding())
        .build()
        .expect("the explicit binding joins the linked target method catalog");
    let method = context
        .type_of::<BuilderBindingFixture>()
        .method("map")
        .expect("the reflected generic method is in the context");
    assert_eq!(method.name(), "map");
}

#[test]
fn specialization_api_exposes_typed_generic_arguments() {
    let context = ReflectionContext::builder()
        .include_linked()
        .bind(builder_mixed_binding())
        .build()
        .expect("the explicit mixed specialization is registered");
    let method = context
        .type_of::<BuilderBindingFixture>()
        .method("mixed")
        .expect("the reflected generic method is in the context");

    assert!(std::ptr::eq(method.context(), &context));
    assert_eq!(
        method.target().rust_type_name(),
        std::any::type_name::<BuilderBindingFixture>()
    );
    assert!(matches!(
        method.namespace(),
        qubit_reflect::descriptor::MethodNamespace::Inherent { .. }
    ));
    assert_eq!(
        method.qualified_name().to_string(),
        format!("{}::mixed", std::any::type_name::<BuilderBindingFixture>())
    );
    assert_eq!(method.specializations().len(), 1);
    let specialization = method
        .specialization(
            &qubit_reflect::descriptor::MethodArguments::new()
                .type_of::<u32>()
                .constant(3usize),
        )
        .expect("the supplied generic arguments select the binding");
    let arguments = specialization.arguments();
    assert_eq!(arguments.len(), 2);
    assert_eq!(
        arguments
            .argument("T")
            .expect("the type parameter is preserved")
            .type_argument()
            .expect("T is a type argument")
            .concrete_type()
            .expect("the concrete Rust type is preserved")
            .rust_type_name(),
        "u32"
    );
    assert_eq!(
        arguments
            .argument("N")
            .expect("the const parameter is preserved")
            .const_argument()
            .expect("N is a const argument")
            .expression(),
        qubit_reflect::descriptor::ConstExpression::UnsignedInteger(3)
    );
}

#[test]
fn impl_method_navigation_uses_context_specializations() {
    let context = ReflectionContext::builder()
        .include_linked()
        .bind(builder_mixed_binding())
        .build()
        .expect("the explicit mixed specialization is registered");
    let target = context.type_of::<BuilderBindingFixture>();
    let by_type = target.method("mixed").expect("type navigation finds mixed");
    let implementation = target
        .inherent_impls()
        .expect("inherent impl applications are available")
        .iter()
        .next()
        .expect("the fixture has an inherent impl");
    let by_impl = implementation.method("mixed").expect("impl navigation finds mixed");

    assert_eq!(by_type.specializations().len(), 1);
    assert_eq!(by_impl.specializations().len(), 1);
    let arguments = qubit_reflect::descriptor::MethodArguments::new()
        .type_of::<u32>()
        .constant(3usize);
    let type_specialization = by_type.specialization(&arguments).expect("type path resolves binding");
    let impl_specialization = by_impl.specialization(&arguments).expect("impl path resolves binding");
    assert_eq!(
        type_specialization.arguments().len(),
        impl_specialization.arguments().len()
    );

    let target_value = BuilderBindingFixture;
    let type_output = match type_specialization.invoke(
        qubit_reflect::Invocation::<qubit_reflect::value::Local>::on_ref(&target_value),
    ) {
        Ok(output) => output,
        Err(failure) => panic!("type navigation failed: {:?}", failure.error().kind()),
    };
    let impl_output = match impl_specialization.invoke(
        qubit_reflect::Invocation::<qubit_reflect::value::Local>::on_ref(&target_value),
    ) {
        Ok(output) => output,
        Err(failure) => panic!("impl navigation failed: {:?}", failure.error().kind()),
    };
    assert_eq!(
        type_output
            .into_owned::<usize>()
            .unwrap_or_else(|_| panic!("type result is usize")),
        3
    );
    assert_eq!(
        impl_output
            .into_owned::<usize>()
            .unwrap_or_else(|_| panic!("impl result is usize")),
        3
    );
}

#[test]
fn specialization_selection_uses_exact_type_identity() {
    let context = ReflectionContext::builder()
        .include_linked()
        .bind(builder_type_binding())
        .build()
        .expect("the explicit type specialization is registered");
    let method = context
        .type_of::<BuilderBindingFixture>()
        .method("map")
        .expect("the reflected generic method is in the context");

    assert!(
        method
            .specialization(&qubit_reflect::descriptor::MethodArguments::new().type_of::<u32>())
            .is_ok()
    );
    assert!(matches!(
        method.specialization(&qubit_reflect::descriptor::MethodArguments::new().type_of::<u64>()),
        Err(qubit_reflect::error::LookupError::Missing { .. })
    ));
}

#[test]
fn specialization_selection_distinguishes_same_named_types() {
    let context = ReflectionContext::builder()
        .include_linked()
        .bind(same_named_type_binding())
        .build()
        .expect("the explicit type specialization is registered");
    let method = context
        .type_of::<BuilderBindingFixture>()
        .method("map")
        .expect("the reflected generic method is in the context");

    assert!(
        method
            .specialization(
                &qubit_reflect::descriptor::MethodArguments::new().type_of::<same_named_types::first::Value>(),
            )
            .is_ok()
    );
    assert!(matches!(
        method.specialization(
            &qubit_reflect::descriptor::MethodArguments::new().type_of::<same_named_types::second::Value>(),
        ),
        Err(qubit_reflect::error::LookupError::Missing { .. })
    ));
}

#[test]
fn method_specializations_do_not_leak_between_contexts() {
    let first_context = ReflectionContext::builder()
        .include_linked()
        .bind(builder_type_binding())
        .build()
        .expect("the first context owns its u32 binding");
    let second_context = ReflectionContext::builder()
        .include_linked()
        .bind(same_named_type_binding())
        .build()
        .expect("the second context owns its Value binding");

    let first_method = first_context
        .type_of::<BuilderBindingFixture>()
        .inherent_impls()
        .expect("first context impls are available")
        .iter()
        .next()
        .expect("first context has an inherent impl")
        .method("map")
        .expect("first context method is available");
    let second_method = second_context
        .type_of::<BuilderBindingFixture>()
        .inherent_impls()
        .expect("second context impls are available")
        .iter()
        .next()
        .expect("second context has an inherent impl")
        .method("map")
        .expect("second context method is available");

    assert!(
        first_method
            .specialization(&qubit_reflect::descriptor::MethodArguments::new().type_of::<u32>())
            .is_ok()
    );
    assert!(matches!(
        first_method.specialization(
            &qubit_reflect::descriptor::MethodArguments::new().type_of::<same_named_types::first::Value>()
        ),
        Err(qubit_reflect::error::LookupError::Missing { .. })
    ));
    assert!(matches!(
        second_method.specialization(&qubit_reflect::descriptor::MethodArguments::new().type_of::<u32>()),
        Err(qubit_reflect::error::LookupError::Missing { .. })
    ));
    assert!(
        second_method
            .specialization(
                &qubit_reflect::descriptor::MethodArguments::new().type_of::<same_named_types::first::Value>()
            )
            .is_ok()
    );
}

#[test]
fn same_named_trait_application_methods_stay_with_their_context_application() {
    let u8_context = ReflectionContext::builder()
        .include_linked()
        .bind(application_label_u8_binding())
        .build()
        .expect("the first trait application is bound in its context");
    let u64_context = ReflectionContext::builder()
        .include_linked()
        .bind(application_label_u64_binding())
        .build()
        .expect("the second trait application is bound in its context");
    let target = ApplicationLabelTarget;
    assert_eq!(
        invoke_application_label(&u8_context, std::any::TypeId::of::<u8>(), &target),
        8
    );
    assert_eq!(
        invoke_application_label(&u64_context, std::any::TypeId::of::<u64>(), &target),
        64
    );

    let shared_context = ReflectionContext::builder()
        .include_linked()
        .bind(application_label_u8_binding())
        .bind(application_label_u64_binding())
        .build()
        .expect("both same-named trait applications are bound in one context");
    assert_eq!(
        invoke_application_label(&shared_context, std::any::TypeId::of::<u8>(), &target),
        8
    );
    assert_eq!(
        invoke_application_label(&shared_context, std::any::TypeId::of::<u64>(), &target),
        64
    );
}

#[test]
fn specialization_selection_matches_typed_const_arguments_exactly() {
    let context = ReflectionContext::builder()
        .include_linked()
        .bind(builder_mixed_binding())
        .build()
        .expect("the explicit mixed specialization is registered");
    let method = context
        .type_of::<BuilderBindingFixture>()
        .method("mixed")
        .expect("the reflected generic method is in the context");

    assert!(
        method
            .specialization(
                &qubit_reflect::descriptor::MethodArguments::new()
                    .type_of::<u32>()
                    .constant(3usize),
            )
            .is_ok()
    );
    for arguments in [
        qubit_reflect::descriptor::MethodArguments::new()
            .type_of::<u32>()
            .constant(3u32),
        qubit_reflect::descriptor::MethodArguments::new()
            .type_of::<u32>()
            .constant(4usize),
        qubit_reflect::descriptor::MethodArguments::new()
            .type_of::<u32>()
            .constant(3.0f64),
    ] {
        assert!(matches!(
            method.specialization(&arguments),
            Err(qubit_reflect::error::LookupError::Missing { .. })
        ));
    }
}

#[test]
fn builder_rejects_duplicate_method_specialization_structurally() {
    let result = ReflectionContext::builder()
        .include_linked()
        .bind(builder_type_binding())
        .bind(builder_type_binding())
        .build();
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("the same specialization cannot be registered twice"),
    };
    assert!(matches!(
        error.primary_issue(),
        qubit_reflect::error::ContextIssue::DuplicateMethodSpecialization { method, arguments, .. }
            if method.display_name.as_ref() == "map" && arguments.len() == 1
    ));
}
