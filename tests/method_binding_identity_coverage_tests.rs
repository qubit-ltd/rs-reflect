// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::bind_method;
use qubit_reflect::expression::ConcreteTypeExpression;
use qubit_reflect::expression::GenericArgument;
use qubit_reflect::expression::TypeExpression;
use qubit_reflect::identity::FragmentIdentity;
use qubit_reflect::registry::RegistrySnapshotBuilder;

#[Reflect]
struct GenericService<T>(T);

#[ReflectImpl]
impl<T> GenericService<T> {
    fn read(&self) -> &T {
        &self.0
    }
}

#[test]
fn binding_keeps_stable_impl_identity_and_concrete_generic_arguments() {
    let binding = bind_method!(GenericService::<u8>::read as fn(&GenericService<u8>) -> &u8);
    let repeated_binding = bind_method!(GenericService::<u8>::read as fn(&GenericService<u8>) -> &u8);
    assert_eq!(binding.identity(), repeated_binding.identity());

    let mut builder = RegistrySnapshotBuilder::new();
    builder.include_linked_fragments();
    builder.add_method_binding(
        binding,
        FragmentIdentity::new("binding-identity-test", module_path!(), line!(), 1, "binding", 1),
    );
    let registry = builder.build().expect("the generic impl binding resolves");

    let [resolved] = registry.method_bindings() else {
        panic!("exactly one generic method specialization should be registered");
    };
    assert_eq!(resolved.impl_identity(), resolved.implementation().fragment_identity());
    assert_eq!(
        resolved.impl_arguments(),
        &[GenericArgument::Type(TypeExpression::Concrete(
            ConcreteTypeExpression::new(["u8"], []).expect("the primitive type path is valid"),
        ))],
    );
}
