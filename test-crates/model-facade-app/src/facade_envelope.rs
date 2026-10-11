// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Generic model facade fixture declaration.

use model_facade_derive::model_reflect;
#[cfg(test)]
use model_facade_runtime::Reflect;
#[cfg(test)]
use model_facade_runtime::ReflectRegistry;

/// A generic facade fixture that exercises generated expression metadata.
#[model_reflect]
pub struct FacadeEnvelope<T> {
    /// The reflected payload.
    pub value: T,
}

/// Verifies the selected provider resolves to the registered generic
/// definition.
#[cfg(test)]
pub(crate) fn assert_selected_definition_contract(registry: &ReflectRegistry) {
    let selected = __model_facade_definition_FacadeEnvelope();
    let registered = registry
        .definition(selected.id())
        .expect("facade-selected generic definition registers");
    assert!(std::ptr::eq(registered, selected));
    let concrete = FacadeEnvelope::<String>::type_descriptor()
        .type_definition()
        .expect("concrete facade type retains its generic definition");
    assert!(std::ptr::eq(concrete, selected));
}
