// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::__private::codegen_v4::descriptor::Reflect;
use qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration;
use qubit_reflect::Invocation;
use qubit_reflect::Reflect as ReflectDerive;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;
use qubit_reflect::bind_method;
use qubit_reflect::descriptor::GenericExpressionArgumentKind;
use qubit_reflect::reflect_external_trait;
use qubit_reflect::register_method_binding;
use qubit_reflect::value::Local;

struct GenericMethodOwner;

impl Reflect for GenericMethodOwner {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        static REGISTRATION: TypeRegistration<GenericMethodOwner> = TypeRegistration::new();
        &REGISTRATION
    }
}

#[ReflectImpl]
impl GenericMethodOwner {
    fn convert<T: Reflect>(&self, value: T) -> T {
        value
    }
}

register_method_binding!(GenericMethodOwner::convert::<u64> as fn(&GenericMethodOwner, u64) -> u64);
qubit_reflect::__private::codegen_v4::registration::submit_type!(GenericMethodOwner);

struct LifetimeBindingTarget;

impl Reflect for LifetimeBindingTarget {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        TypeRegistration::get_or_init_with(TypeRegistration::new)
    }
}

qubit_reflect::__private::codegen_v4::registration::submit_type!(LifetimeBindingTarget);

#[ReflectDerive]
trait LifetimeBindingTrait<'a, T> {
    fn bound(&self) -> usize;
}

#[ReflectImpl(reflected_trait = LifetimeBindingTrait::<'static, u8>)]
impl LifetimeBindingTrait<'static, u8> for LifetimeBindingTarget {
    #[reflect(skip)]
    fn bound(&self) -> usize {
        41
    }
}

register_method_binding!(
    <LifetimeBindingTarget as LifetimeBindingTrait<'static, u8>>::bound as fn(&LifetimeBindingTarget) -> usize
);

#[allow(dead_code)]
trait ExternalLifetimeBindingTrait<'a, T> {
    fn bound(&self) -> usize;
}

reflect_external_trait! {
    ExternalLifetimeBindingHandle = ExternalLifetimeBindingTrait;
}

struct ExternalLifetimeBindingTarget;

impl Reflect for ExternalLifetimeBindingTarget {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        TypeRegistration::get_or_init_with(TypeRegistration::new)
    }
}

qubit_reflect::__private::codegen_v4::registration::submit_type!(ExternalLifetimeBindingTarget);

#[ReflectImpl(external_trait = ExternalLifetimeBindingHandle)]
impl ExternalLifetimeBindingTrait<'static, u8> for ExternalLifetimeBindingTarget {
    #[reflect(skip)]
    fn bound(&self) -> usize {
        43
    }
}

#[test]
fn test_public_register_method_binding_accepts_typed_method_path() {
    assert_eq!(qubit_reflect::__private::codegen_v4::PROTOCOL_VERSION, 4,);
}

#[test]
fn test_lifetime_trait_method_binding_matches_runtime_application_identity() {
    let context = ReflectionContext::builder()
        .include_linked()
        .build()
        .expect("the linked trait application and method catalog build a context");
    let target = LifetimeBindingTarget;
    let method = context
        .type_of::<LifetimeBindingTarget>()
        .method("bound")
        .expect("the explicit lifetime trait binding is registered");
    let output = match method.invoke(Invocation::<Local>::on_ref(&target)) {
        Ok(output) => output,
        Err(_) => panic!("the binding identity matches the registered trait application"),
    };
    let value = match output.into_owned::<usize>() {
        Ok(value) => value,
        Err(_) => panic!("the method returns usize"),
    };
    assert_eq!(value, 41);
}

#[test]
fn test_external_lifetime_trait_application_keeps_typed_identity_and_source() {
    let context = ReflectionContext::builder()
        .include_linked()
        .build()
        .expect("the external lifetime trait handle builds a context");
    let applications = context
        .trait_applications_by_path("ExternalLifetimeBindingTrait")
        .expect("external trait applications are queryable by source path");
    assert_eq!(applications.len(), 1);
    assert_eq!(applications[0].arguments().len(), 1);
    assert_eq!(
        applications[0].completeness(),
        qubit_reflect::__private::codegen_v4::traits::Completeness::ExternalIncomplete,
    );
    assert_eq!(
        applications[0].supertrait_completeness(),
        qubit_reflect::__private::codegen_v4::traits::SupertraitStatus::Incomplete,
    );
    let definition = applications[0]
        .find_definition()
        .expect("the typed external handle links the application");
    assert_eq!(
        definition.completeness(),
        qubit_reflect::__private::codegen_v4::traits::Completeness::ExternalIncomplete,
    );
    assert_eq!(definition.path(), "ExternalLifetimeBindingTrait",);
    let implementation = context
        .impl_definitions()
        .expect("external impl source definitions are available")
        .iter()
        .find(|implementation| {
            implementation
                .implemented_trait()
                .is_some_and(|trait_| trait_.path() == "ExternalLifetimeBindingTrait")
        })
        .expect("the external lifetime impl source expression is retained");
    let source_arguments = implementation
        .implemented_trait()
        .expect("the impl retains its trait expression")
        .arguments();
    assert_eq!(source_arguments.len(), 2);
    assert!(matches!(
        source_arguments.get(0).expect("source lifetime argument").kind(),
        GenericExpressionArgumentKind::Lifetime(_)
    ));
    assert!(matches!(
        source_arguments.get(1).expect("source type argument").kind(),
        GenericExpressionArgumentKind::Type(_)
    ));
}

#[allow(dead_code)]
trait ExternalAliasBindingTrait<T> {
    fn value(&self) -> usize;
}

use self::ExternalAliasBindingTrait as ImportedExternalAliasBindingTrait;
use self::ExternalAliasBindingTrait as AnotherExternalAliasBindingTrait;

reflect_external_trait! {
    ExternalAliasBindingHandle = ExternalAliasBindingTrait;
}

struct ExternalAliasBindingTarget;

impl Reflect for ExternalAliasBindingTarget {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        TypeRegistration::get_or_init_with(TypeRegistration::new)
    }
}

qubit_reflect::__private::codegen_v4::registration::submit_type!(ExternalAliasBindingTarget);

#[ReflectImpl(external_trait = ExternalAliasBindingHandle)]
impl ImportedExternalAliasBindingTrait<u16> for ExternalAliasBindingTarget {
    #[reflect(skip)]
    fn value(&self) -> usize {
        47
    }
}

struct SecondExternalAliasBindingTarget;

impl Reflect for SecondExternalAliasBindingTarget {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        TypeRegistration::get_or_init_with(TypeRegistration::new)
    }
}

qubit_reflect::__private::codegen_v4::registration::submit_type!(SecondExternalAliasBindingTarget);

#[ReflectImpl(external_trait = ExternalAliasBindingHandle)]
impl AnotherExternalAliasBindingTrait<u16> for SecondExternalAliasBindingTarget {
    #[reflect(skip)]
    fn value(&self) -> usize {
        53
    }
}

register_method_binding!(
    <SecondExternalAliasBindingTarget as AnotherExternalAliasBindingTrait<u16>>::value
        as fn(&SecondExternalAliasBindingTarget) -> usize,
    external_trait = ExternalAliasBindingHandle
);

#[test]
fn test_external_trait_handle_binds_alias_path_and_invokes_method() {
    let binding = bind_method!(
        <ExternalAliasBindingTarget as ImportedExternalAliasBindingTrait<u16>>::value
            as fn(&ExternalAliasBindingTarget) -> usize,
        external_trait = ExternalAliasBindingHandle
    );
    let context = ReflectionContext::builder()
        .include_linked()
        .bind(binding)
        .build()
        .expect("the external handle links both alias-path method bindings");
    let target = ExternalAliasBindingTarget;
    let method = context
        .type_of::<ExternalAliasBindingTarget>()
        .method("value")
        .expect("the external method binding is registered");
    let output = match method.invoke(Invocation::<Local>::on_ref(&target)) {
        Ok(output) => output,
        Err(_) => panic!("the external method can be invoked"),
    };
    let value = match output.into_owned::<usize>() {
        Ok(value) => value,
        Err(_) => panic!("the external method returns usize"),
    };
    assert_eq!(value, 47);

    let second_target = SecondExternalAliasBindingTarget;
    let second_method = context
        .type_of::<SecondExternalAliasBindingTarget>()
        .method("value")
        .expect("the second alias method binding is registered");
    let second_output = match second_method.invoke(Invocation::<Local>::on_ref(&second_target)) {
        Ok(output) => output,
        Err(_) => panic!("the second alias method can be invoked"),
    };
    let second_value = match second_output.into_owned::<usize>() {
        Ok(value) => value,
        Err(_) => panic!("the second alias method returns usize"),
    };
    assert_eq!(second_value, 53);
}
