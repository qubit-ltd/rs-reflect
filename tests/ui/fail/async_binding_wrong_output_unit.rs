// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::bind_method;

#[Reflect]
struct Target;

#[ReflectImpl]
impl Target {
    async fn value<T: qubit_reflect::__private::codegen_v4::descriptor::Reflect>(&self) -> String {
        let _: std::marker::PhantomData<T> = std::marker::PhantomData;
        String::new()
    }
}

fn main() {
    let _ = bind_method!(Target::value::<u32> as async fn(&Target) -> ());
}
