// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::marker::PhantomData;

use qubit_reflect::TypeDescriptor;

#[qubit_reflect::Reflect]
struct Constrained<T>(#[reflect(skip)] PhantomData<T>);

#[qubit_reflect::ReflectImpl]
impl<T: Clone + Send> Constrained<T> where T: Sync {}

#[qubit_reflect::Reflect]
struct Borrowed<'scope, T>(#[reflect(skip)] PhantomData<&'scope T>);

#[qubit_reflect::ReflectImpl]
impl<'scope, T: 'scope> Borrowed<'scope, T> {}

fn main() {
    assert_eq!(
        TypeDescriptor::of::<Constrained<u8>>().name(),
        "Constrained"
    );
    assert_eq!(
        TypeDescriptor::of::<Borrowed<'static, u8>>().name(),
        "Borrowed"
    );
}
