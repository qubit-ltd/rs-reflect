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

    pub fn mixed<T: qubit_reflect::__private::codegen_v4::descriptor::Reflect, const N: usize>(
        &self,
        _value: T,
    ) -> usize {
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

fn builder_mixed_binding() -> MethodBinding<BuilderBindingFixture> {
    bind_method!(BuilderBindingFixture::mixed::<u32, 3usize> as fn(&BuilderBindingFixture, u32) -> usize)
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
