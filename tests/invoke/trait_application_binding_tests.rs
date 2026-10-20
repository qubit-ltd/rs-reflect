// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

extern crate self as integration_tests;

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;
use qubit_reflect::bind_method;
use qubit_reflect::lookup::Lookup;

#[Reflect]
trait ApplicationMethod<T> {
    fn identify<M: qubit_reflect::__private::codegen_v4::descriptor::Reflect>(&self, _: M) -> String;
}

#[Reflect]
struct ApplicationMethodTarget;

#[ReflectImpl(reflected_trait = ApplicationMethod::<u8>)]
impl ApplicationMethod<u8> for ApplicationMethodTarget {
    fn identify<M: qubit_reflect::__private::codegen_v4::descriptor::Reflect>(&self, _: M) -> String {
        "u8 application".to_owned()
    }
}

#[ReflectImpl(reflected_trait = ApplicationMethod::<u16>)]
impl ApplicationMethod<u16> for ApplicationMethodTarget {
    fn identify<M: qubit_reflect::__private::codegen_v4::descriptor::Reflect>(&self, _: M) -> String {
        "u16 application".to_owned()
    }
}

#[test]
fn bind_method_selects_the_exact_generic_trait_application() {
    let u8_binding = bind_method!(
        <ApplicationMethodTarget as crate::invoke::trait_application_binding_tests::ApplicationMethod<u8>>::identify::<
            u32,
        > as fn(&ApplicationMethodTarget, u32) -> String
    );
    let u16_binding = bind_method!(
        <ApplicationMethodTarget as integration_tests::invoke::trait_application_binding_tests::ApplicationMethod<
            u16,
        >>::identify::<u32> as fn(&ApplicationMethodTarget, u32) -> String
    );
    let context = ReflectionContext::builder()
        .include_linked()
        .bind(u8_binding)
        .bind(u16_binding)
        .build()
        .expect("both concrete trait applications and bindings are registered");
    let target = context.type_of::<ApplicationMethodTarget>();
    let Lookup::Ambiguous(methods) = target
        .lookup_method("identify")
        .expect("both trait methods remain available")
    else {
        panic!("the distinct trait applications produce two method candidates")
    };
    assert_eq!(methods.len(), 2);
    assert!(methods.iter().all(|method| method.name() == "identify"));
}

#[Reflect]
pub trait DefaultedApplicationMethod<
    T: qubit_reflect::__private::codegen_v4::descriptor::Reflect = u8,
    const N: usize = 5,
>
{
    fn identify(&self) -> String;
}

mod defaulted_trait_alias {
    pub use super::DefaultedApplicationMethod as Imported;
}

mod super_path_provider {
    use qubit_reflect::Reflect;
    use qubit_reflect::ReflectImpl;

    #[Reflect]
    pub trait Service {
        fn value(&self) -> u32;
    }

    #[Reflect]
    pub struct Target;

    #[ReflectImpl(reflected_trait = Service)]
    impl Service for Target {
        #[reflect(skip)]
        fn value(&self) -> u32 {
            42
        }
    }

    pub mod child {
        use qubit_reflect::bind_method;

        pub fn binding() -> qubit_reflect::__private::codegen_v4::registration::MethodBinding<super::Target> {
            bind_method!(
                <super::Target as super::Service>::value as fn(&super::Target) -> u32,
                reflected_trait = super::Service
            )
        }
    }
}

#[Reflect]
struct FullyDefaultedApplicationTarget;

#[ReflectImpl(reflected_trait = DefaultedApplicationMethod)]
impl DefaultedApplicationMethod for FullyDefaultedApplicationTarget {
    #[reflect(skip)]
    fn identify(&self) -> String {
        format!("{}:{}", std::any::type_name::<u8>(), 5)
    }
}

#[Reflect]
struct PartiallyDefaultedApplicationTarget;

#[ReflectImpl(reflected_trait = DefaultedApplicationMethod::<u16>)]
impl DefaultedApplicationMethod<u16> for PartiallyDefaultedApplicationTarget {
    #[reflect(skip)]
    fn identify(&self) -> String {
        format!("{}:{}", std::any::type_name::<u16>(), 5)
    }
}

#[test]
fn bind_method_uses_compiler_resolved_default_trait_arguments() {
    let full_binding = bind_method!(
        <FullyDefaultedApplicationTarget as defaulted_trait_alias::Imported>::identify
            as fn(&FullyDefaultedApplicationTarget) -> String,
        reflected_trait = self::DefaultedApplicationMethod
    );
    let full_context = ReflectionContext::builder()
        .include_linked()
        .bind(full_binding)
        .build()
        .expect("the fully defaulted trait application matches its method binding");
    let full_target = FullyDefaultedApplicationTarget;
    let full_method = full_context
        .type_of::<FullyDefaultedApplicationTarget>()
        .method("identify")
        .expect("the defaulted method binding is registered");
    let full_output = match full_method.invoke(qubit_reflect::Invocation::<qubit_reflect::value::Local>::on_ref(
        &full_target,
    )) {
        Ok(output) => output,
        Err(_) => panic!("the defaulted method can be invoked"),
    };
    let full_value = match full_output.into_owned::<String>() {
        Ok(value) => value,
        Err(_) => panic!("the method returns a String"),
    };
    assert_eq!(full_value, "u8:5",);

    let partial_binding = bind_method!(
        <PartiallyDefaultedApplicationTarget as DefaultedApplicationMethod<u16>>::identify
            as fn(&PartiallyDefaultedApplicationTarget) -> String,
        reflected_trait = DefaultedApplicationMethod
    );
    let partial_context = ReflectionContext::builder()
        .include_linked()
        .bind(partial_binding)
        .build()
        .expect("the partially defaulted trait application matches its method binding");
    let partial_target = PartiallyDefaultedApplicationTarget;
    let partial_method = partial_context
        .type_of::<PartiallyDefaultedApplicationTarget>()
        .method("identify")
        .expect("the partially defaulted method binding is registered");
    let partial_output = match partial_method.invoke(qubit_reflect::Invocation::<qubit_reflect::value::Local>::on_ref(
        &partial_target,
    )) {
        Ok(output) => output,
        Err(_) => panic!("the partially defaulted method can be invoked"),
    };
    let partial_value = match partial_output.into_owned::<String>() {
        Ok(value) => value,
        Err(_) => panic!("the method returns a String"),
    };
    assert_eq!(partial_value, "u16:5",);
}

#[test]
fn bind_method_resolves_super_path_provider_from_the_call_site_module() {
    let context = ReflectionContext::builder()
        .include_linked()
        .bind(super_path_provider::child::binding())
        .build()
        .expect("the super path provider links to the reflected trait definition");
    let target = super_path_provider::Target;
    let method = context
        .type_of::<super_path_provider::Target>()
        .method("value")
        .expect("the super path trait method is registered");
    let output = match method.invoke(qubit_reflect::Invocation::<qubit_reflect::value::Local>::on_ref(
        &target,
    )) {
        Ok(output) => output,
        Err(_) => panic!("the super path binding invokes its method"),
    };
    let value = match output.into_owned::<u32>() {
        Ok(value) => value,
        Err(_) => panic!("the method returns u32"),
    };
    assert_eq!(value, 42);
}
