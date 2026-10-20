// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Ensures one default method's ThreadSafe bounds do not disable its siblings.

use std::rc::Rc;

use qubit_reflect::Invocation;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

#[Reflect]
struct IndependentDefaultTarget;

#[Reflect]
pub trait IndependentDefaultMethods {
    fn local_only(&self, marker: Rc<()>) -> usize {
        Rc::strong_count(&marker)
    }

    fn thread_safe(&self, input: String) -> String {
        format!("safe: {input}")
    }
}

#[ReflectImpl(reflected_trait = IndependentDefaultMethods)]
impl IndependentDefaultMethods for IndependentDefaultTarget {}

#[test]
fn test_local_only_default_method_does_not_disable_thread_safe_sibling() {
    let ty = ReflectionContext::global().type_of::<IndependentDefaultTarget>();
    let target = IndependentDefaultTarget;

    let local_method = ty
        .method("local_only")
        .expect("the Rc-taking default method is effective");
    let local_output = local_method
        .invoke(Invocation::<Local>::on_ref(&target).arg(Rc::new(())))
        .unwrap_or_else(|failure| {
            panic!(
                "the Rc-taking method retains its Local adapter: {:?}",
                failure.error().kind()
            )
        });
    let local_count = match local_output.into_owned::<usize>() {
        Ok(count) => count,
        Err(_) => panic!("local_only returns usize"),
    };
    assert_eq!(local_count, 1,);

    let thread_safe_method = ty
        .method("thread_safe")
        .expect("the Send + Sync default method is effective");
    let thread_safe_output = thread_safe_method
        .invoke(Invocation::<ThreadSafe>::on_ref(&target).arg(String::from("value")))
        .unwrap_or_else(|failure| {
            panic!(
                "an unrelated Rc parameter does not disable this ThreadSafe adapter: {:?}",
                failure.error().kind()
            )
        });
    let thread_safe_result = match thread_safe_output.into_owned::<String>() {
        Ok(result) => result,
        Err(_) => panic!("thread_safe returns String"),
    };
    assert_eq!(thread_safe_result, "safe: value",);
}
