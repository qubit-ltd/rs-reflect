// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::marker::PhantomPinned;
use std::pin::Pin;

use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::Reflect;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::descriptor::TypeRegistration;
use crate::runtime::descriptor::metadata::TypeMetadata;
use crate::runtime::descriptor::method::EffectiveMethod;
use crate::runtime::descriptor::method::FunctionAbi;
use crate::runtime::descriptor::method::GenericDefinitionMetadata;
use crate::runtime::descriptor::method::InvocationAdapters;
use crate::runtime::descriptor::method::MethodCatalog;
use crate::runtime::descriptor::method::MethodImplementationSource;
use crate::runtime::descriptor::method::MethodMetadata;
use crate::runtime::descriptor::method::MethodNamespaceMetadata;
use crate::runtime::descriptor::method::MethodQualifiers;
use crate::runtime::descriptor::method::MethodVisibility;
use crate::runtime::descriptor::method::ParameterMetadata;
use crate::runtime::descriptor::method::ParameterPattern;
use crate::runtime::descriptor::method::PassingMode;
use crate::runtime::descriptor::method::ReceiverDescriptor;
use crate::runtime::descriptor::method::ReceiverKind;
use crate::runtime::descriptor::method::ResolvedMethodArguments;
use crate::runtime::descriptor::method::ResolvedMethodSpecialization;
use crate::runtime::descriptor::method::ReturnKind;
use crate::runtime::descriptor::method::ReturnMetadata;
use crate::runtime::error::RecoverableFailure;
use crate::runtime::expression::ExpressionData;
use crate::runtime::identity::GenericArgumentIdentity;
use crate::runtime::invoke::Invocation;
use crate::runtime::invoke::InvocationAdapter;
use crate::runtime::invoke::InvocationAvailability;
use crate::runtime::invoke::InvocationErrorKind;
use crate::runtime::invoke::InvocationOutput;
use crate::runtime::invoke::InvocationReceiver;
use crate::runtime::invoke::Local;
use crate::runtime::invoke::PinnedInvocation;
use crate::runtime::invoke::PinnedInvocationAdapter;
use crate::runtime::invoke::PinnedReceiver;
use crate::runtime::invoke::ThreadSafe;
use crate::runtime::invoke::ValidatedInvocation;

struct Counter(u64);
impl Reflect for Counter {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        static REGISTRATION: TypeRegistration<Counter> = TypeRegistration::new();
        &REGISTRATION
    }
}
struct Amount(u64);
#[derive(Debug)]
struct Wrong;
struct PinnedCounter {
    value: u64,
    _pin: PhantomPinned,
}

trait PinnedProbe {
    fn probe(&self) -> u32;
}

struct PinnedProbeValue(u32, PhantomPinned);
struct OtherPinnedCounter {
    value: u64,
    _pin: PhantomPinned,
}

impl PinnedProbe for PinnedProbeValue {
    fn probe(&self) -> u32 {
        self.0
    }
}
impl Reflect for Amount {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        static REGISTRATION: TypeRegistration<Amount> = TypeRegistration::new();
        &REGISTRATION
    }
}
impl Reflect for Wrong {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        static REGISTRATION: TypeRegistration<Wrong> = TypeRegistration::new();
        &REGISTRATION
    }
}
impl Reflect for PinnedCounter {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        static REGISTRATION: TypeRegistration<PinnedCounter> = TypeRegistration::new();
        &REGISTRATION
    }
}

static COUNTER_TYPE: TypeMetadata = TypeMetadata::new::<Counter>();
static PINNED_COUNTER_TYPE: TypeMetadata = TypeMetadata::new::<PinnedCounter>();
static AMOUNT_TYPE: TypeMetadata = TypeMetadata::new::<Amount>();
fn counter_type_metadata() -> &'static TypeMetadata {
    &COUNTER_TYPE
}
fn amount_type_metadata() -> &'static TypeMetadata {
    &AMOUNT_TYPE
}
fn pinned_counter_type_metadata() -> &'static TypeMetadata {
    &PINNED_COUNTER_TYPE
}
static EMPTY_GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata {
    parameters: &[],
    predicates: &[],
};
static U64_EXPRESSION: ExpressionData = ExpressionData::Named {
    path: "Amount",
    definition: None,
    arguments: &[],
};
static COUNTER_RECEIVER: ReceiverDescriptor = ReceiverDescriptor {
    kind: ReceiverKind::Mutable,
    type_metadata: Some(counter_type_metadata),
    expression: None,
};
static PINNED_MUT_RECEIVER: ReceiverDescriptor = ReceiverDescriptor {
    kind: ReceiverKind::PinnedMutable,
    type_metadata: Some(pinned_counter_type_metadata),
    expression: None,
};
static VALUE_PARAMETER: ParameterMetadata = ParameterMetadata {
    index: 0,
    pattern: ParameterPattern::Identifier("amount"),
    passing: PassingMode::Owned,
    expression: &U64_EXPRESSION,
    concrete_type: Some(amount_type_metadata),
};
static FIRST_PARAMETER: ParameterMetadata = ParameterMetadata {
    index: 0,
    pattern: ParameterPattern::Identifier("first"),
    passing: PassingMode::Owned,
    expression: &U64_EXPRESSION,
    concrete_type: Some(amount_type_metadata),
};
static SECOND_PARAMETER: ParameterMetadata = ParameterMetadata {
    index: 1,
    pattern: ParameterPattern::Identifier("second"),
    passing: PassingMode::Owned,
    expression: &U64_EXPRESSION,
    concrete_type: Some(amount_type_metadata),
};
static TWO_PARAMETERS: [ParameterMetadata; 2] = [FIRST_PARAMETER, SECOND_PARAMETER];
static UNIT_RETURN: ReturnMetadata = ReturnMetadata {
    kind: ReturnKind::Unit,
    expression: None,
    concrete_type: None,
};
static METHOD: MethodMetadata = MethodMetadata {
    name: "add",
    namespace: MethodNamespaceMetadata::Inherent {
        implementation: "Counter::inherent",
    },
    visibility: MethodVisibility::Public,
    receiver: Some(COUNTER_RECEIVER),
    parameters: &[VALUE_PARAMETER],
    output: &UNIT_RETURN,
    qualifiers: MethodQualifiers {
        is_const: false,
        is_async: false,
        is_unsafe: false,
        abi: Some(FunctionAbi::Rust),
        is_variadic: false,
    },
    generics: &EMPTY_GENERICS,
    is_generic: false,
    default_body: false,
};
static TRAIT_METHOD: MethodMetadata = MethodMetadata {
    name: "add",
    namespace: MethodNamespaceMetadata::Trait {
        trait_: "CounterOps",
        implementation: "CounterOps for Counter",
    },
    visibility: MethodVisibility::Public,
    receiver: Some(COUNTER_RECEIVER),
    parameters: &[VALUE_PARAMETER],
    output: &UNIT_RETURN,
    qualifiers: MethodQualifiers {
        is_const: false,
        is_async: false,
        is_unsafe: false,
        abi: Some(FunctionAbi::Rust),
        is_variadic: false,
    },
    generics: &EMPTY_GENERICS,
    is_generic: false,
    default_body: true,
};
static TWO_ARGUMENT_METHOD: MethodMetadata = MethodMetadata {
    name: "add_both",
    namespace: MethodNamespaceMetadata::Inherent {
        implementation: "Counter::inherent",
    },
    visibility: MethodVisibility::Public,
    receiver: Some(COUNTER_RECEIVER),
    parameters: &TWO_PARAMETERS,
    output: &UNIT_RETURN,
    qualifiers: MethodQualifiers {
        is_const: false,
        is_async: false,
        is_unsafe: false,
        abi: None,
        is_variadic: false,
    },
    generics: &EMPTY_GENERICS,
    is_generic: false,
    default_body: false,
};

struct AddAdapter;
impl InvocationAdapter<Local> for AddAdapter {
    fn invoke<'call>(&self, invocation: ValidatedInvocation<'call, Local>) -> InvocationOutput<'call, Local> {
        let (receiver, arguments) = invocation.into_parts();
        let Some(InvocationReceiver::Mut(mut receiver)) = receiver else {
            panic!("validated mutable receiver")
        };
        let amount = match arguments.into_vec().into_iter().next().expect("one argument") {
            crate::runtime::invoke::InvocationValue::Owned(value) => match value.downcast::<Amount>() {
                Ok(value) => value,
                Err(_) => panic!("validated Amount"),
            },
            _ => panic!("validated owned argument"),
        };
        match receiver.downcast_mut::<Counter>() {
            Some(counter) => counter.0 += amount.0,
            None => panic!("validated Counter"),
        }
        InvocationOutput::Unit
    }
}

struct AddThreadSafeAdapter;
impl InvocationAdapter<ThreadSafe> for AddThreadSafeAdapter {
    fn invoke<'call>(&self, invocation: ValidatedInvocation<'call, ThreadSafe>) -> InvocationOutput<'call, ThreadSafe> {
        let (receiver, arguments) = invocation.into_parts();
        let Some(InvocationReceiver::Mut(mut receiver)) = receiver else {
            panic!("validated mutable receiver")
        };
        let amount = match arguments.into_vec().into_iter().next().expect("one argument") {
            crate::runtime::invoke::InvocationValue::Owned(value) => match value.downcast::<Amount>() {
                Ok(value) => value,
                Err(_) => panic!("validated Amount"),
            },
            _ => panic!("validated owned argument"),
        };
        match receiver.downcast_mut::<Counter>() {
            Some(counter) => counter.0 += amount.0,
            None => panic!("validated Counter"),
        }
        InvocationOutput::Unit
    }
}

struct PinnedReadAdapter;
impl PinnedInvocationAdapter<Local> for PinnedReadAdapter {
    fn receiver_type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<PinnedCounter>()
    }
    fn receiver_kind(&self) -> ReceiverKind {
        ReceiverKind::PinnedMutable
    }
    fn invoke<'call>(
        &self,
        receiver: PinnedReceiver<'call>,
        _invocation: ValidatedInvocation<'call, Local>,
    ) -> InvocationOutput<'call, Local> {
        let Some(receiver) = receiver.downcast_mut::<PinnedCounter>() else {
            panic!("validated pinned receiver type and mode")
        };
        let _value = receiver.as_ref().get_ref().value;
        InvocationOutput::Unit
    }
}
static PINNED_READ_ADAPTER: PinnedReadAdapter = PinnedReadAdapter;
static PINNED_ADAPTERS: InvocationAdapters = InvocationAdapters {
    local: None,
    thread_safe: None,
    catching_local: None,
    catching_thread_safe: None,
    pinned_local: Some(&PINNED_READ_ADAPTER),
    pinned_thread_safe: None,
};
static PINNED_SPECIALIZATIONS: [ResolvedMethodSpecialization; 1] = [ResolvedMethodSpecialization {
    arguments: ResolvedMethodArguments::Static(&EMPTY_ARGUMENTS),
    argument_metadata: None,
    availability: InvocationAvailability::Callable,
    adapters: PINNED_ADAPTERS,
}];

static ADD_ADAPTER: AddAdapter = AddAdapter;
static ADD_THREAD_SAFE_ADAPTER: AddThreadSafeAdapter = AddThreadSafeAdapter;
static ADAPTERS: InvocationAdapters = InvocationAdapters {
    local: Some(&ADD_ADAPTER),
    thread_safe: Some(&ADD_THREAD_SAFE_ADAPTER),
    catching_local: Some(&ADD_ADAPTER),
    catching_thread_safe: Some(&ADD_THREAD_SAFE_ADAPTER),
    pinned_local: None,
    pinned_thread_safe: None,
};
struct AddBothAdapter;
impl InvocationAdapter<Local> for AddBothAdapter {
    fn invoke<'call>(&self, invocation: ValidatedInvocation<'call, Local>) -> InvocationOutput<'call, Local> {
        let (receiver, arguments) = invocation.into_parts();
        let Some(InvocationReceiver::Mut(mut receiver)) = receiver else {
            panic!("validated mutable receiver")
        };
        let mut amounts = arguments.into_vec().into_iter().map(|argument| match argument {
            crate::runtime::invoke::InvocationValue::Owned(value) => match value.downcast::<Amount>() {
                Ok(value) => value.0,
                Err(_) => panic!("validated Amount"),
            },
            _ => panic!("validated owned argument"),
        });
        let sum = amounts.next().expect("first argument") + amounts.next().expect("second argument");
        match receiver.downcast_mut::<Counter>() {
            Some(counter) => counter.0 += sum,
            None => panic!("validated Counter"),
        }
        InvocationOutput::Unit
    }
}
static ADD_BOTH_ADAPTER: AddBothAdapter = AddBothAdapter;
static ADD_BOTH_ADAPTERS: InvocationAdapters = InvocationAdapters {
    local: Some(&ADD_BOTH_ADAPTER),
    thread_safe: None,
    catching_local: Some(&ADD_BOTH_ADAPTER),
    catching_thread_safe: None,
    pinned_local: None,
    pinned_thread_safe: None,
};
static TWO_ARGUMENT_SPECIALIZATIONS: [ResolvedMethodSpecialization; 1] = [ResolvedMethodSpecialization {
    arguments: ResolvedMethodArguments::Static(&EMPTY_ARGUMENTS),
    argument_metadata: None,
    availability: InvocationAvailability::Callable,
    adapters: ADD_BOTH_ADAPTERS,
}];
static EMPTY_ARGUMENTS: [GenericArgumentIdentity; 0] = [];
static SPECIALIZATION_SET: [ResolvedMethodSpecialization; 1] = [ResolvedMethodSpecialization {
    arguments: ResolvedMethodArguments::Static(&EMPTY_ARGUMENTS),
    argument_metadata: None,
    availability: InvocationAvailability::Callable,
    adapters: ADAPTERS,
}];
static UNAVAILABLE_REASONS: [crate::runtime::invoke::InvocationUnavailableReason; 3] = [
    crate::runtime::invoke::InvocationUnavailableReason::UnsupportedReceiver,
    crate::runtime::invoke::InvocationUnavailableReason::DisabledByPolicy,
    crate::runtime::invoke::InvocationUnavailableReason::UnsupportedBorrowedReturn,
];
static UNAVAILABLE_SPECIALIZATIONS: [ResolvedMethodSpecialization; 1] = [ResolvedMethodSpecialization {
    arguments: ResolvedMethodArguments::Static(&EMPTY_ARGUMENTS),
    argument_metadata: None,
    availability: InvocationAvailability::Unavailable {
        reasons: &UNAVAILABLE_REASONS,
    },
    adapters: ADAPTERS,
}];

static UNSAFE_METHOD: MethodMetadata = MethodMetadata {
    name: "unsafe_add",
    namespace: MethodNamespaceMetadata::Inherent {
        implementation: "Counter::inherent",
    },
    visibility: MethodVisibility::Public,
    receiver: Some(COUNTER_RECEIVER),
    parameters: &[VALUE_PARAMETER],
    output: &UNIT_RETURN,
    qualifiers: MethodQualifiers {
        is_const: false,
        is_async: false,
        is_unsafe: true,
        abi: None,
        is_variadic: false,
    },
    generics: &EMPTY_GENERICS,
    is_generic: false,
    default_body: false,
};
static PRIVATE_METHOD: MethodMetadata = MethodMetadata {
    name: "private_add",
    namespace: MethodNamespaceMetadata::Inherent {
        implementation: "Counter::inherent",
    },
    visibility: MethodVisibility::Private,
    receiver: Some(COUNTER_RECEIVER),
    parameters: &[VALUE_PARAMETER],
    output: &UNIT_RETURN,
    qualifiers: MethodQualifiers {
        is_const: false,
        is_async: false,
        is_unsafe: false,
        abi: None,
        is_variadic: false,
    },
    generics: &EMPTY_GENERICS,
    is_generic: false,
    default_body: false,
};
static ASYNC_METHOD: MethodMetadata = MethodMetadata {
    name: "async_add",
    namespace: MethodNamespaceMetadata::Inherent {
        implementation: "Counter::inherent",
    },
    visibility: MethodVisibility::Public,
    receiver: Some(COUNTER_RECEIVER),
    parameters: &[VALUE_PARAMETER],
    output: &UNIT_RETURN,
    qualifiers: MethodQualifiers {
        is_const: false,
        is_async: true,
        is_unsafe: false,
        abi: None,
        is_variadic: false,
    },
    generics: &EMPTY_GENERICS,
    is_generic: false,
    default_body: false,
};
static PANIC_METHOD: MethodMetadata = MethodMetadata {
    name: "panic_after_start",
    namespace: MethodNamespaceMetadata::Inherent {
        implementation: "Counter::inherent",
    },
    visibility: MethodVisibility::Public,
    receiver: Some(COUNTER_RECEIVER),
    parameters: &[VALUE_PARAMETER],
    output: &UNIT_RETURN,
    qualifiers: MethodQualifiers {
        is_const: false,
        is_async: false,
        is_unsafe: false,
        abi: None,
        is_variadic: false,
    },
    generics: &EMPTY_GENERICS,
    is_generic: false,
    default_body: false,
};
static PINNED_METHOD: MethodMetadata = MethodMetadata {
    name: "pinned_read",
    namespace: MethodNamespaceMetadata::Inherent {
        implementation: "PinnedCounter::inherent",
    },
    visibility: MethodVisibility::Public,
    receiver: Some(PINNED_MUT_RECEIVER),
    parameters: &[VALUE_PARAMETER],
    output: &UNIT_RETURN,
    qualifiers: MethodQualifiers {
        is_const: false,
        is_async: false,
        is_unsafe: false,
        abi: None,
        is_variadic: false,
    },
    generics: &EMPTY_GENERICS,
    is_generic: false,
    default_body: false,
};
struct PanickingAdapter;
impl InvocationAdapter<Local> for PanickingAdapter {
    fn invoke<'call>(&self, _invocation: ValidatedInvocation<'call, Local>) -> InvocationOutput<'call, Local> {
        panic!("user method panic")
    }
}
static PANICKING_ADAPTER: PanickingAdapter = PanickingAdapter;
static BUILT_PANICKING_ADAPTERS: InvocationAdapters = InvocationAdapters::builder()
    .local(&PANICKING_ADAPTER)
    .catching_local(&PANICKING_ADAPTER)
    .build();
static PANICKING_ADAPTERS: InvocationAdapters = InvocationAdapters {
    local: Some(&PANICKING_ADAPTER),
    thread_safe: None,
    catching_local: Some(&PANICKING_ADAPTER),
    catching_thread_safe: None,
    pinned_local: None,
    pinned_thread_safe: None,
};
static NO_CATCH_ADAPTERS: InvocationAdapters = InvocationAdapters {
    local: Some(&ADD_ADAPTER),
    thread_safe: None,
    catching_local: None,
    catching_thread_safe: None,
    pinned_local: None,
    pinned_thread_safe: None,
};
static NO_CATCH_SPECIALIZATIONS: [ResolvedMethodSpecialization; 1] = [ResolvedMethodSpecialization {
    arguments: ResolvedMethodArguments::Static(&EMPTY_ARGUMENTS),
    argument_metadata: None,
    availability: InvocationAvailability::Callable,
    adapters: NO_CATCH_ADAPTERS,
}];
static PANIC_SPECIALIZATIONS: [ResolvedMethodSpecialization; 1] = [ResolvedMethodSpecialization {
    arguments: ResolvedMethodArguments::Static(&EMPTY_ARGUMENTS),
    argument_metadata: None,
    availability: InvocationAvailability::Callable,
    adapters: PANICKING_ADAPTERS,
}];
static BUILT_PANIC_SPECIALIZATIONS: [ResolvedMethodSpecialization; 1] = [ResolvedMethodSpecialization {
    arguments: ResolvedMethodArguments::Static(&EMPTY_ARGUMENTS),
    argument_metadata: None,
    availability: InvocationAvailability::Callable,
    adapters: BUILT_PANICKING_ADAPTERS,
}];

fn effective() -> EffectiveMethod {
    effective_for(&METHOD)
}

fn effective_for(metadata: &'static MethodMetadata) -> EffectiveMethod {
    EffectiveMethod {
        target: counter_type_metadata,
        metadata,
        source: MethodImplementationSource::Declared,
        availability: crate::runtime::descriptor::method::MethodAvailability::Callable,
        specializations: &SPECIALIZATION_SET,
        specialization_specs: &[],
        adapters: None,
    }
}

fn type_descriptor(context: &ReflectionContext) -> TypeDescriptor<'_> {
    TypeDescriptor::new(&COUNTER_TYPE, context)
}

#[test]
fn invocation_calls_the_selected_adapter_with_checked_typed_inputs() {
    let context = ReflectionContext::empty();
    let methods = [effective()];
    let catalog = MethodCatalog::new(type_descriptor(&context), &methods);
    let method = catalog.method("add").expect("unique method");
    let mut counter = Counter(5);

    match method.invoke(Invocation::<Local>::on_mut(&mut counter).arg(Amount(7))) {
        Ok(output) => assert!(output.into_unit().is_ok(), "adapter returns unit"),
        Err(_) => panic!("valid invocation"),
    }

    assert_eq!(counter.0, 12);
}

#[test]
fn thread_safe_mode_selects_a_send_sync_adapter() {
    let context = ReflectionContext::empty();
    let methods = [effective()];
    let method = MethodCatalog::new(type_descriptor(&context), &methods)
        .method("add")
        .expect("method");
    let mut counter = Counter(2);

    match method.invoke(Invocation::<ThreadSafe>::on_mut(&mut counter).arg(Amount(8))) {
        Ok(output) => assert!(output.into_unit().is_ok(), "adapter returns unit"),
        Err(_) => panic!("thread-safe invocation is supported"),
    }

    assert_eq!(counter.0, 10);
}

#[test]
fn caller_ordered_named_and_positional_bindings_reach_the_adapter_in_declaration_order() {
    let context = ReflectionContext::empty();
    let method = EffectiveMethod {
        target: counter_type_metadata,
        metadata: &TWO_ARGUMENT_METHOD,
        source: MethodImplementationSource::Declared,
        availability: crate::runtime::descriptor::method::MethodAvailability::Callable,
        specializations: &TWO_ARGUMENT_SPECIALIZATIONS,
        specialization_specs: &[],
        adapters: None,
    };
    let methods = [method];
    let method = MethodCatalog::new(type_descriptor(&context), &methods)
        .method("add_both")
        .expect("method");
    let mut counter = Counter(1);
    let invocation = Invocation::<Local>::on_mut(&mut counter)
        .named_arg("second", Amount(4))
        .arg(Amount(3));

    match method.invoke(invocation) {
        Ok(output) => assert!(output.into_unit().is_ok(), "adapter returns unit"),
        Err(_) => panic!("mixed named and positional bindings are valid"),
    }

    assert_eq!(counter.0, 8);
}

#[test]
fn lookup_preserves_missing_unique_and_ambiguous_states() {
    let context = ReflectionContext::empty();
    let inherent = effective();
    let trait_method = EffectiveMethod {
        metadata: &TRAIT_METHOD,
        source: MethodImplementationSource::Overridden,
        ..effective()
    };
    let methods = [inherent, trait_method];
    let catalog = MethodCatalog::new(type_descriptor(&context), &methods);

    assert!(catalog.lookup_method("absent").expect("context").is_missing());
    assert!(
        matches!(catalog.lookup_method("add").expect("context"), crate::runtime::lookup::Lookup::Ambiguous(candidates) if candidates.len() == 2)
    );
    assert!(matches!(
        catalog.method("add"),
        Err(crate::runtime::error::LookupError::Ambiguous { .. })
    ));
    assert_eq!(
        catalog
            .inherent_method("add")
            .expect("inherent method")
            .implementation_source(),
        MethodImplementationSource::Declared
    );
    assert_eq!(
        catalog
            .trait_method("CounterOps", "add")
            .expect("trait method")
            .implementation_source(),
        MethodImplementationSource::Overridden
    );
}

#[test]
fn preflight_failure_keeps_inputs_available_for_correction_and_retry() {
    let context = ReflectionContext::empty();
    let methods = [effective()];
    let method = MethodCatalog::new(type_descriptor(&context), &methods)
        .method("add")
        .expect("method");
    let mut counter = Counter(10);
    let failure = match method.invoke(Invocation::<Local>::on_mut(&mut counter).arg(Wrong)) {
        Ok(_) => panic!("wrong exact type must fail"),
        Err(failure) => failure,
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ArgumentTypeMismatch { index: 0, .. }
    ));

    let invocation = match failure.into_invocation().replace_arg(0, Amount(4)) {
        Ok(invocation) => invocation,
        Err(_) => panic!("binding exists"),
    };
    match method.invoke(invocation) {
        Ok(output) => assert!(output.into_unit().is_ok(), "adapter returns unit"),
        Err(_) => panic!("corrected invocation succeeds"),
    }
    assert_eq!(counter.0, 14);
}

#[test]
#[allow(unsafe_code)]
fn unsafe_method_rejection_preserves_receiver_and_all_arguments() {
    let context = ReflectionContext::empty();
    let methods = [effective_for(&UNSAFE_METHOD)];
    let method = MethodCatalog::new(type_descriptor(&context), &methods)
        .method("unsafe_add")
        .expect("method");
    let mut counter = Counter(10);
    let failure = match method.invoke(Invocation::<Local>::on_mut(&mut counter).arg(Amount(3))) {
        Ok(_) => panic!("safe invoke rejects unsafe methods"),
        Err(failure) => failure,
    };
    assert!(matches!(
        RecoverableFailure::error(&failure).kind(),
        InvocationErrorKind::UnsafeMethod
    ));
    assert_eq!(
        RecoverableFailure::recovery(&failure).as_invocation().argument_count(),
        1
    );
    let (error, recovery) = RecoverableFailure::into_parts(failure);
    assert!(matches!(error.kind(), InvocationErrorKind::UnsafeMethod));
    let recovered = recovery.into_invocation();
    assert_eq!(recovered.argument_count(), 1);
    drop(recovered);
    assert_eq!(counter.0, 10);

    let mut counter = Counter(10);
    {
        let output = unsafe { method.invoke_unchecked(Invocation::<Local>::on_mut(&mut counter).arg(Amount(3))) };
        let output = match output {
            Ok(output) => output,
            Err(_) => panic!("unchecked invocation is callable"),
        };
        match output.into_unit() {
            Ok(()) => {}
            Err(_) => panic!("unchecked invocation returns unit"),
        }
    }
    assert_eq!(counter.0, 13);

    let private = EffectiveMethod {
        target: counter_type_metadata,
        metadata: &PRIVATE_METHOD,
        source: MethodImplementationSource::Declared,
        availability: crate::runtime::descriptor::method::MethodAvailability::Callable,
        specializations: &SPECIALIZATION_SET,
        specialization_specs: &[],
        adapters: None,
    };
    let methods = [private];
    let method = MethodCatalog::new(type_descriptor(&context), &methods)
        .method("private_add")
        .expect("private method");
    let mut counter = Counter(1);
    let failure = match method.invoke(Invocation::<Local>::on_mut(&mut counter).arg(Amount(1))) {
        Ok(_) => panic!("safe invocation rejects private method"),
        Err(failure) => failure,
    };
    assert!(matches!(failure.error().kind(), InvocationErrorKind::VisibilityDenied));
    drop(failure.into_invocation());
    {
        let output = unsafe { method.invoke_unchecked(Invocation::<Local>::on_mut(&mut counter).arg(Amount(2))) };
        match output {
            Ok(output) => match output.into_unit() {
                Ok(()) => {}
                Err(_) => panic!("private adapter returns unit"),
            },
            Err(_) => panic!("unchecked call bypasses visibility"),
        }
    }
    assert_eq!(counter.0, 3);
}

#[test]
fn ordinary_invoke_leaves_user_panics_unwinding() {
    let context = ReflectionContext::empty();
    let method = EffectiveMethod {
        target: counter_type_metadata,
        metadata: &PANIC_METHOD,
        source: MethodImplementationSource::Declared,
        availability: crate::runtime::descriptor::method::MethodAvailability::Callable,
        specializations: &PANIC_SPECIALIZATIONS,
        specialization_specs: &[],
        adapters: None,
    };
    let methods = [method];
    let method = MethodCatalog::new(type_descriptor(&context), &methods)
        .method("panic_after_start")
        .expect("method");
    let mut counter = Counter(11);
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = method.invoke(Invocation::<Local>::on_mut(&mut counter).arg(Amount(1)));
    }));

    assert!(outcome.is_err(), "invoke does not intercept the method panic");
    assert_eq!(counter.0, 11);
}

#[test]
fn catching_invoke_reports_panics_but_preserves_preflight_rejections() {
    let context = ReflectionContext::empty();
    let method = EffectiveMethod {
        target: counter_type_metadata,
        metadata: &PANIC_METHOD,
        source: MethodImplementationSource::Declared,
        availability: crate::runtime::descriptor::method::MethodAvailability::Callable,
        specializations: &PANIC_SPECIALIZATIONS,
        specialization_specs: &[],
        adapters: None,
    };
    let methods = [method];
    let method = MethodCatalog::new(type_descriptor(&context), &methods)
        .method("panic_after_start")
        .expect("method");
    let mut counter = Counter(11);
    let failure = match method.invoke_catching(Invocation::<Local>::on_mut(&mut counter).arg(Amount(1))) {
        Ok(_) => panic!("catching call returns the panic"),
        Err(failure) => failure,
    };
    match failure {
        crate::runtime::invoke::CaughtInvocationFailure::Panicked(panic) => {
            assert_eq!(panic.message(), Some("user method panic"));
        }
        crate::runtime::invoke::CaughtInvocationFailure::Rejected(_) => {
            panic!("valid invocation reaches the adapter")
        }
    }

    let mut counter = Counter(11);
    let failure = match method.invoke_catching(Invocation::<Local>::on_mut(&mut counter).arg(Wrong)) {
        Ok(_) => panic!("wrong argument is rejected before adapter execution"),
        Err(failure) => failure,
    };
    match failure {
        crate::runtime::invoke::CaughtInvocationFailure::Rejected(failure) => {
            assert!(matches!(
                failure.error().kind(),
                InvocationErrorKind::ArgumentTypeMismatch { index: 0, .. }
            ));
            let recovered = failure.into_invocation();
            assert_eq!(recovered.argument_count(), 1);
            drop(recovered);
        }
        crate::runtime::invoke::CaughtInvocationFailure::Panicked(_) => {
            panic!("preflight rejection is not a panic")
        }
    }

    let no_catch = EffectiveMethod {
        target: counter_type_metadata,
        metadata: &METHOD,
        source: MethodImplementationSource::Declared,
        availability: crate::runtime::descriptor::method::MethodAvailability::Callable,
        specializations: &NO_CATCH_SPECIALIZATIONS,
        specialization_specs: &[],
        adapters: None,
    };
    let methods = [no_catch];
    let method = MethodCatalog::new(type_descriptor(&context), &methods)
        .method("add")
        .expect("method");
    let mut counter = Counter(0);
    let failure = match method.invoke_catching(Invocation::<Local>::on_mut(&mut counter).arg(Amount(4))) {
        Ok(_) => panic!("missing catching adapter is recoverable"),
        Err(failure) => failure,
    };
    {
        match failure {
            crate::runtime::invoke::CaughtInvocationFailure::Rejected(failure) => {
                assert!(matches!(
                    failure.error().kind(),
                    InvocationErrorKind::ValueModeUnavailable
                ));
                let recovered = failure.into_invocation();
                assert_eq!(recovered.argument_count(), 1);
                drop(recovered);
            }
            crate::runtime::invoke::CaughtInvocationFailure::Panicked(_) => {
                panic!("missing adapter must be rejected before execution")
            }
        }
    }

    let asynchronous = EffectiveMethod {
        target: counter_type_metadata,
        metadata: &ASYNC_METHOD,
        source: MethodImplementationSource::Declared,
        availability: crate::runtime::descriptor::method::MethodAvailability::Callable,
        specializations: &SPECIALIZATION_SET,
        specialization_specs: &[],
        adapters: None,
    };
    let methods = [asynchronous];
    let method = MethodCatalog::new(type_descriptor(&context), &methods)
        .method("async_add")
        .expect("async method");
    let mut counter = Counter(0);
    let failure = match method.invoke_catching(Invocation::<Local>::on_mut(&mut counter).arg(Amount(1))) {
        Ok(_) => panic!("synchronous catcher declines async methods"),
        Err(crate::runtime::invoke::CaughtInvocationFailure::Rejected(failure)) => failure,
        Err(crate::runtime::invoke::CaughtInvocationFailure::Panicked(_)) => {
            panic!("async method is rejected before execution")
        }
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ValueModeUnavailable
    ));
    drop(failure.into_invocation());
}

#[test]
fn builder_catching_slot_catches_only_explicit_invocation() {
    let context = ReflectionContext::empty();
    let method = EffectiveMethod {
        target: counter_type_metadata,
        metadata: &PANIC_METHOD,
        source: MethodImplementationSource::Declared,
        availability: crate::runtime::descriptor::method::MethodAvailability::Callable,
        specializations: &BUILT_PANIC_SPECIALIZATIONS,
        specialization_specs: &[],
        adapters: None,
    };
    let methods = [method];
    let method = MethodCatalog::new(type_descriptor(&context), &methods)
        .method("panic_after_start")
        .expect("method");

    let mut counter = Counter(11);
    let ordinary = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = method.invoke(Invocation::<Local>::on_mut(&mut counter).arg(Amount(1)));
    }));
    assert!(ordinary.is_err(), "ordinary invoke must keep propagating panics");

    let mut counter = Counter(11);
    let caught = match method.invoke_catching(Invocation::<Local>::on_mut(&mut counter).arg(Amount(1))) {
        Ok(_) => panic!("a panicking method is reported by invoke_catching"),
        Err(caught) => caught,
    };
    match caught {
        crate::runtime::invoke::CaughtInvocationFailure::Panicked(panic) => {
            assert_eq!(panic.message(), Some("user method panic"));
        }
        crate::runtime::invoke::CaughtInvocationFailure::Rejected(_) => {
            panic!("valid invocation reaches the catching adapter")
        }
    }
}

#[test]
fn pinned_invocation_uses_pinned_adapter_and_recovers_pin_on_preflight_failure() {
    let context = ReflectionContext::empty();
    let effective = EffectiveMethod {
        target: pinned_counter_type_metadata,
        metadata: &PINNED_METHOD,
        source: MethodImplementationSource::Declared,
        availability: crate::runtime::descriptor::method::MethodAvailability::Callable,
        specializations: &PINNED_SPECIALIZATIONS,
        specialization_specs: &[],
        adapters: None,
    };
    let methods = [effective];
    let method = MethodCatalog::new(TypeDescriptor::new(&PINNED_COUNTER_TYPE, &context), &methods)
        .method("pinned_read")
        .expect("method");
    let mut pinned = Box::pin(PinnedCounter {
        value: 19,
        _pin: PhantomPinned,
    });
    {
        let output = method.invoke_pinned(PinnedInvocation::<Local, _>::mutable(pinned.as_mut()).arg(Amount(5)));
        match output {
            Ok(output) => match output.into_unit() {
                Ok(()) => {}
                Err(_) => panic!("pinned adapter returns unit"),
            },
            Err(_) => panic!("valid pinned invocation succeeds"),
        }
    }
    {
        let failure = match method.invoke_pinned(PinnedInvocation::<Local, _>::mutable(pinned.as_mut()).arg(Wrong)) {
            Ok(_) => panic!("wrong argument type is rejected before pinned adapter"),
            Err(failure) => failure,
        };
        assert!(matches!(
            failure.error().kind(),
            InvocationErrorKind::ArgumentTypeMismatch { index: 0, .. }
        ));
        let recovered = failure.into_invocation();
        assert_eq!(recovered.argument_count(), 1);
        assert!(recovered.receiver_is_mutable());
        assert_eq!(recovered.receiver_type_id(), std::any::TypeId::of::<PinnedCounter>());
        drop(recovered);
    }
}

#[test]
fn pinned_receiver_type_mismatch_recovers_the_original_pin_and_arguments() {
    let context = ReflectionContext::empty();
    let effective = EffectiveMethod {
        target: pinned_counter_type_metadata,
        metadata: &PINNED_METHOD,
        source: MethodImplementationSource::Declared,
        availability: crate::runtime::descriptor::method::MethodAvailability::Callable,
        specializations: &PINNED_SPECIALIZATIONS,
        specialization_specs: &[],
        adapters: None,
    };
    let methods = [effective];
    let method = MethodCatalog::new(TypeDescriptor::new(&PINNED_COUNTER_TYPE, &context), &methods)
        .method("pinned_read")
        .expect("method");
    let mut pinned = Box::pin(OtherPinnedCounter {
        value: 41,
        _pin: PhantomPinned,
    });
    let failure = match method.invoke_pinned(PinnedInvocation::<Local, _>::mutable(pinned.as_mut()).arg(Amount(7))) {
        Ok(_) => panic!("receiver with a different type is rejected before dispatch"),
        Err(failure) => failure,
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ReceiverTypeMismatch { .. }
    ));
    let (receiver, arguments) = failure.into_invocation().into_parts();
    assert_eq!(arguments.argument_count(), 1);
    let receiver = receiver
        .downcast_mut::<OtherPinnedCounter>()
        .expect("recovery retains the original pinned receiver type");
    assert_eq!(receiver.as_ref().get_ref().value, 41);
}

#[test]
fn pinned_holder_preserves_trait_object_and_slice_metadata() {
    let trait_object: Pin<Box<dyn PinnedProbe>> = Box::pin(PinnedProbeValue(73, PhantomPinned));
    let (receiver, arguments) = PinnedInvocation::<Local, _>::shared(trait_object.as_ref()).into_parts();
    assert_eq!(arguments.argument_count(), 0);
    let receiver = receiver
        .downcast_ref::<dyn PinnedProbe>()
        .expect("typed erased holder preserves trait-object receiver");
    assert_eq!(receiver.probe(), 73);

    let mut values = [4_u32, 9_u32];
    let pinned_slice = Pin::new(&mut values[..]);
    let (receiver, arguments) = PinnedInvocation::<Local, _>::mutable(pinned_slice).into_parts();
    assert_eq!(arguments.argument_count(), 0);
    {
        let mut receiver = receiver
            .downcast_mut::<[u32]>()
            .expect("typed erased holder preserves slice metadata");
        receiver.as_mut().get_mut()[1] = 17;
    }
    drop(arguments);
    assert_eq!(values, [4, 17]);
}

#[test]
fn preflight_errors_preserve_the_specific_binding_and_receiver_failure() {
    let context = ReflectionContext::empty();
    let methods = [effective()];
    let method = MethodCatalog::new(type_descriptor(&context), &methods)
        .method("add")
        .expect("method");
    let mut counter = Counter(2);

    let failure = match method.invoke(Invocation::<Local>::associated().arg(Amount(1))) {
        Err(failure) => failure,
        Ok(_) => panic!("a receiver is required"),
    };
    assert!(matches!(failure.error().kind(), InvocationErrorKind::ReceiverMissing));
    assert_eq!(failure.recovery().as_invocation().argument_count(), 1);
    drop(failure.into_invocation());

    let failure = match method.invoke(Invocation::<Local>::on_ref(&counter).arg(Amount(1))) {
        Err(failure) => failure,
        Ok(_) => panic!("a mutable receiver cannot be supplied by shared reference"),
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ReceiverModeMismatch
    ));
    drop(failure.into_invocation());

    let failure = match method.invoke(Invocation::<Local>::on_mut(&mut counter).named_arg("missing", Amount(1))) {
        Err(failure) => failure,
        Ok(_) => panic!("unknown named arguments are rejected"),
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::UnknownArgumentName { name } if name.as_ref() == "missing"
    ));
    drop(failure.into_invocation());

    let failure = match method.invoke(Invocation::<Local>::on_mut(&mut counter)) {
        Err(failure) => failure,
        Ok(_) => panic!("all declared parameters must be bound"),
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::MissingArgument { index: 0 }
    ));
    drop(failure.into_invocation());

    let failure = match method.invoke(
        Invocation::<Local>::on_mut(&mut counter)
            .named_arg("amount", Amount(1))
            .named_arg("amount", Amount(2)),
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("a parameter cannot be bound twice"),
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::DuplicateArgumentBinding { name } if name.as_ref() == "amount"
    ));
    drop(failure.into_invocation());

    let failure = match method.invoke(Invocation::<Local>::on_mut(&mut counter).arg(Amount(1)).arg(Amount(2))) {
        Err(failure) => failure,
        Ok(_) => panic!("extra positional bindings are rejected"),
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::TooManyArguments { expected: 1, actual: 2 }
    ));
    drop(failure.into_invocation());

    let failure = match method.invoke(Invocation::<Local>::on_mut(&mut counter).arg_ref(&Amount(1))) {
        Err(failure) => failure,
        Ok(_) => panic!("owned parameters reject borrowed arguments"),
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ArgumentModeMismatch { index: 0 }
    ));
    drop(failure.into_invocation());
    assert_eq!(counter.0, 2, "no rejected invocation reaches its adapter");
}

#[test]
fn argument_binding_reports_unavailable_names_and_concrete_types() {
    static PATTERN_PARAMETER: ParameterMetadata = ParameterMetadata {
        index: 0,
        pattern: ParameterPattern::Destructure,
        passing: PassingMode::Owned,
        expression: &U64_EXPRESSION,
        concrete_type: Some(amount_type_metadata),
    };
    static PATTERN_PARAMETERS: [ParameterMetadata; 1] = [PATTERN_PARAMETER];
    static PATTERN_METHOD: MethodMetadata = MethodMetadata {
        parameters: &PATTERN_PARAMETERS,
        ..METHOD
    };
    static UNKNOWN_TYPE_PARAMETER: ParameterMetadata = ParameterMetadata {
        index: 0,
        pattern: ParameterPattern::Identifier("amount"),
        passing: PassingMode::Owned,
        expression: &U64_EXPRESSION,
        concrete_type: None,
    };
    static UNKNOWN_TYPE_PARAMETERS: [ParameterMetadata; 1] = [UNKNOWN_TYPE_PARAMETER];
    static UNKNOWN_TYPE_METHOD: MethodMetadata = MethodMetadata {
        parameters: &UNKNOWN_TYPE_PARAMETERS,
        ..METHOD
    };
    let context = ReflectionContext::empty();
    let pattern_methods = [effective_for(&PATTERN_METHOD)];
    let pattern_method = MethodCatalog::new(type_descriptor(&context), &pattern_methods)
        .method("add")
        .expect("pattern method");
    let mut counter = Counter(0);
    let failure = match pattern_method.invoke(Invocation::<Local>::on_mut(&mut counter).named_arg("amount", Amount(1)))
    {
        Err(failure) => failure,
        Ok(_) => panic!("destructured parameters do not expose a stable binding name"),
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::NamedArgumentUnavailable { name } if name.as_ref() == "amount"
    ));
    drop(failure.into_invocation());

    let unknown_type_methods = [effective_for(&UNKNOWN_TYPE_METHOD)];
    let unknown_type_method = MethodCatalog::new(type_descriptor(&context), &unknown_type_methods)
        .method("add")
        .expect("unknown type method");
    let failure = match unknown_type_method.invoke(Invocation::<Local>::on_mut(&mut counter).arg(Amount(1))) {
        Err(failure) => failure,
        Ok(_) => panic!("opaque concrete parameter types cannot be checked"),
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ArgumentTypeUnavailable { index: 0 }
    ));
    drop(failure.into_invocation());
}

#[test]
fn output_extractors_retain_values_and_borrow_origins_after_mismatch() {
    use crate::runtime::invoke::BorrowOrigin;
    use crate::runtime::invoke::OutputMismatch;
    use crate::runtime::value::DynamicMut;
    use crate::runtime::value::DynamicRef;

    let mismatch: OutputMismatch<InvocationOutput<'_, Local>> =
        match InvocationOutput::owned(Amount(17)).into_owned::<Wrong>() {
            Err(mismatch) => mismatch,
            Ok(_) => panic!("type mismatch"),
        };
    let owned = match mismatch.into_output().into_owned::<Amount>() {
        Ok(value) => value,
        Err(_) => panic!("the original owned value is retained"),
    };
    assert_eq!(owned.0, 17);

    let value = Amount(23);
    let shared_output: InvocationOutput<'_, Local> = InvocationOutput::Ref {
        value: DynamicRef::<Local>::new(&value),
        origins: vec![BorrowOrigin::Parameter(1)].into_boxed_slice(),
    };
    let mismatch = match shared_output.into_ref::<Wrong>() {
        Err(mismatch) => mismatch,
        Ok(_) => panic!("wrong shared type"),
    };
    match mismatch.into_output() {
        InvocationOutput::Ref { value, origins } => {
            assert_eq!(origins.as_ref(), &[BorrowOrigin::Parameter(1)]);
            assert_eq!(value.downcast::<Amount>().expect("retained reference").0, 23);
        }
        _ => panic!("failed extraction retains its output kind"),
    }

    let mut value = Amount(31);
    let mutable_output: InvocationOutput<'_, Local> = InvocationOutput::Mut {
        value: DynamicMut::<Local>::new(&mut value),
        origin: BorrowOrigin::Receiver,
    };
    let mismatch = match mutable_output.into_mut::<Wrong>() {
        Err(mismatch) => mismatch,
        Ok(_) => panic!("wrong mutable type"),
    };
    match mismatch.into_output() {
        InvocationOutput::Mut { value, origin } => {
            assert_eq!(origin, BorrowOrigin::Receiver);
            value.downcast::<Amount>().expect("retained mutable reference").0 += 1;
        }
        _ => panic!("failed extraction retains its output kind"),
    }
    assert_eq!(value.0, 32);

    let optional = match InvocationOutput::<Local>::optional_shared_with_origins::<Amount>(None, &[])
        .into_optional_ref::<Amount>()
    {
        Ok(optional) => optional,
        Err(_) => panic!("empty optional shared result"),
    };
    assert!(optional.is_none());
    let mismatch = match InvocationOutput::<Local>::unit().into_future() {
        Err(mismatch) => mismatch,
        Ok(_) => panic!("non-future output is preserved"),
    };
    assert!(mismatch.into_output().into_unit().is_ok());
}

#[test]
fn unavailable_method_reasons_are_reported_and_inputs_are_recoverable() {
    let context = ReflectionContext::empty();
    let unavailable = EffectiveMethod {
        specializations: &UNAVAILABLE_SPECIALIZATIONS,
        ..effective()
    };
    let methods = [unavailable];
    let method = MethodCatalog::new(type_descriptor(&context), &methods)
        .method("add")
        .expect("method");
    let mut counter = Counter(7);
    let failure = match method.invoke(Invocation::<Local>::on_mut(&mut counter).arg(Amount(9))) {
        Err(failure) => failure,
        Ok(_) => panic!("unavailable method is rejected before adapter execution"),
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::MethodUnavailable { reasons } if *reasons == UNAVAILABLE_REASONS
    ));
    let recovered = failure.into_invocation();
    assert_eq!(recovered.argument_count(), 1);
    drop(recovered);
    assert_eq!(counter.0, 7);
}

#[test]
fn pinned_thread_safe_dispatch_without_a_mode_adapter_recovers_the_pin() {
    let context = ReflectionContext::empty();
    let effective = EffectiveMethod {
        target: pinned_counter_type_metadata,
        metadata: &PINNED_METHOD,
        source: MethodImplementationSource::Declared,
        availability: crate::runtime::descriptor::method::MethodAvailability::Callable,
        specializations: &PINNED_SPECIALIZATIONS,
        specialization_specs: &[],
        adapters: None,
    };
    let methods = [effective];
    let method = MethodCatalog::new(TypeDescriptor::new(&PINNED_COUNTER_TYPE, &context), &methods)
        .method("pinned_read")
        .expect("method");
    let mut pinned = Box::pin(PinnedCounter {
        value: 13,
        _pin: PhantomPinned,
    });
    let failure = match method.invoke_pinned(PinnedInvocation::<ThreadSafe, _>::mutable(pinned.as_mut()).arg(Amount(2)))
    {
        Err(failure) => failure,
        Ok(_) => panic!("thread-safe dispatch requires a thread-safe pinned adapter"),
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ReceiverAdapterUnavailable
    ));
    let recovered = failure.into_invocation();
    assert_eq!(recovered.argument_count(), 1);
    assert!(recovered.receiver_is_mutable());
    assert_eq!(recovered.receiver_type_id(), std::any::TypeId::of::<PinnedCounter>());
    drop(recovered);
    assert_eq!(pinned.as_ref().get_ref().value, 13);
}

#[test]
fn optional_and_slice_return_extractors_restore_origins_and_values() {
    use crate::runtime::invoke::BorrowOrigin;
    use crate::runtime::invoke::InvocationOutput;

    let first = Amount(21);
    let second = Amount(34);
    let optional = InvocationOutput::<Local>::optional_shared_with_origins(
        Some(&first),
        &[BorrowOrigin::Parameter(0), BorrowOrigin::Receiver],
    );
    let optional_value = match optional.into_optional_ref::<Amount>() {
        Ok(Some(value)) => value,
        Ok(None) => panic!("the method returned Some"),
        Err(_) => panic!("optional shared result has the requested type"),
    };
    assert_eq!(optional_value.0, 21);

    let optional = InvocationOutput::<Local>::optional_shared_with_origins(Some(&first), &[BorrowOrigin::Parameter(2)]);
    let mismatch = match optional.into_optional_ref::<Wrong>() {
        Err(mismatch) => mismatch,
        Ok(_) => panic!("optional extraction rejects the wrong type"),
    };
    match mismatch.into_output() {
        InvocationOutput::OptionalRef {
            value: Some(value),
            origins,
        } => {
            assert_eq!(origins.as_ref(), &[BorrowOrigin::Parameter(2)]);
            assert_eq!(value.downcast::<Amount>().expect("value is retained").0, 21);
        }
        _ => panic!("failed extraction retains the optional result"),
    }

    let slice: InvocationOutput<'_, Local> = InvocationOutput::RefSlice {
        values: vec![
            crate::runtime::value::DynamicRef::<Local>::new(&first),
            crate::runtime::value::DynamicRef::<Local>::new(&second),
        ]
        .into_boxed_slice(),
        origins: vec![BorrowOrigin::Receiver].into_boxed_slice(),
    };
    let references = match slice.into_ref_slice::<Amount>() {
        Ok(references) => references,
        Err(_) => panic!("all slice elements have the requested type"),
    };
    assert_eq!(references.iter().map(|value| value.0).collect::<Vec<_>>(), [21, 34]);

    let slice: InvocationOutput<'_, Local> = InvocationOutput::RefSlice {
        values: vec![
            crate::runtime::value::DynamicRef::<Local>::new(&first),
            crate::runtime::value::DynamicRef::<Local>::new(&second),
        ]
        .into_boxed_slice(),
        origins: vec![BorrowOrigin::Parameter(1)].into_boxed_slice(),
    };
    let mismatch = match slice.into_ref_slice::<Wrong>() {
        Err(mismatch) => mismatch,
        Ok(_) => panic!("slice extraction rejects the wrong element type"),
    };
    match mismatch.into_output() {
        InvocationOutput::RefSlice { values, origins } => {
            assert_eq!(origins.as_ref(), &[BorrowOrigin::Parameter(1)]);
            assert_eq!(values.len(), 2);
            assert_eq!(values[0].clone().downcast::<Amount>().expect("first retained").0, 21);
            assert_eq!(values[1].clone().downcast::<Amount>().expect("second retained").0, 34);
        }
        _ => panic!("failed extraction retains the complete slice result"),
    }
}

#[test]
fn reflected_future_extracts_and_polls_local_and_thread_safe_results() {
    use std::future::Future;
    use std::task::Context;
    use std::task::Waker;

    fn poll_ready<F: Future + Unpin>(future: &mut F) -> F::Output {
        let mut context = Context::from_waker(Waker::noop());
        match std::pin::Pin::new(future).poll(&mut context) {
            std::task::Poll::Ready(output) => output,
            std::task::Poll::Pending => panic!("ready test future does not suspend"),
        }
    }

    let local_output = crate::runtime::invoke::InvocationOutput::<Local>::future(async {
        let marker = std::rc::Rc::new(());
        assert_eq!(std::rc::Rc::strong_count(&marker), 1);
        crate::runtime::invoke::InvocationOutput::owned(Amount(55))
    });
    let mut local_future = match local_output.into_future() {
        Ok(future) => future,
        Err(_) => panic!("future output can be extracted"),
    };
    let local_result = match poll_ready(&mut local_future).into_owned::<Amount>() {
        Ok(value) => value,
        Err(_) => panic!("future returns an owned Amount"),
    };
    assert_eq!(local_result.0, 55);

    let thread_safe_output = crate::runtime::invoke::InvocationOutput::<ThreadSafe>::future(async {
        crate::runtime::invoke::InvocationOutput::owned(Amount(89))
    });
    let mut thread_safe_future = match thread_safe_output.into_future() {
        Ok(future) => future,
        Err(_) => panic!("Send future output can be extracted in thread-safe mode"),
    };
    let thread_safe_result = match poll_ready(&mut thread_safe_future).into_owned::<Amount>() {
        Ok(value) => value,
        Err(_) => panic!("future returns an owned Amount"),
    };
    assert_eq!(thread_safe_result.0, 89);
}
