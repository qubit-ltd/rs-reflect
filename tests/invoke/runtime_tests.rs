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
    use std::task::Context;
    use std::task::Poll;
    use std::task::Waker;

    use qubit_reflect as reflect;
    use reflect::identity::FragmentIdentity;
    use reflect::identity::MemberId;
    use reflect::invoke::ArgumentExpectation;
    use reflect::invoke::BorrowOrigin;
    use reflect::invoke::Invocation;
    use reflect::invoke::InvocationArg;
    use reflect::invoke::InvocationErrorKind;
    use reflect::invoke::InvocationFailure;
    use reflect::invoke::InvocationInputMode;
    use reflect::invoke::InvocationOutput;
    use reflect::invoke::InvocationPanic;
    use reflect::invoke::InvocationReceiver;
    use reflect::invoke::ReceiverExpectation;
    use reflect::invoke::ReflectedFuture;
    use reflect::value::DynamicMut;
    use reflect::value::DynamicOwned;
    use reflect::value::DynamicRef;
    use reflect::value::Local;
    use reflect::value::ThreadSafe;

    /// A small receiver used by the hand-written invocation adapters.
    #[derive(Debug, Eq, PartialEq)]
    struct Counter {
        value: i32,
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
            failure.error.kind(),
            InvocationErrorKind::ArgumentModeMismatch {
                index: 0,
                expected: InvocationInputMode::Ref,
                actual: InvocationInputMode::Owned,
            }
        ));
        let (_, arguments) = failure.recovery.into_parts();
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

        assert_eq!(failure.error.method_identity(), &identity);
        let InvocationErrorKind::ArgumentTypeMismatch {
            index,
            expected,
            actual,
            expected_name,
        } = failure.error.kind()
        else {
            panic!("the mismatch should identify the argument type")
        };
        assert_eq!(*index, 1);
        assert_eq!(*expected, TypeId::of::<bool>());
        assert_eq!(*actual, TypeId::of::<String>());
        assert_eq!(*expected_name, "bool");
        let (receiver, arguments) = failure.recovery.into_parts();
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
            failure.error.kind(),
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
            failure.error.kind(),
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
