// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
//    You may obtain a copy of the License at
//
//        https://www.apache.org/licenses/LICENSE-2.0
//
//    Unless required by applicable law or agreed to in writing, software
//    distributed under the License is distributed on an "AS IS" BASIS,
//    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//    See the License for the specific language governing permissions and
//    limitations under the License.
// =============================================================================

#![cfg(feature = "derive")]

mod shadowed_names {
    #![allow(dead_code)]

    #[allow(unused_macros)]
    macro_rules! unreachable {
        ($($tokens:tt)*) => {
            compile_error!("caller macro selected")
        };
    }

    struct Option;
    struct Some;
    struct None;
    struct Result;
    struct Ok;
    struct Err;
    struct Vec;

    #[qubit_reflect::Reflect]
    pub struct Target {
        pub count: u32,
    }

    #[qubit_reflect::Reflect]
    pub trait DefaultMethods {
        fn read(&self) -> u32 {
            7
        }
        fn sum(&self, value: u32) -> u32 {
            value + 1
        }
        fn update(&mut self, value: u32) -> u32 {
            self.assign(value)
        }
        fn assign(&mut self, value: u32) -> u32;
    }

    #[qubit_reflect::ReflectImpl(reflected_trait = DefaultMethods)]
    impl DefaultMethods for Target {
        fn assign(&mut self, value: u32) -> u32 {
            self.count = value;
            self.count
        }
    }
}

fn assert_default_methods_for_local() {
    use qubit_reflect::value::Local;

    let mut target = shadowed_names::Target { count: 0 };
    let type_descriptor =
        qubit_reflect::ReflectionContext::global().type_of::<shadowed_names::Target>();

    let read = type_descriptor.method("read").expect("default read method");
    let output = read
        .invoke(qubit_reflect::Invocation::<Local>::on_ref(&target))
        .unwrap_or_else(|_| panic!("default read invocation"));
    assert_eq!(
        output
            .into_owned::<u32>()
            .unwrap_or_else(|_| panic!("u32 result")),
        7
    );

    let sum = type_descriptor.method("sum").expect("default sum method");
    let output = sum
        .invoke(qubit_reflect::Invocation::<Local>::on_ref(&target).arg(4_u32))
        .unwrap_or_else(|_| panic!("default sum invocation"));
    assert_eq!(
        output
            .into_owned::<u32>()
            .unwrap_or_else(|_| panic!("u32 result")),
        5
    );

    let update = type_descriptor
        .method("update")
        .expect("default update method");
    let output = update
        .invoke(qubit_reflect::Invocation::<Local>::on_mut(&mut target).arg(9_u32))
        .unwrap_or_else(|_| panic!("default update invocation"));
    assert_eq!(
        output
            .into_owned::<u32>()
            .unwrap_or_else(|_| panic!("u32 result")),
        9
    );
    assert_eq!(target.count, 9);
}

fn assert_default_methods_for_thread_safe() {
    use qubit_reflect::value::ThreadSafe;

    let mut target = shadowed_names::Target { count: 0 };
    let type_descriptor =
        qubit_reflect::ReflectionContext::global().type_of::<shadowed_names::Target>();

    let read = type_descriptor.method("read").expect("default read method");
    let output = read
        .invoke(qubit_reflect::Invocation::<ThreadSafe>::on_ref(&target))
        .unwrap_or_else(|_| panic!("default read invocation"));
    assert_eq!(
        output
            .into_owned::<u32>()
            .unwrap_or_else(|_| panic!("u32 result")),
        7
    );

    let sum = type_descriptor.method("sum").expect("default sum method");
    let output = sum
        .invoke(qubit_reflect::Invocation::<ThreadSafe>::on_ref(&target).arg(4_u32))
        .unwrap_or_else(|_| panic!("default sum invocation"));
    assert_eq!(
        output
            .into_owned::<u32>()
            .unwrap_or_else(|_| panic!("u32 result")),
        5
    );

    let update = type_descriptor
        .method("update")
        .expect("default update method");
    let output = update
        .invoke(qubit_reflect::Invocation::<ThreadSafe>::on_mut(&mut target).arg(9_u32))
        .unwrap_or_else(|_| panic!("default update invocation"));
    assert_eq!(
        output
            .into_owned::<u32>()
            .unwrap_or_else(|_| panic!("u32 result")),
        9
    );
    assert_eq!(target.count, 9);
}

#[test]
fn test_default_methods_are_hygienic_for_local_invocations() {
    assert_default_methods_for_local();
}

#[test]
fn test_default_methods_are_hygienic_for_thread_safe_invocations() {
    assert_default_methods_for_thread_safe();
}
