// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

//! Regression tests for compiler-resolved default trait arguments.

use std::marker::PhantomData;

use qubit_reflect::Invocation;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;
use qubit_reflect::descriptor::ConstExpression;
use qubit_reflect::reflect_external_trait;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

#[Reflect]
trait Defaulted<T = u8, const N: usize = 5> {
    fn default_value(&self) -> usize {
        N
    }
}

#[allow(dead_code)]
trait ExternalDefaulted<T = u8, const N: usize = 5> {}

reflect_external_trait! {
    pub ExternalDefaultedHandle = ExternalDefaulted;
}

use self::Defaulted as DefaultedAlias;

#[Reflect]
struct OmittedTarget;
#[Reflect]
struct ExplicitTarget;
#[Reflect]
struct PartialTarget;
#[Reflect]
struct AliasTarget;
#[Reflect]
struct GenericTarget<T: 'static>(PhantomData<T>);
#[Reflect]
struct ExternalTarget;

#[ReflectImpl(reflected_trait = Defaulted)]
impl Defaulted for OmittedTarget {}

#[ReflectImpl(reflected_trait = Defaulted)]
impl Defaulted<u8, 5> for ExplicitTarget {}

#[ReflectImpl(reflected_trait = Defaulted)]
impl Defaulted<u16> for PartialTarget {}

#[ReflectImpl(reflected_trait = Defaulted)]
impl DefaultedAlias for AliasTarget {}

#[ReflectImpl(reflected_trait = Defaulted::<T>)]
impl<T: 'static> Defaulted<T> for GenericTarget<T> {}

#[ReflectImpl(external_trait = ExternalDefaultedHandle)]
impl ExternalDefaulted for ExternalTarget {}

fn application_for<T: 'static>(context: &ReflectionContext) -> qubit_reflect::descriptor::TraitDescriptor<'_> {
    context
        .type_by_name(std::any::type_name::<T>())
        .expect("reflected target is registered")
        .trait_impls()
        .expect("target trait impls are available")
        .iter()
        .find_map(|implementation| implementation.implemented_trait())
        .expect("target has its trait application")
}

#[test]
fn default_and_explicit_arguments_share_one_application_identity() {
    let context = ReflectionContext::builder()
        .include_linked()
        .build()
        .expect("context builds with defaulted trait applications");
    let omitted = application_for::<OmittedTarget>(&context);
    let explicit = application_for::<ExplicitTarget>(&context);

    assert_eq!(omitted.arguments().len(), 2);
    assert_eq!(
        omitted
            .arguments()
            .argument("T")
            .expect("type default exists")
            .type_argument()
            .expect("T is a type argument")
            .concrete_type()
            .expect("default type is concrete")
            .type_id(),
        std::any::TypeId::of::<u8>(),
    );
    assert_eq!(
        omitted
            .arguments()
            .argument("N")
            .expect("const default exists")
            .const_argument()
            .expect("N is a const argument")
            .expression(),
        ConstExpression::UnsignedInteger(5),
    );
    assert!(
        context
            .type_by_name(std::any::type_name::<OmittedTarget>())
            .expect("omitted target is registered")
            .find_trait_impl(explicit)
            .expect("same default application is a valid lookup")
            .is_some(),
    );

    let method = context
        .type_of::<OmittedTarget>()
        .method("default_value")
        .expect("the omitted application selects its default method");
    let output = match method.invoke(Invocation::<Local>::on_ref(&OmittedTarget)) {
        Ok(output) => match output.into_owned::<usize>() {
            Ok(value) => value,
            Err(_) => panic!("the default method returns usize"),
        },
        Err(_) => panic!("the omitted default application invokes its method"),
    };
    assert_eq!(output, 5);

    let alias_method = context
        .type_of::<AliasTarget>()
        .method("default_value")
        .expect("the aliased omitted application selects its default method");
    let alias_output = match alias_method.invoke(Invocation::<Local>::on_ref(&AliasTarget)) {
        Ok(output) => match output.into_owned::<usize>() {
            Ok(value) => value,
            Err(_) => panic!("the default method returns usize"),
        },
        Err(_) => panic!("the aliased default application invokes its method"),
    };
    assert_eq!(alias_output, 5);
}

#[test]
fn partial_defaults_aliases_and_generic_targets_keep_resolved_arguments() {
    let context = ReflectionContext::builder()
        .include_linked()
        .build()
        .expect("context builds with defaulted trait applications");
    let partial = application_for::<PartialTarget>(&context);
    assert_eq!(
        partial
            .arguments()
            .argument("T")
            .expect("explicit type argument exists")
            .type_argument()
            .expect("T is a type argument")
            .concrete_type()
            .expect("explicit type is concrete")
            .type_id(),
        std::any::TypeId::of::<u16>(),
    );
    assert_eq!(
        partial
            .arguments()
            .argument("N")
            .expect("partial application receives const default")
            .const_argument()
            .expect("N is a const argument")
            .expression(),
        ConstExpression::UnsignedInteger(5),
    );

    let alias = application_for::<AliasTarget>(&context);
    assert!(alias.path().ends_with("DefaultedAlias"));
    assert!(alias.find_definition().is_some());

    let method = context
        .type_of::<PartialTarget>()
        .method("default_value")
        .expect("the partial application selects its default method");
    let output = match method.invoke(Invocation::<Local>::on_ref(&PartialTarget)) {
        Ok(output) => match output.into_owned::<usize>() {
            Ok(value) => value,
            Err(_) => panic!("the default method returns usize"),
        },
        Err(_) => panic!("the partial default application invokes its method"),
    };
    assert_eq!(output, 5);
    let omitted_method = context
        .type_of::<OmittedTarget>()
        .method("default_value")
        .expect("the omitted application selects its default method");
    let output = match omitted_method.invoke(Invocation::<ThreadSafe>::on_ref(&OmittedTarget)) {
        Ok(output) => match output.into_owned::<usize>() {
            Ok(value) => value,
            Err(_) => panic!("the ThreadSafe default method returns usize"),
        },
        Err(_) => panic!("the omitted default application selects a ThreadSafe adapter"),
    };
    assert_eq!(output, 5);
}

#[test]
fn application_hook_keeps_generic_target_instantiations_distinct() {
    let (u8_arguments, u8_identity, _, _) = <GenericTarget<u8> as Defaulted<u8>>::__qbr_trait_application_metadata();
    let (string_arguments, string_identity, _, _) =
        <GenericTarget<String> as Defaulted<String>>::__qbr_trait_application_metadata();

    assert_eq!(u8_arguments.len(), 2);
    assert_eq!(string_arguments.len(), 2);
    assert!(u8_identity[0] != string_identity[0]);
    assert!(u8_identity[1] == string_identity[1]);
    let _ = u8_arguments;
    let _ = string_arguments;
    match u8_identity[0] {
        qubit_reflect::__private::codegen_v4::traits::ArgumentIdentity::Type(type_id) => {
            assert_eq!(type_id(), std::any::TypeId::of::<u8>());
        }
        _ => panic!("the u8 instantiation has a typed type identity"),
    }
    match string_identity[0] {
        qubit_reflect::__private::codegen_v4::traits::ArgumentIdentity::Type(type_id) => {
            assert_eq!(type_id(), std::any::TypeId::of::<String>());
        }
        _ => panic!("the String instantiation has a typed type identity"),
    }
}

#[test]
fn external_handle_does_not_infer_unwritten_default_arguments() {
    let context = ReflectionContext::builder()
        .include_linked()
        .build()
        .expect("context keeps external defaulted trait incomplete");
    let application = application_for::<ExternalTarget>(&context);

    assert_eq!(application.arguments().len(), 0);
    assert!(application.find_definition().is_some());
    assert_eq!(
        application.completeness(),
        qubit_reflect::__private::codegen_v4::traits::Completeness::ExternalIncomplete,
    );
}
