// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow explicit-imports
//! Integration tests for the reflected invocation runtime.
mod invocation_runtime {
    use std::any::TypeId;
    use std::cell::Cell;
    use std::future::Future;
    use std::panic::AssertUnwindSafe;
    use std::panic::catch_unwind;
    use std::pin::Pin;
    use std::rc::Rc;
    use std::sync::Mutex;
    use std::sync::MutexGuard;
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;
    use std::task::Context;
    use std::task::Poll;
    use std::task::Waker;

    use qubit_reflect as reflect;
    use qubit_reflect::identity::FragmentIdentity;
    use qubit_reflect::identity::MemberId;
    use qubit_reflect::invoke::ArgumentExpectation;
    use qubit_reflect::invoke::BorrowOrigin;
    use qubit_reflect::invoke::Invocation;
    use qubit_reflect::invoke::InvocationArg;
    use qubit_reflect::invoke::InvocationBinding;
    use qubit_reflect::invoke::InvocationErrorKind;
    use qubit_reflect::invoke::InvocationFailure;
    use qubit_reflect::invoke::InvocationInputMode;
    use qubit_reflect::invoke::InvocationMode;
    use qubit_reflect::invoke::InvocationOutput;
    use qubit_reflect::invoke::InvocationPanic;
    use qubit_reflect::invoke::InvocationReceiver;
    use qubit_reflect::invoke::ReceiverExpectation;
    use qubit_reflect::invoke::ReflectedFuture;
    use qubit_reflect::value::DynamicMut;
    use qubit_reflect::value::DynamicOwned;
    use qubit_reflect::value::DynamicRef;
    use qubit_reflect::value::Local;
    use qubit_reflect::value::ThreadSafe;

    /// A small receiver used by the hand-written invocation adapters.
    #[derive(Debug, Eq, PartialEq)]
    struct Counter {
        value: i32,
    }

    /// A reflected target used to exercise descriptor-aware argument binding.
    #[derive(reflect::Reflect)]
    #[reflect(opaque)]
    struct NamedInvocationTarget;

    /// Counts entries into the reflected user function.
    static NAMED_INVOCATION_CALLS: AtomicUsize = AtomicUsize::new(0);

    /// Serializes tests that observe the shared invocation-entry counter.
    static NAMED_INVOCATION_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[reflect::reflect_impl]
    impl NamedInvocationTarget {
        /// Encodes three differently sized inputs so binding order is visible.
        fn encode(first: u8, second: u16, third: u32) -> u64 {
            NAMED_INVOCATION_CALLS.fetch_add(1, Ordering::SeqCst);
            u64::from(first) * 1_000_000 + u64::from(second) * 1_000 + u64::from(third)
        }

        /// Uses wildcard and destructuring parameters that remain positional.
        fn encode_patterns(_: u8, (left, right): (u16, u16), named: u32) -> u64 {
            NAMED_INVOCATION_CALLS.fetch_add(1, Ordering::SeqCst);
            u64::from(left) * 1_000_000 + u64::from(right) * 1_000 + u64::from(named)
        }

        /// Encodes named inputs through an explicitly generated panic catcher.
        #[reflect(catch_unwind)]
        fn encode_catching(first: u8, second: u16, third: u32) -> u64 {
            NAMED_INVOCATION_CALLS.fetch_add(1, Ordering::SeqCst);
            u64::from(first) * 1_000_000 + u64::from(second) * 1_000 + u64::from(third)
        }

        /// Encodes named inputs while retaining a pinned shared receiver.
        fn encode_pinned(self: Pin<&Self>, first: u8, second: u16, third: u32) -> u64 {
            NAMED_INVOCATION_CALLS.fetch_add(1, Ordering::SeqCst);
            u64::from(first) * 1_000_000 + u64::from(second) * 1_000 + u64::from(third)
        }

        /// Encodes named inputs while retaining a pinned mutable receiver.
        fn encode_pinned_mut(self: Pin<&mut Self>, first: u8, second: u16, third: u32) -> u64 {
            NAMED_INVOCATION_CALLS.fetch_add(1, Ordering::SeqCst);
            u64::from(first) * 1_000_000 + u64::from(second) * 1_000 + u64::from(third)
        }

        /// Encodes same-typed inputs so raw adapter order mistakes are visible.
        fn encode_same(first: u8, second: u8) -> u16 {
            NAMED_INVOCATION_CALLS.fetch_add(1, Ordering::SeqCst);
            u16::from(first) * 100 + u16::from(second)
        }

        /// Uses an identifier-at-subpattern that remains positional-only.
        fn encode_at(whole @ (left, right): (u16, u16), named: u32) -> u64 {
            let _ = whole;
            NAMED_INVOCATION_CALLS.fetch_add(1, Ordering::SeqCst);
            u64::from(left) * 1_000_000 + u64::from(right) * 1_000 + u64::from(named)
        }
    }

    /// Creates a structured identity for one hand-written test adapter.
    fn method_identity(index: usize) -> MemberId {
        MemberId::new(
            "runtime_tests::Counter",
            "method",
            index,
            FragmentIdentity::new(
                "qubit-reflect",
                "invoke::runtime_tests",
                1,
                index as u32,
                "method",
                index as u64,
            ),
        )
    }

    /// Returns the concrete reflected instance for the named-binding fixture.
    fn named_invocation_method() -> &'static reflect::descriptor::MethodInstanceDescriptor {
        named_invocation_method_by_name("encode")
    }

    /// Returns one concrete reflected instance from the named-binding fixture.
    fn named_invocation_method_by_name(name: &str) -> &'static reflect::descriptor::MethodInstanceDescriptor {
        let registry =
            reflect::registry::ReflectRegistry::initialize().expect("the named invocation fixture must register");
        let implementations =
            registry.implementations(<NamedInvocationTarget as reflect::Reflect>::type_descriptor().type_id());
        let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
            implementations,
            reflect::descriptor::MethodQualifier::Inherent,
            name,
        ) else {
            panic!("the named invocation method must be uniquely discoverable")
        };
        instance
    }

    /// Locks the named invocation fixture even after an earlier test panic.
    fn lock_named_invocation_fixture() -> MutexGuard<'static, ()> {
        NAMED_INVOCATION_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Validates and executes a hand-written mutable receiver adapter.
    fn invoke_add<'call>(
        invocation: Invocation<'call, Local>,
    ) -> Result<InvocationOutput<'call, Local>, InvocationFailure<'call, Local>> {
        let validated = invocation.validate(
            &method_identity(0),
            ReceiverExpectation::borrowed_mut::<Counter>(),
            &[ArgumentExpectation::owned::<i32>()],
        )?;
        let (receiver, arguments) = validated.into_parts();
        let Some(InvocationReceiver::Mut(mut receiver)) = receiver else {
            unreachable!("validation guarantees a mutable receiver")
        };
        let [InvocationArg::Owned(value)] = <Vec<InvocationArg<'call, Local>> as TryInto<
            [InvocationArg<'call, Local>; 1],
        >>::try_into(arguments.into_vec())
        .unwrap_or_else(|_| unreachable!("validation guarantees one argument")) else {
            unreachable!("validation guarantees an owned argument")
        };
        let counter = receiver
            .downcast_mut::<Counter>()
            .unwrap_or_else(|| unreachable!("validation guarantees receiver type"));
        let value = DynamicOwned::<Local>::downcast::<i32>(value)
            .unwrap_or_else(|_| unreachable!("validation guarantees argument type"));
        counter.value += value;
        Ok(InvocationOutput::Unit)
    }

    /// Validates an invocation whose shared input accepts a mutable reborrow.
    fn invoke_read<'call>(
        invocation: Invocation<'call, Local>,
    ) -> Result<InvocationOutput<'call, Local>, InvocationFailure<'call, Local>> {
        let validated = invocation.validate(
            &method_identity(1),
            ReceiverExpectation::borrowed::<Counter>(),
            &[ArgumentExpectation::borrowed::<String>()],
        )?;
        let (receiver, arguments) = validated.into_parts();
        let Some(InvocationReceiver::Mut(receiver)) = receiver else {
            unreachable!("shared validation accepts the supplied mutable receiver")
        };
        let [InvocationArg::Mut(argument)] = <Vec<InvocationArg<'call, Local>> as TryInto<
            [InvocationArg<'call, Local>; 1],
        >>::try_into(arguments.into_vec())
        .unwrap_or_else(|_| unreachable!("validation guarantees one argument")) else {
            unreachable!("shared validation accepts the supplied mutable argument")
        };
        let counter = receiver
            .downcast_ref::<Counter>()
            .unwrap_or_else(|| unreachable!("validation guarantees receiver type"));
        let suffix = argument
            .downcast_ref::<String>()
            .unwrap_or_else(|| unreachable!("validation guarantees argument type"));
        Ok(InvocationOutput::Owned(DynamicOwned::<Local>::new(format!(
            "{}{}",
            counter.value, suffix
        ))))
    }

    /// Panics after validation to prove user panic propagation stays distinct.
    fn invoke_panicking<'call>(
        invocation: Invocation<'call, Local>,
    ) -> Result<InvocationOutput<'call, Local>, InvocationFailure<'call, Local>> {
        let _validated = invocation.validate(&method_identity(2), ReceiverExpectation::none(), &[])?;
        panic!("user panic payload")
    }

    /// Returns a shared sub-borrow after consuming the validated receiver
    /// wrapper.
    fn invoke_label<'call>(
        invocation: Invocation<'call, Local>,
    ) -> Result<InvocationOutput<'call, Local>, InvocationFailure<'call, Local>> {
        let validated = invocation.validate(&method_identity(3), ReceiverExpectation::borrowed::<Counter>(), &[])?;
        let (receiver, arguments) = validated.into_parts();
        assert!(arguments.is_empty());
        let Some(InvocationReceiver::Ref(receiver)) = receiver else {
            unreachable!("validation guarantees a shared receiver")
        };
        let counter = DynamicRef::<Local>::downcast::<Counter>(receiver)
            .unwrap_or_else(|_| unreachable!("validation guarantees receiver type"));
        Ok(InvocationOutput::Ref {
            value: DynamicRef::<Local>::new(&counter.value),
            origins: [BorrowOrigin::Receiver].into(),
        })
    }

    /// Returns an exclusive sub-borrow after consuming the validated receiver
    /// wrapper.
    fn invoke_value_mut<'call>(
        invocation: Invocation<'call, Local>,
    ) -> Result<InvocationOutput<'call, Local>, InvocationFailure<'call, Local>> {
        let validated =
            invocation.validate(&method_identity(4), ReceiverExpectation::borrowed_mut::<Counter>(), &[])?;
        let (receiver, arguments) = validated.into_parts();
        assert!(arguments.is_empty());
        let Some(InvocationReceiver::Mut(receiver)) = receiver else {
            unreachable!("validation guarantees a mutable receiver")
        };
        let counter = DynamicMut::<Local>::downcast::<Counter>(receiver)
            .unwrap_or_else(|_| unreachable!("validation guarantees receiver type"));
        Ok(InvocationOutput::Mut {
            value: DynamicMut::<Local>::new(&mut counter.value),
            origin: BorrowOrigin::Receiver,
        })
    }

    /// Returns a dedicated `str` borrow after consuming a validated argument
    /// wrapper.
    fn invoke_str_identity<'call>(
        invocation: Invocation<'call, Local>,
    ) -> Result<InvocationOutput<'call, Local>, InvocationFailure<'call, Local>> {
        let validated = invocation.validate(
            &method_identity(5),
            ReceiverExpectation::none(),
            &[ArgumentExpectation::borrowed::<str>()],
        )?;
        let (receiver, arguments) = validated.into_parts();
        assert!(receiver.is_none());
        let [InvocationArg::Ref(argument)] = <Vec<InvocationArg<'call, Local>> as TryInto<
            [InvocationArg<'call, Local>; 1],
        >>::try_into(arguments.into_vec())
        .unwrap_or_else(|_| unreachable!("validation guarantees one argument")) else {
            unreachable!("validation guarantees a shared argument")
        };
        let text = DynamicRef::<Local>::into_str(argument)
            .unwrap_or_else(|_| unreachable!("validation guarantees str storage"));
        Ok(InvocationOutput::Ref {
            value: DynamicRef::<Local>::new_str(text),
            origins: [BorrowOrigin::Parameter(0)].into(),
        })
    }

    /// Creates a lazy future that returns a receiver sub-borrow on first poll.
    fn invoke_label_async<'call>(
        invocation: Invocation<'call, Local>,
        polls: Rc<Cell<usize>>,
    ) -> Result<InvocationOutput<'call, Local>, InvocationFailure<'call, Local>> {
        let validated = invocation.validate(&method_identity(6), ReceiverExpectation::borrowed::<Counter>(), &[])?;
        let (receiver, arguments) = validated.into_parts();
        assert!(arguments.is_empty());
        let Some(InvocationReceiver::Ref(receiver)) = receiver else {
            unreachable!("validation guarantees a shared receiver")
        };
        let counter = DynamicRef::<Local>::downcast::<Counter>(receiver)
            .unwrap_or_else(|_| unreachable!("validation guarantees receiver type"));
        let future = async move {
            polls.set(polls.get() + 1);
            InvocationOutput::Ref {
                value: DynamicRef::<Local>::new(&counter.value),
                origins: [BorrowOrigin::Receiver].into(),
            }
        };
        Ok(InvocationOutput::Future(ReflectedFuture::<Local>::new(future)))
    }

    /// Polls one future exactly once without selecting an executor.
    fn poll_once<F: Future + Unpin>(future: &mut F) -> Poll<F::Output> {
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        Pin::new(future).poll(&mut context)
    }

    /// Verifies a mutable receiver and owned parameter execute after
    /// validation.
    #[test]
    fn test_validate_and_execute_mutable_receiver_with_owned_argument() {
        let mut counter = Counter { value: 5 };
        let invocation = Invocation::borrowed_mut(
            DynamicMut::<Local>::new(&mut counter),
            [InvocationArg::Owned(DynamicOwned::<Local>::new(7_i32))],
        );

        let output = invoke_add(invocation).expect("a valid invocation should execute");

        assert!(matches!(output, InvocationOutput::Unit));
        drop(output);
        assert_eq!(counter.value, 12);
    }

    /// Verifies mutable inputs can satisfy shared receiver and argument
    /// expectations.
    #[test]
    fn test_shared_expectation_accepts_mutable_reborrow() {
        let mut counter = Counter { value: 4 };
        let mut suffix = String::from(" items");
        let invocation = Invocation::borrowed_mut(
            DynamicMut::<Local>::new(&mut counter),
            [InvocationArg::Mut(DynamicMut::<Local>::new(&mut suffix))],
        );

        let output = invoke_read(invocation).expect("mutable inputs may be read through shared borrows");

        let InvocationOutput::Owned(value) = output else {
            panic!("the adapter should return an owned value")
        };
        assert_eq!(
            DynamicOwned::<Local>::downcast::<String>(value).unwrap_or_else(|_| panic!("output should be String")),
            "4 items"
        );
    }

    /// Verifies consuming validated receiver storage can return a shared
    /// sub-borrow.
    #[test]
    fn test_validated_receiver_can_produce_a_call_lifetime_borrowed_output() {
        let counter = Counter { value: 41 };
        let invocation = Invocation::borrowed(DynamicRef::<Local>::new(&counter), []);

        let output = invoke_label(invocation).expect("borrowed output should retain call lifetime");

        let InvocationOutput::Ref { value, origins } = output else {
            panic!("adapter should return a shared output")
        };
        assert_eq!(value.downcast_ref::<i32>(), Some(&41));
        assert_eq!(origins.as_ref(), &[BorrowOrigin::Receiver]);
    }

    /// Verifies consuming validated mutable storage can return an exclusive
    /// sub-borrow.
    #[test]
    fn test_validated_receiver_can_produce_a_mutable_borrowed_output() {
        let mut counter = Counter { value: 13 };
        let invocation = Invocation::borrowed_mut(DynamicMut::<Local>::new(&mut counter), []);

        let output = invoke_value_mut(invocation).expect("mutable output should retain call lifetime");

        let InvocationOutput::Mut { mut value, origin } = output else {
            panic!("adapter should return a mutable output")
        };
        assert_eq!(origin, BorrowOrigin::Receiver);
        *value
            .downcast_mut::<i32>()
            .expect("output should retain the exact field type") = 21;
        assert_eq!(value.downcast_ref::<i32>(), Some(&21));
    }

    /// Verifies dedicated `str` storage retains its call lifetime through an
    /// adapter.
    #[test]
    fn test_validated_str_argument_can_produce_a_borrowed_str_output() {
        let text = String::from("borrowed text");
        let invocation = Invocation::associated([InvocationArg::Ref(DynamicRef::<Local>::new_str(text.as_str()))]);

        let output = invoke_str_identity(invocation).expect("dedicated str input should preserve the call lifetime");

        let InvocationOutput::Ref { value, origins } = output else {
            panic!("adapter should return a shared str output")
        };
        assert_eq!(value.as_str(), Some("borrowed text"));
        assert_eq!(origins.as_ref(), &[BorrowOrigin::Parameter(0)]);
    }

    /// Verifies a lazy future may retain and later return a validated receiver
    /// borrow.
    #[test]
    fn test_validated_receiver_can_feed_a_lazy_borrowing_future() {
        let counter = Counter { value: 29 };
        let polls = Rc::new(Cell::new(0));
        let invocation = Invocation::borrowed(DynamicRef::<Local>::new(&counter), []);

        let output =
            invoke_label_async(invocation, Rc::clone(&polls)).expect("async adapter should retain receiver borrow");

        assert_eq!(polls.get(), 0);
        let InvocationOutput::Future(mut future) = output else {
            panic!("adapter should return a reflected future")
        };
        let Poll::Ready(InvocationOutput::Ref { value, origins }) = poll_once(&mut future) else {
            panic!("the simple future should complete on first poll")
        };
        assert_eq!(value.downcast_ref::<i32>(), Some(&29));
        assert_eq!(origins.as_ref(), &[BorrowOrigin::Receiver]);
        assert_eq!(polls.get(), 1);
    }

    /// Verifies owned arguments cannot silently satisfy shared-borrow
    /// parameters.
    #[test]
    fn test_owned_input_is_not_implicitly_borrowed() {
        let invocation =
            Invocation::associated([InvocationArg::Owned(DynamicOwned::<Local>::new(String::from("owned")))]);

        let identity = method_identity(7);
        let failure = invocation
            .validate(
                &identity,
                ReceiverExpectation::none(),
                &[ArgumentExpectation::borrowed::<String>()],
            )
            .expect_err("owned arguments must not be implicitly borrowed");

        assert!(matches!(
            failure.error().kind(),
            InvocationErrorKind::ArgumentModeMismatch {
                index: 0,
                expected: InvocationInputMode::Ref,
                actual: InvocationInputMode::Owned,
            }
        ));
        let (_, arguments) = failure.into_recovery().into_parts();
        let [InvocationArg::Owned(value)] =
            <Vec<InvocationArg<'_, Local>> as TryInto<[InvocationArg<'_, Local>; 1]>>::try_into(arguments.into_vec())
                .unwrap_or_else(|_| panic!("recovery should preserve the argument"))
        else {
            panic!("recovery should preserve owned mode")
        };
        assert_eq!(
            DynamicOwned::<Local>::downcast::<String>(value).unwrap_or_else(|_| panic!("owned value should be intact")),
            "owned"
        );
    }

    /// Verifies named and positional inputs may be interleaved
    /// deterministically.
    #[test]
    fn test_named_positional_named_binding_uses_declaration_order_for_positional_input() {
        let _guard = lock_named_invocation_fixture();
        NAMED_INVOCATION_CALLS.store(0, Ordering::SeqCst);
        let invocation = Invocation::associated_bindings([
            InvocationBinding::named("third", InvocationArg::Owned(DynamicOwned::<Local>::new(3_u32))),
            InvocationBinding::positional(InvocationArg::Owned(DynamicOwned::<Local>::new(1_u8))),
            InvocationBinding::named("second", InvocationArg::Owned(DynamicOwned::<Local>::new(2_u16))),
        ]);

        let output = named_invocation_method()
            .invoke_local(invocation)
            .expect("the generated adapter must support local invocation")
            .expect("all three bindings are valid");

        let InvocationOutput::Owned(output) = output else {
            panic!("the associated function must return an owned value")
        };
        assert_eq!(
            DynamicOwned::<Local>::downcast::<u64>(output)
                .unwrap_or_else(|_| panic!("the output must retain its exact type")),
            1_002_003
        );
        assert_eq!(NAMED_INVOCATION_CALLS.load(Ordering::SeqCst), 1);
    }

    /// Verifies positional binding skips a parameter already occupied by name.
    #[test]
    fn test_positional_binding_skips_parameter_already_bound_by_name() {
        let _guard = lock_named_invocation_fixture();
        NAMED_INVOCATION_CALLS.store(0, Ordering::SeqCst);
        let invocation = Invocation::associated_bindings([
            InvocationBinding::named("first", InvocationArg::Owned(DynamicOwned::<Local>::new(4_u8))),
            InvocationBinding::positional(InvocationArg::Owned(DynamicOwned::<Local>::new(5_u16))),
            InvocationBinding::named("third", InvocationArg::Owned(DynamicOwned::<Local>::new(6_u32))),
        ]);

        let output = named_invocation_method()
            .invoke_local(invocation)
            .expect("the generated adapter must support local invocation")
            .expect("the positional binding must skip the named first parameter");

        let InvocationOutput::Owned(output) = output else {
            panic!("the associated function must return an owned value")
        };
        assert_eq!(
            DynamicOwned::<Local>::downcast::<u64>(output)
                .unwrap_or_else(|_| panic!("the output must retain its exact type")),
            4_005_006
        );
        assert_eq!(NAMED_INVOCATION_CALLS.load(Ordering::SeqCst), 1);
    }

    /// Verifies duplicate bindings fail before extraction and preserve inputs.
    #[test]
    fn test_duplicate_named_binding_recovers_original_arguments_without_execution() {
        let _guard = lock_named_invocation_fixture();
        NAMED_INVOCATION_CALLS.store(0, Ordering::SeqCst);
        let invocation = Invocation::associated_bindings([
            InvocationBinding::named("first", InvocationArg::Owned(DynamicOwned::<Local>::new(7_u8))),
            InvocationBinding::named("first", InvocationArg::Owned(DynamicOwned::<Local>::new(8_u8))),
            InvocationBinding::named("third", InvocationArg::Owned(DynamicOwned::<Local>::new(9_u32))),
        ]);

        let result = named_invocation_method()
            .invoke_local(invocation)
            .expect("the generated adapter must support local invocation");
        let Err(failure) = result else {
            panic!("binding the first parameter twice must fail")
        };

        assert!(matches!(
            failure.error().kind(),
            InvocationErrorKind::DuplicateArgumentBinding {
                input_index: 1,
                parameter_index: 0,
            }
        ));
        let failure_source =
            std::error::Error::source(&failure).expect("an invocation failure must expose its structured error");
        let invocation_error = failure_source
            .downcast_ref::<reflect::invoke::InvocationError>()
            .expect("the first source must be InvocationError");
        assert!(
            std::error::Error::source(invocation_error)
                .and_then(|source| source.downcast_ref::<InvocationErrorKind>())
                .is_some(),
            "InvocationError must expose InvocationErrorKind as its source"
        );
        assert_eq!(NAMED_INVOCATION_CALLS.load(Ordering::SeqCst), 0);
        assert_eq!(failure.recovery().argument_name(0), Some("first"));
        assert_eq!(failure.recovery().argument_name(1), Some("first"));
        assert_eq!(failure.recovery().argument_name(2), Some("third"));
        let (_, arguments) = failure.into_recovery().into_parts();
        let [first, second, third] =
            <Vec<InvocationArg<'_, Local>> as TryInto<[InvocationArg<'_, Local>; 3]>>::try_into(arguments.into_vec())
                .unwrap_or_else(|_| panic!("all original arguments must be recovered"));
        let InvocationArg::Owned(first) = first else {
            panic!("the first argument must remain owned")
        };
        let InvocationArg::Owned(second) = second else {
            panic!("the second argument must remain owned")
        };
        let InvocationArg::Owned(third) = third else {
            panic!("the third argument must remain owned")
        };
        assert_eq!(
            DynamicOwned::<Local>::downcast::<u8>(first).unwrap_or_else(|_| panic!("the first value must be intact")),
            7
        );
        assert_eq!(
            DynamicOwned::<Local>::downcast::<u8>(second).unwrap_or_else(|_| panic!("the second value must be intact")),
            8
        );
        assert_eq!(
            DynamicOwned::<Local>::downcast::<u32>(third).unwrap_or_else(|_| panic!("the third value must be intact")),
            9
        );
    }

    /// Verifies adapter validation failure restores caller order after binding.
    #[test]
    fn test_named_binding_type_failure_recovers_caller_order_before_execution() {
        let _guard = lock_named_invocation_fixture();
        NAMED_INVOCATION_CALLS.store(0, Ordering::SeqCst);
        let invocation = Invocation::associated_bindings([
            InvocationBinding::named("third", InvocationArg::Owned(DynamicOwned::<Local>::new(12_u32))),
            InvocationBinding::positional(InvocationArg::Owned(DynamicOwned::<Local>::new(10_u8))),
            InvocationBinding::named(
                "second",
                InvocationArg::Owned(DynamicOwned::<Local>::new(String::from("wrong"))),
            ),
        ]);

        let result = named_invocation_method()
            .invoke_local(invocation)
            .expect("the generated adapter must support local invocation");
        let Err(failure) = result else {
            panic!("the second parameter must reject the wrong exact type")
        };

        assert!(matches!(
            failure.error().kind(),
            InvocationErrorKind::ArgumentTypeMismatch { index: 1, .. }
        ));
        assert_eq!(NAMED_INVOCATION_CALLS.load(Ordering::SeqCst), 0);
        assert_eq!(failure.recovery().argument_name(0), Some("third"));
        assert_eq!(failure.recovery().argument_name(1), None);
        assert_eq!(failure.recovery().argument_name(2), Some("second"));
        let (_, arguments) = failure.into_recovery().into_parts();
        let [third, first, second] =
            <Vec<InvocationArg<'_, Local>> as TryInto<[InvocationArg<'_, Local>; 3]>>::try_into(arguments.into_vec())
                .unwrap_or_else(|_| panic!("all original arguments must be recovered"));
        let InvocationArg::Owned(third) = third else {
            panic!("the first caller argument must remain owned")
        };
        let InvocationArg::Owned(first) = first else {
            panic!("the second caller argument must remain owned")
        };
        let InvocationArg::Owned(second) = second else {
            panic!("the third caller argument must remain owned")
        };
        assert_eq!(
            DynamicOwned::<Local>::downcast::<u32>(third)
                .unwrap_or_else(|_| panic!("the third-parameter value must be intact")),
            12
        );
        assert_eq!(
            DynamicOwned::<Local>::downcast::<u8>(first)
                .unwrap_or_else(|_| panic!("the positional value must be intact")),
            10
        );
        assert_eq!(
            DynamicOwned::<Local>::downcast::<String>(second)
                .unwrap_or_else(|_| panic!("the mismatched value must be intact")),
            "wrong"
        );
    }

    /// Verifies wildcard and destructuring parameters remain position-bindable.
    #[test]
    fn test_wildcard_and_destructure_parameters_bind_positionally_with_named_identifier() {
        let _guard = lock_named_invocation_fixture();
        NAMED_INVOCATION_CALLS.store(0, Ordering::SeqCst);
        let method = named_invocation_method_by_name("encode_patterns");
        assert_eq!(method.effective_method().parameters()[0].name(), None);
        assert_eq!(method.effective_method().parameters()[1].name(), None);
        assert_eq!(method.effective_method().parameters()[2].name(), Some("named"));
        let invocation = Invocation::associated_bindings([
            InvocationBinding::positional(InvocationArg::Owned(DynamicOwned::<Local>::new(0_u8))),
            InvocationBinding::positional(InvocationArg::Owned(DynamicOwned::<Local>::new((13_u16, 14_u16)))),
            InvocationBinding::named("named", InvocationArg::Owned(DynamicOwned::<Local>::new(15_u32))),
        ]);

        let output = method
            .invoke_local(invocation)
            .expect("the generated adapter must support local invocation")
            .expect("wildcard and destructuring parameters must support positional binding");

        let InvocationOutput::Owned(output) = output else {
            panic!("the associated function must return an owned value")
        };
        assert_eq!(
            DynamicOwned::<Local>::downcast::<u64>(output)
                .unwrap_or_else(|_| panic!("the output must retain its exact type")),
            13_014_015
        );
        assert_eq!(NAMED_INVOCATION_CALLS.load(Ordering::SeqCst), 1);
    }

    /// Verifies a non-identifier pattern cannot be selected by a guessed name.
    #[test]
    fn test_wildcard_parameter_rejects_named_binding_before_execution() {
        let _guard = lock_named_invocation_fixture();
        NAMED_INVOCATION_CALLS.store(0, Ordering::SeqCst);
        let invocation = Invocation::associated_bindings([
            InvocationBinding::named("_", InvocationArg::Owned(DynamicOwned::<Local>::new(16_u8))),
            InvocationBinding::positional(InvocationArg::Owned(DynamicOwned::<Local>::new((17_u16, 18_u16)))),
            InvocationBinding::named("named", InvocationArg::Owned(DynamicOwned::<Local>::new(19_u32))),
        ]);

        let result = named_invocation_method_by_name("encode_patterns")
            .invoke_local(invocation)
            .expect("the generated adapter must support local invocation");
        let Err(failure) = result else {
            panic!("a wildcard parameter has no bindable identifier")
        };

        assert!(matches!(
            failure.error().kind(),
            InvocationErrorKind::UnknownArgumentName { input_index: 0, name }
                if name.as_ref() == "_"
        ));
        assert_eq!(NAMED_INVOCATION_CALLS.load(Ordering::SeqCst), 0);
        assert_eq!(failure.recovery().argument_name(0), Some("_"));
        assert_eq!(failure.recovery().arguments().len(), 3);
    }

    /// Verifies omitted parameters fail during binding instead of in user code.
    #[test]
    fn test_named_binding_reports_missing_parameter_before_execution() {
        let _guard = lock_named_invocation_fixture();
        NAMED_INVOCATION_CALLS.store(0, Ordering::SeqCst);
        let invocation = Invocation::associated_bindings([
            InvocationBinding::named("first", InvocationArg::Owned(DynamicOwned::<Local>::new(20_u8))),
            InvocationBinding::named("third", InvocationArg::Owned(DynamicOwned::<Local>::new(22_u32))),
        ]);

        let result = named_invocation_method()
            .invoke_local(invocation)
            .expect("the generated adapter must support local invocation");
        let Err(failure) = result else {
            panic!("the omitted second parameter must fail binding")
        };

        assert!(matches!(
            failure.error().kind(),
            InvocationErrorKind::MissingArgumentBinding {
                parameter_index: 1,
                name: Some("second"),
            }
        ));
        assert_eq!(NAMED_INVOCATION_CALLS.load(Ordering::SeqCst), 0);
        assert_eq!(failure.recovery().arguments().len(), 2);
    }

    /// Verifies catching invocation uses descriptor-aware named binding.
    #[test]
    fn test_catching_named_binding_reorders_before_user_code() {
        let _guard = lock_named_invocation_fixture();
        NAMED_INVOCATION_CALLS.store(0, Ordering::SeqCst);
        let invocation = Invocation::associated_bindings([
            InvocationBinding::named("third", InvocationArg::Owned(DynamicOwned::<Local>::new(25_u32))),
            InvocationBinding::named("first", InvocationArg::Owned(DynamicOwned::<Local>::new(23_u8))),
            InvocationBinding::named("second", InvocationArg::Owned(DynamicOwned::<Local>::new(24_u16))),
        ]);

        let output = named_invocation_method_by_name("encode_catching")
            .invoke_catching_local(invocation)
            .expect("the catching adapter must be generated")
            .expect("binding must pass before entering the catching adapter")
            .expect("the user function must not panic");

        let InvocationOutput::Owned(output) = output else {
            panic!("the catching associated function must return an owned value")
        };
        assert_eq!(
            DynamicOwned::<Local>::downcast::<u64>(output)
                .unwrap_or_else(|_| panic!("the output must retain its exact type")),
            23_024_025
        );
        assert_eq!(NAMED_INVOCATION_CALLS.load(Ordering::SeqCst), 1);
    }

    /// Verifies a pinned shared receiver supports descriptor-aware bindings.
    #[test]
    fn test_pinned_ref_named_binding_reorders_before_user_code() {
        let _guard = lock_named_invocation_fixture();
        NAMED_INVOCATION_CALLS.store(0, Ordering::SeqCst);
        let target = Box::pin(NamedInvocationTarget);
        let invocation = reflect::invoke::PinnedRefInvocation::from_bindings(
            target.as_ref(),
            [
                InvocationBinding::named("third", InvocationArg::Owned(DynamicOwned::<Local>::new(28_u32))),
                InvocationBinding::named("first", InvocationArg::Owned(DynamicOwned::<Local>::new(26_u8))),
                InvocationBinding::named("second", InvocationArg::Owned(DynamicOwned::<Local>::new(27_u16))),
            ],
        );

        let output = named_invocation_method_by_name("encode_pinned")
            .invoke_pinned_ref_local(invocation)
            .expect("the pinned shared adapter must be generated")
            .expect("all pinned shared bindings must validate");

        let InvocationOutput::Owned(output) = output else {
            panic!("the pinned shared method must return an owned value")
        };
        assert_eq!(
            DynamicOwned::<Local>::downcast::<u64>(output)
                .unwrap_or_else(|_| panic!("the output must retain its exact type")),
            26_027_028
        );
        assert_eq!(NAMED_INVOCATION_CALLS.load(Ordering::SeqCst), 1);
    }

    /// Verifies a pinned mutable binding failure preserves caller order.
    #[test]
    fn test_pinned_mut_named_binding_failure_recovers_before_user_code() {
        let _guard = lock_named_invocation_fixture();
        NAMED_INVOCATION_CALLS.store(0, Ordering::SeqCst);
        let mut target = Box::pin(NamedInvocationTarget);
        let invocation = reflect::invoke::PinnedMutInvocation::from_bindings(
            target.as_mut(),
            [
                InvocationBinding::named("third", InvocationArg::Owned(DynamicOwned::<Local>::new(31_u32))),
                InvocationBinding::named("first", InvocationArg::Owned(DynamicOwned::<Local>::new(29_u8))),
                InvocationBinding::named(
                    "second",
                    InvocationArg::Owned(DynamicOwned::<Local>::new(String::from("wrong"))),
                ),
            ],
        );

        let result = named_invocation_method_by_name("encode_pinned_mut")
            .invoke_pinned_mut_local(invocation)
            .expect("the pinned mutable adapter must be generated");
        let Err(failure) = result else {
            panic!("the wrong second type must fail before the pinned method runs")
        };

        assert!(matches!(
            failure.error().kind(),
            InvocationErrorKind::ArgumentTypeMismatch { index: 1, .. }
        ));
        assert!(
            std::error::Error::source(&failure)
                .and_then(|source| source.downcast_ref::<reflect::invoke::InvocationError>())
                .is_some(),
            "a pinned failure must expose its structured invocation error"
        );
        assert_eq!(NAMED_INVOCATION_CALLS.load(Ordering::SeqCst), 0);
        assert_eq!(failure.recovery().argument_name(0), Some("third"));
        assert_eq!(failure.recovery().argument_name(1), Some("first"));
        assert_eq!(failure.recovery().argument_name(2), Some("second"));
        let arguments = failure.recovery().arguments();
        assert_eq!(arguments.len(), 3);
        assert_eq!(arguments[0].type_id(), TypeId::of::<u32>());
        assert_eq!(arguments[1].type_id(), TypeId::of::<u8>());
        assert_eq!(arguments[2].type_id(), TypeId::of::<String>());
    }

    /// Verifies raw adapters reject named inputs instead of using caller order.
    #[test]
    fn test_raw_adapter_rejects_unbound_same_typed_named_arguments() {
        let _guard = lock_named_invocation_fixture();
        NAMED_INVOCATION_CALLS.store(0, Ordering::SeqCst);
        let method = named_invocation_method_by_name("encode_same");
        let adapter = method
            .adapter()
            .expect("the same-typed fixture must have a generated adapter");
        let invocation = Invocation::associated_bindings([
            InvocationBinding::named("second", InvocationArg::Owned(DynamicOwned::<Local>::new(34_u8))),
            InvocationBinding::named("first", InvocationArg::Owned(DynamicOwned::<Local>::new(33_u8))),
        ]);

        let result = adapter.invoke_local(invocation).expect("the raw local adapter exists");
        let Err(failure) = result else {
            panic!("a raw adapter must not silently treat named inputs as positional")
        };

        assert!(matches!(
            failure.error().kind(),
            InvocationErrorKind::NamedBindingRequiresDescriptor {
                input_index: 0,
                name,
            } if name.as_ref() == "second"
        ));
        assert_eq!(NAMED_INVOCATION_CALLS.load(Ordering::SeqCst), 0);
        assert_eq!(failure.recovery().argument_name(0), Some("second"));
        assert_eq!(failure.recovery().argument_name(1), Some("first"));
    }

    /// Verifies identifier-at-subpattern metadata remains positional-only.
    #[test]
    fn test_identifier_at_subpattern_binds_only_positionally() {
        let _guard = lock_named_invocation_fixture();
        NAMED_INVOCATION_CALLS.store(0, Ordering::SeqCst);
        let method = named_invocation_method_by_name("encode_at");
        assert_eq!(method.effective_method().parameters()[0].name(), None);
        assert!(matches!(
            method.effective_method().parameters()[0].pattern(),
            reflect::descriptor::ParameterPatternDescriptor::Destructure(_)
        ));
        let invocation = Invocation::associated_bindings([
            InvocationBinding::positional(InvocationArg::Owned(DynamicOwned::<Local>::new((35_u16, 36_u16)))),
            InvocationBinding::named("named", InvocationArg::Owned(DynamicOwned::<Local>::new(37_u32))),
        ]);

        let output = method
            .invoke_local(invocation)
            .expect("the local adapter must be generated")
            .expect("the at-subpattern must remain position-bindable");

        let InvocationOutput::Owned(output) = output else {
            panic!("the at-subpattern method must return an owned value")
        };
        assert_eq!(
            DynamicOwned::<Local>::downcast::<u64>(output)
                .unwrap_or_else(|_| panic!("the output must retain its exact type")),
            35_036_037
        );
        assert_eq!(NAMED_INVOCATION_CALLS.load(Ordering::SeqCst), 1);
    }

    /// Verifies type validation failure recovers every owned input in original
    /// order.
    #[test]
    fn test_validation_failure_recovers_receiver_and_all_arguments_in_order() {
        let invocation = Invocation::owned(
            DynamicOwned::<Local>::new(Counter { value: 9 }),
            [
                InvocationArg::Owned(DynamicOwned::<Local>::new(11_u8)),
                InvocationArg::Owned(DynamicOwned::<Local>::new(String::from("second"))),
                InvocationArg::Owned(DynamicOwned::<Local>::new(33_u32)),
            ],
        );

        let identity = method_identity(8);
        let failure = invocation
            .validate(
                &identity,
                ReceiverExpectation::owned::<Counter>(),
                &[
                    ArgumentExpectation::owned::<u8>(),
                    ArgumentExpectation::owned::<bool>(),
                    ArgumentExpectation::owned::<u32>(),
                ],
            )
            .expect_err("the second argument has the wrong exact type");

        assert_eq!(failure.error().method_identity(), &identity);
        let InvocationErrorKind::ArgumentTypeMismatch {
            index,
            expected,
            actual,
            expected_name,
        } = failure.error().kind()
        else {
            panic!("the mismatch should identify the argument type")
        };
        assert_eq!(*index, 1);
        assert_eq!(*expected, TypeId::of::<bool>());
        assert_eq!(*actual, TypeId::of::<String>());
        assert_eq!(*expected_name, "bool");
        let (receiver, arguments) = failure.into_recovery().into_parts();
        let Some(InvocationReceiver::Owned(receiver)) = receiver else {
            panic!("owned receiver should be recovered")
        };
        assert_eq!(
            DynamicOwned::<Local>::downcast::<Counter>(receiver)
                .unwrap_or_else(|_| panic!("receiver should remain intact")),
            Counter { value: 9 }
        );
        let [first, second, third] =
            <Vec<InvocationArg<'_, Local>> as TryInto<[InvocationArg<'_, Local>; 3]>>::try_into(arguments.into_vec())
                .unwrap_or_else(|_| panic!("all arguments should be recovered"));
        let InvocationArg::Owned(first) = first else {
            panic!("first argument should remain owned")
        };
        let InvocationArg::Owned(second) = second else {
            panic!("second argument should remain owned")
        };
        let InvocationArg::Owned(third) = third else {
            panic!("third argument should remain owned")
        };
        assert_eq!(
            DynamicOwned::<Local>::downcast::<u8>(first).unwrap_or_else(|_| panic!("first value should be intact")),
            11
        );
        assert_eq!(
            DynamicOwned::<Local>::downcast::<String>(second)
                .unwrap_or_else(|_| panic!("second value should be intact")),
            "second"
        );
        assert_eq!(
            DynamicOwned::<Local>::downcast::<u32>(third).unwrap_or_else(|_| panic!("third value should be intact")),
            33
        );
    }

    /// Verifies receiver-shape and arity mismatches are reported before
    /// execution.
    #[test]
    fn test_receiver_shape_and_argument_count_fail_before_execution() {
        let identity = method_identity(9);
        let invocation = Invocation::<Local>::associated([]);
        let failure = invocation
            .validate(&identity, ReceiverExpectation::borrowed::<Counter>(), &[])
            .expect_err("a shared receiver is required");
        assert!(matches!(
            failure.error().kind(),
            InvocationErrorKind::ReceiverModeMismatch {
                expected: Some(InvocationInputMode::Ref),
                actual: None,
            }
        ));

        let counter = Counter { value: 1 };
        let invocation = Invocation::borrowed(DynamicRef::<Local>::new(&counter), []);
        let failure = invocation
            .validate(
                &identity,
                ReceiverExpectation::borrowed::<Counter>(),
                &[ArgumentExpectation::owned::<u8>()],
            )
            .expect_err("one positional argument is required");
        assert!(matches!(
            failure.error().kind(),
            InvocationErrorKind::ArgumentCountMismatch { expected: 1, actual: 0 }
        ));
    }

    /// Verifies borrowed outputs expose explicit receiver and parameter
    /// origins.
    #[test]
    fn test_borrowed_outputs_retain_explicit_origins() {
        let text = String::from("borrowed");
        let shared: InvocationOutput<'_, Local> = InvocationOutput::Ref {
            value: DynamicRef::<Local>::new(&text),
            origins: [BorrowOrigin::Receiver, BorrowOrigin::Parameter(1)].into(),
        };
        let mut number = 4_u32;
        let mutable: InvocationOutput<'_, Local> = InvocationOutput::Mut {
            value: DynamicMut::<Local>::new(&mut number),
            origin: BorrowOrigin::Parameter(0),
        };

        let InvocationOutput::Ref { value, origins } = shared else {
            panic!("shared output should retain its variant")
        };
        assert_eq!(value.downcast_ref::<String>(), Some(&text));
        assert_eq!(origins.as_ref(), &[BorrowOrigin::Receiver, BorrowOrigin::Parameter(1)]);
        let InvocationOutput::Mut { mut value, origin } = mutable else {
            panic!("mutable output should retain its variant")
        };
        assert_eq!(origin, BorrowOrigin::Parameter(0));
        *value
            .downcast_mut::<u32>()
            .expect("mutable output should preserve the borrow") = 8;
        assert_eq!(value.downcast_ref::<u32>(), Some(&8));
    }

    /// Verifies local futures are lazy, may be non-Send, and retain local
    /// borrows.
    #[test]
    fn test_reflected_future_is_lazy_and_preserves_local_borrows() {
        let polls = Rc::new(Cell::new(0));
        let text = String::from("future output");
        let future_polls = Rc::clone(&polls);
        let future = async {
            future_polls.set(future_polls.get() + 1);
            InvocationOutput::Ref {
                value: DynamicRef::<Local>::new(&text),
                origins: [BorrowOrigin::Parameter(0)].into(),
            }
        };
        let mut reflected = ReflectedFuture::<Local>::new(future);

        assert_eq!(polls.get(), 0);
        let Poll::Ready(InvocationOutput::Ref { value, origins }) = poll_once(&mut reflected) else {
            panic!("the simple future should complete on its first poll")
        };
        assert_eq!(
            value.downcast_ref::<String>().map(String::as_str),
            Some("future output")
        );
        assert_eq!(origins.as_ref(), &[BorrowOrigin::Parameter(0)]);
        assert_eq!(polls.get(), 1);
    }

    /// Verifies thread-safe invocation values and futures retain their Send
    /// boundary.
    #[test]
    fn test_thread_safe_runtime_types_cross_send_boundary() {
        fn assert_send<T: Send>() {}

        assert_send::<Invocation<'static, ThreadSafe>>();
        assert_send::<InvocationArg<'static, ThreadSafe>>();
        assert_send::<InvocationOutput<'static, ThreadSafe>>();
        assert_send::<ReflectedFuture<'static, ThreadSafe>>();

        let future = ReflectedFuture::<ThreadSafe>::new(async {
            InvocationOutput::Owned(DynamicOwned::<ThreadSafe>::new(17_u32))
        });
        let output = std::thread::spawn(move || {
            let mut future = future;
            poll_once(&mut future)
        })
        .join()
        .expect("thread-safe reflected future should cross a thread boundary");
        let Poll::Ready(InvocationOutput::Owned(value)) = output else {
            panic!("thread-safe future should complete")
        };
        assert_eq!(
            DynamicOwned::<ThreadSafe>::downcast::<u32>(value).unwrap_or_else(|_| panic!("output should be u32")),
            17
        );
    }

    /// Verifies each invocation mode reports exact identities for owned,
    /// shared, mutable, and dedicated string dynamic values.
    #[test]
    fn test_invocation_modes_report_dynamic_value_type_identities() {
        let local_owned = DynamicOwned::<Local>::new(1_u8);
        let local_number = 2_u16;
        let local_ref = DynamicRef::<Local>::new(&local_number);
        let mut local_mut_number = 3_u32;
        let local_mut = DynamicMut::<Local>::new(&mut local_mut_number);
        assert_eq!(
            <Local as InvocationMode>::owned_type_id(&local_owned),
            TypeId::of::<u8>()
        );
        assert_eq!(<Local as InvocationMode>::ref_type_id(&local_ref), TypeId::of::<u16>());
        assert_eq!(<Local as InvocationMode>::mut_type_id(&local_mut), TypeId::of::<u32>());
        assert_eq!(
            <Local as InvocationMode>::ref_type_id(&DynamicRef::<Local>::new_str("text")),
            TypeId::of::<str>(),
        );

        let thread_owned = DynamicOwned::<ThreadSafe>::new(4_u8);
        let thread_number = 5_u16;
        let thread_ref = DynamicRef::<ThreadSafe>::new(&thread_number);
        let mut thread_mut_number = 6_u32;
        let thread_mut = DynamicMut::<ThreadSafe>::new(&mut thread_mut_number);
        assert_eq!(
            <ThreadSafe as InvocationMode>::owned_type_id(&thread_owned),
            TypeId::of::<u8>(),
        );
        assert_eq!(
            <ThreadSafe as InvocationMode>::ref_type_id(&thread_ref),
            TypeId::of::<u16>(),
        );
        assert_eq!(
            <ThreadSafe as InvocationMode>::mut_type_id(&thread_mut),
            TypeId::of::<u32>(),
        );
        assert_eq!(
            <ThreadSafe as InvocationMode>::ref_type_id(&DynamicRef::<ThreadSafe>::new_str("text")),
            TypeId::of::<str>(),
        );
    }

    /// Verifies ordinary panic propagation and the separate caught-panic
    /// payload type.
    #[test]
    fn test_user_panic_propagates_unchanged_and_catching_uses_separate_type() {
        let panic = catch_unwind(AssertUnwindSafe(|| {
            let _ = invoke_panicking(Invocation::<Local>::associated([]));
        }))
        .expect_err("ordinary invocation should propagate the user panic");
        assert_eq!(panic.downcast_ref::<&str>(), Some(&"user panic payload"));

        let method_identity = MemberId::new(
            "runtime_tests::Counter",
            "method",
            0,
            FragmentIdentity::new("qubit-reflect", "invoke::runtime_tests", 1, 1, "method", 7),
        );
        let caught = InvocationPanic::new(method_identity.clone(), Box::new(String::from("caught")));
        assert_eq!(caught.method_identity(), &method_identity);
        assert_eq!(
            caught.payload().downcast_ref::<String>().map(String::as_str),
            Some("caught")
        );
        assert_eq!(
            caught
                .downcast_payload::<String>()
                .expect("the payload type should be retained"),
            "caught"
        );
    }
}
