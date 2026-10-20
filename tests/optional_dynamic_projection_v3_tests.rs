// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use qubit_reflect::Invocation;
use qubit_reflect::InvocationOutput;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;
use qubit_reflect::descriptor::BorrowOrigin;
use qubit_reflect::invoke::InvocationErrorKind;
use qubit_reflect::invoke::InvocationUnavailableReason;
use qubit_reflect::value::Local;

#[Reflect]
struct OptionalTarget {
    value: u32,
}

#[ReflectImpl]
impl OptionalTarget {
    pub fn maybe_value(&self, present: bool) -> Option<&u32> {
        present.then_some(&self.value)
    }

    pub fn maybe_argument(value: &u32, present: bool) -> Option<&u32> {
        present.then_some(value)
    }

    #[expect(
        dead_code,
        reason = "The fixture verifies that optional mutable returns stay descriptive."
    )]
    pub fn maybe_mut_value(&mut self, present: bool) -> Option<&mut u32> {
        present.then_some(&mut self.value)
    }

    pub fn maybe_by_value(self, value: &u32) -> Option<&u32> {
        let _ = self;
        Some(value)
    }

    #[expect(
        dead_code,
        reason = "The fixture checks conservative handling of explicit output lifetimes."
    )]
    pub fn maybe_explicit<'a>(&self, value: &'a u32) -> Option<&'a u32> {
        Some(value)
    }

    #[expect(
        dead_code,
        reason = "The fixture checks conservative handling of static reference outputs."
    )]
    pub fn maybe_static(&self, present: bool) -> Option<&'static u32> {
        static VALUE: u32 = 41;
        present.then_some(&VALUE)
    }
}

#[Reflect]
trait OptionalDefaultMethod {
    fn no_value(&self) -> Option<&u32> {
        None
    }
}

#[ReflectImpl(reflected_trait = OptionalDefaultMethod)]
impl OptionalDefaultMethod for OptionalTarget {}

fn maybe_value() -> qubit_reflect::MethodDescriptor<'static> {
    ReflectionContext::global()
        .type_of::<OptionalTarget>()
        .method("maybe_value")
        .expect("optional reference method is registered")
}

fn maybe_argument() -> qubit_reflect::MethodDescriptor<'static> {
    ReflectionContext::global()
        .type_of::<OptionalTarget>()
        .method("maybe_argument")
        .expect("optional reference method is registered")
}

fn maybe_mut_value() -> qubit_reflect::MethodDescriptor<'static> {
    ReflectionContext::global()
        .type_of::<OptionalTarget>()
        .method("maybe_mut_value")
        .expect("optional mutable reference method is registered")
}

fn maybe_by_value() -> qubit_reflect::MethodDescriptor<'static> {
    ReflectionContext::global()
        .type_of::<OptionalTarget>()
        .method("maybe_by_value")
        .expect("optional reference method is registered")
}

fn maybe_explicit() -> qubit_reflect::MethodDescriptor<'static> {
    ReflectionContext::global()
        .type_of::<OptionalTarget>()
        .method("maybe_explicit")
        .expect("explicit-lifetime optional reference method is registered")
}

fn maybe_static() -> qubit_reflect::MethodDescriptor<'static> {
    ReflectionContext::global()
        .type_of::<OptionalTarget>()
        .method("maybe_static")
        .expect("static optional reference method is registered")
}

fn no_value() -> qubit_reflect::MethodDescriptor<'static> {
    ReflectionContext::global()
        .type_of::<OptionalTarget>()
        .method("no_value")
        .expect("optional reference default method is registered")
}

#[test]
fn optional_reference_output_preserves_some_and_none_semantics() {
    let target = OptionalTarget { value: 17 };
    assert_eq!(target.maybe_value(true), Some(&17));
    assert_eq!(target.maybe_value(false), None);

    let some = maybe_value()
        .invoke(Invocation::<Local>::on_ref(&target).arg(true))
        .unwrap_or_else(|failure| {
            panic!(
                "optional borrowed output is dynamically callable: {:?}",
                failure.error().kind()
            )
        });
    let InvocationOutput::OptionalRef {
        value: Some(value),
        origins,
    } = some
    else {
        panic!("present optional borrow is retained as an optional reference");
    };
    assert_eq!(origins.as_ref(), &[BorrowOrigin::Receiver]);
    let some = InvocationOutput::OptionalRef {
        value: Some(value),
        origins,
    }
    .into_optional_ref::<u32>()
    .unwrap_or_else(|_| panic!("the declared output is an optional u32 reference"));
    assert_eq!(some, Some(&17),);

    let none = maybe_value()
        .invoke(Invocation::<Local>::on_ref(&target).arg(false))
        .unwrap_or_else(|failure| {
            panic!(
                "optional borrowed output is dynamically callable: {:?}",
                failure.error().kind()
            )
        })
        .into_optional_ref::<u32>()
        .unwrap_or_else(|_| panic!("the declared output is an optional u32 reference"));
    assert_eq!(none, None,);

    let argument = 31_u32;
    let output = maybe_argument()
        .invoke(Invocation::<Local>::associated().arg_ref(&argument).arg(true))
        .unwrap_or_else(|failure| {
            panic!(
                "optional argument borrow is dynamically callable: {:?}",
                failure.error().kind()
            )
        });
    let InvocationOutput::OptionalRef {
        value: Some(value),
        origins,
    } = output
    else {
        panic!("present optional argument borrow is retained as an optional reference");
    };
    assert_eq!(origins.as_ref(), &[BorrowOrigin::Parameter(0)]);
    assert_eq!(value.downcast_ref::<u32>(), Some(&31));

    let argument = 43_u32;
    let output = maybe_by_value()
        .invoke(Invocation::<Local>::on_owned(OptionalTarget { value: 0 }).arg_ref(&argument))
        .unwrap_or_else(|failure| panic!("a unique input borrow is returned: {:?}", failure.error().kind()));
    let InvocationOutput::OptionalRef { value, origins } = output else {
        panic!("the by-value receiver does not become the borrow origin");
    };
    assert_eq!(origins.as_ref(), &[BorrowOrigin::Parameter(0)]);
    let value = value.expect("the argument result is present");
    assert_eq!(value.downcast_ref::<u32>(), Some(&43));
}

#[test]
fn optional_reference_output_extraction_preserves_value_on_type_mismatch() {
    let target = OptionalTarget { value: 23 };
    let output = maybe_value()
        .invoke(Invocation::<Local>::on_ref(&target).arg(true))
        .unwrap_or_else(|failure| {
            panic!(
                "optional borrowed output is dynamically callable: {:?}",
                failure.error().kind()
            )
        });
    let failure = output
        .into_optional_ref::<str>()
        .expect_err("the contained reference has the wrong type");
    let InvocationOutput::OptionalRef { value, origins } = failure.into_output() else {
        panic!("failed typed extraction retains the optional reference output");
    };
    assert_eq!(origins.as_ref(), &[BorrowOrigin::Receiver]);
    let Some(value) = value else {
        panic!("failed typed extraction retains the present reference");
    };
    assert_eq!(value.downcast_ref::<u32>(), Some(&23));
}

#[test]
fn optional_reference_default_method_preserves_none_and_receiver_origin() {
    let target = OptionalTarget { value: 37 };
    let output = no_value()
        .invoke(Invocation::<Local>::on_ref(&target))
        .unwrap_or_else(|failure| {
            panic!(
                "optional borrowed default output is callable: {:?}",
                failure.error().kind()
            )
        });
    let InvocationOutput::OptionalRef { value, origins } = output else {
        panic!("default method output uses the optional reference representation");
    };
    assert!(value.is_none());
    assert_eq!(origins.as_ref(), &[BorrowOrigin::Receiver]);
}

#[test]
fn optional_mutable_reference_output_is_described_but_not_dynamically_callable() {
    let mut target = OptionalTarget { value: 29 };
    let failure = match maybe_mut_value().invoke(Invocation::<Local>::on_mut(&mut target).arg(true)) {
        Ok(_) => panic!("the invocation output API has no optional mutable reference variant"),
        Err(failure) => failure,
    };
    assert_eq!(
        failure.error().kind(),
        &InvocationErrorKind::MethodUnavailable {
            reasons: &[InvocationUnavailableReason::UnsupportedBorrowedReturn],
        },
        "an unsupported mutable borrow is retained as descriptive metadata",
    );
}

#[test]
fn explicit_optional_borrow_lifetimes_are_not_inferred_as_receiver_borrows() {
    let target = OptionalTarget { value: 47 };
    let value = 53_u32;
    let failure = match maybe_explicit().invoke(Invocation::<Local>::on_ref(&target).arg_ref(&value)) {
        Ok(_) => panic!("an explicit method lifetime requires an exact specialization"),
        Err(failure) => failure,
    };
    assert_eq!(failure.error().kind(), &InvocationErrorKind::SpecializationRequired);
}

#[test]
fn static_optional_reference_output_is_not_misclassified_as_owned() {
    let target = OptionalTarget { value: 59 };
    let failure = match maybe_static().invoke(Invocation::<Local>::on_ref(&target).arg(true)) {
        Ok(_) => panic!("a static reference remains borrowed, not owned Option data"),
        Err(failure) => failure,
    };
    assert_eq!(
        failure.error().kind(),
        &InvocationErrorKind::MethodUnavailable {
            reasons: &[InvocationUnavailableReason::UnsupportedBorrowedReturn],
        },
        "a static reference is unavailable in the local adapter",
    );
}
