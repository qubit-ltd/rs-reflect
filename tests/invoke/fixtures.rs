// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::pin::Pin;
use std::rc::Rc;

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;

#[derive(Default)]
#[Reflect]
pub struct Worker {
    pub value: i32,
}

#[derive(Default)]
#[Reflect]
pub struct LocalOnlyWorker {
    #[reflect(skip)]
    _local_state: Rc<()>,
    pub value: i32,
}

#[ReflectImpl]
impl Worker {
    pub fn add(&mut self, amount: i32) {
        self.value += amount;
    }

    pub fn sum(&self, left: i32, right: i32) -> i32 {
        self.value + left + right
    }

    pub fn echo(&self, value: String) -> String {
        value
    }

    pub fn panic(&self) {
        panic!("worker panic payload");
    }

    fn private_method(&mut self, amount: i32) {
        self.value += amount;
    }

    pub unsafe fn unsafe_method(&mut self, amount: i32) {
        self.value += amount;
    }

    pub fn pinned_value(self: Pin<&mut Worker>) -> i32 {
        self.value
    }

    pub async fn async_value(&self) -> i32 {
        self.value
    }

    pub async fn async_sum(&self, left: i32, right: i32) -> i32 {
        self.value + left + right
    }
}

#[ReflectImpl]
impl LocalOnlyWorker {
    pub fn sum(&self, left: i32, right: i32) -> i32 {
        self.value + left + right
    }
}

pub fn method(name: &str) -> qubit_reflect::MethodDescriptor<'static> {
    qubit_reflect::ReflectionContext::global()
        .type_of::<Worker>()
        .method(name)
        .expect("the reflected worker method must be registered")
}

pub fn local_only_method(name: &str) -> qubit_reflect::MethodDescriptor<'static> {
    qubit_reflect::ReflectionContext::global()
        .type_of::<LocalOnlyWorker>()
        .method(name)
        .expect("the reflected local-only worker method must be registered")
}

pub fn local_only_worker(value: i32) -> LocalOnlyWorker {
    LocalOnlyWorker {
        value,
        ..LocalOnlyWorker::default()
    }
}
