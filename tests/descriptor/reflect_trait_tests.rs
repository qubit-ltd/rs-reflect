// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Integration tests for reflected trait declaration metadata.

use qubit_reflect::Reflect;
use qubit_reflect::registry::ReflectRegistry;

#[Reflect]
// This declaration is inspected through its reflected metadata only.
#[allow(dead_code)]
trait GenericService<T> {
    fn required(&self, value: T);

    fn convert<U: Clone>(&self, value: U) -> U;

    #[reflect(skip)]
    fn omitted(&self);
}

#[test]
fn test_reflect_trait_preserves_identity_generics_and_visible_method_metadata() {
    let registry = ReflectRegistry::initialize().expect("trait declarations register");
    let definition = registry
        .trait_definition_by_path(concat!(module_path!(), "::GenericService"))
        .expect("the reflected trait definition is registered");

    assert_eq!(definition.rust_name(), "GenericService");
    assert_eq!(definition.generic_definition().parameters().len(), 1);
    assert_eq!(
        definition
            .methods()
            .iter()
            .map(|method| method.rust_name())
            .collect::<Vec<_>>(),
        ["required", "convert"]
    );
    assert_eq!(definition.methods()[1].generic_definition().parameters().len(), 1);
    assert!(
        definition
            .methods()
            .iter()
            .all(|method| method.rust_name() != "omitted")
    );
}
