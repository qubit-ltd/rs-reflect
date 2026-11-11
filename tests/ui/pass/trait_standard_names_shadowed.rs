// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![allow(dead_code)]

struct Option;
struct Some;
struct None;
struct Result;
struct Ok;
struct Err;
struct Vec;

#[qubit_reflect::Reflect]
trait MetadataContract {
    fn read(&self) -> u32;
    fn write(&mut self, value: u32);
    fn consume(self)
    where
        Self: Sized;
    fn pinned(self: ::core::pin::Pin<&mut Self>);
    fn associated() -> u32
    where
        Self: Sized;
}

fn main() {}
