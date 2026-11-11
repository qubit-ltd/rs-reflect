// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![allow(dead_code)]

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::bind_method;
use qubit_reflect::register_method_binding;

struct Option;
struct Some;
struct None;
struct Result;
struct Ok;
struct Err;
struct Vec;

#[Reflect]
pub struct Owner;

impl Owner {
    pub fn read(&self) -> u32 {
        7
    }
    pub fn associated() -> u32 {
        9
    }
}

register_method_binding!(Owner::read as fn(&Owner) -> u32);

#[Reflect]
pub struct ImplOwner;

#[ReflectImpl]
impl ImplOwner {
    pub fn read(&self) -> u32 {
        11
    }
}

fn main() {
    let _ = bind_method!(Owner::read as fn(&Owner) -> u32);
    let _ = bind_method!(Owner::associated as fn() -> u32);
}
