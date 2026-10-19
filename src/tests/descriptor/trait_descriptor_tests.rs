// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================

//! Unit coverage for context-bound trait declarations and applications.

use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::TypeDescriptor;

#[cfg(feature = "derive")]
#[crate::Reflect]
struct Fixture;

#[cfg(feature = "derive")]
#[crate::Reflect]
trait FixtureContract {
    type Output: crate::Reflect;

    const LIMIT: u32;

    fn convert(&self, value: u32) -> Self::Output;
}

#[cfg(feature = "derive")]
#[crate::ReflectImpl]
impl FixtureContract for Fixture {
    type Output = u16;

    const LIMIT: u32 = 17;

    fn convert(&self, value: u32) -> Self::Output {
        u16::try_from(value).expect("test value fits the reflected output type")
    }
}

#[test]
#[cfg(feature = "derive")]
fn test_trait_context_exposes_definition_and_impl_application() {
    let context = ReflectionContext::global();
    context.initialize().expect("linked trait metadata validates");
    let definition = context
        .trait_definition_by_name(concat!(module_path!(), "::FixtureContract"))
        .expect("trait declaration is registered");

    assert_eq!(definition.name(), "FixtureContract");
    assert_eq!(definition.methods().len(), 1);
    assert_eq!(definition.method("convert").unwrap().name(), "convert");
    assert_eq!(definition.associated_types().len(), 1);
    assert_eq!(definition.associated_consts().len(), 1);

    let implementation = TypeDescriptor::of::<Fixture>()
        .trait_impls()
        .expect("linked trait implementations are available")
        .iter()
        .find(|implementation| {
            implementation
                .implemented_trait()
                .is_some_and(|trait_| trait_.name() == "FixtureContract")
        })
        .expect("the fixture implementation is registered");

    assert_eq!(implementation.method("convert").unwrap().name(), "convert");
    assert_eq!(implementation.associated_types().len(), 1);
    assert_eq!(implementation.associated_consts().len(), 1);
}
