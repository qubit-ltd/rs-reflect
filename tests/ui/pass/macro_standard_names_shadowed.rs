// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![allow(dead_code)]

use qubit_reflect::Reflect;

struct Option;
struct Some;
struct None;
struct Result;
struct Vec;

macro_rules! unreachable {
    ($($tokens:tt)*) => {
        panic!("caller unreachable macro was selected")
    };
}

fn exercise_shadowed_macro() {
    unreachable!("the caller-defined macro remains callable");
}

#[Reflect]
pub struct Named {
    pub first: u32,
    pub second: u32,
}

#[Reflect]
pub struct Tuple(pub u32, pub u32);

#[Reflect]
pub struct Unit;

#[Reflect]
pub enum Choice {
    Named { first: u32, second: u32 },
    Tuple(u32, u32),
    Unit,
}

fn main() {}
