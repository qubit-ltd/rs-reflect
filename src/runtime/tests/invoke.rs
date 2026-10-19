use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::method::{
    EffectiveMethod, FunctionAbi, GenericDefinitionMetadata, InvocationAdapters, MethodCatalog,
    MethodImplementationSource, MethodMetadata, MethodNamespaceMetadata, MethodQualifiers,
    MethodVisibility, ParameterMetadata, ParameterPattern, PassingMode, ReceiverDescriptor,
    ReceiverKind, ResolvedMethodArguments, ResolvedMethodSpecialization, ReturnKind,
    ReturnMetadata, TypeMetadata,
};
use crate::runtime::descriptor::{Reflect, TypeDescriptor, TypeRegistration};
use crate::runtime::expression::ExpressionData;
use crate::runtime::identity::GenericArgumentIdentity;
use crate::runtime::invoke::{
    Invocation, InvocationAdapter, InvocationAvailability, InvocationErrorKind, InvocationOutput,
    InvocationReceiver, Local, PinnedInvocation, PinnedInvocationAdapter, PinnedReceiver,
    ThreadSafe, ValidatedInvocation,
};
use std::marker::PhantomPinned;
use std::pin::Pin;

struct Counter(u64);
impl Reflect for Counter {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        static REGISTRATION: TypeRegistration<Counter> = TypeRegistration::new();
        &REGISTRATION
    }
}
struct Amount(u64);
struct Wrong(String);
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
    fn invoke<'call>(
        &self,
        invocation: ValidatedInvocation<'call, Local>,
    ) -> InvocationOutput<'call, Local> {
        let (receiver, arguments) = invocation.into_parts();
        let Some(InvocationReceiver::Mut(mut receiver)) = receiver else {
            panic!("validated mutable receiver")
        };
        let amount = match arguments
            .into_vec()
            .into_iter()
            .next()
            .expect("one argument")
        {
            crate::runtime::invoke::InvocationValue::Owned(value) => {
                match value.downcast::<Amount>() {
                    Ok(value) => value,
                    Err(_) => panic!("validated Amount"),
                }
            }
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
    fn invoke<'call>(
        &self,
        invocation: ValidatedInvocation<'call, ThreadSafe>,
    ) -> InvocationOutput<'call, ThreadSafe> {
        let (receiver, arguments) = invocation.into_parts();
        let Some(InvocationReceiver::Mut(mut receiver)) = receiver else {
            panic!("validated mutable receiver")
        };
        let amount = match arguments
            .into_vec()
            .into_iter()
            .next()
            .expect("one argument")
        {
            crate::runtime::invoke::InvocationValue::Owned(value) => {
                match value.downcast::<Amount>() {
                    Ok(value) => value,
                    Err(_) => panic!("validated Amount"),
                }
            }
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
    fn invoke<'call>(
        &self,
        invocation: ValidatedInvocation<'call, Local>,
    ) -> InvocationOutput<'call, Local> {
        let (receiver, arguments) = invocation.into_parts();
        let Some(InvocationReceiver::Mut(mut receiver)) = receiver else {
            panic!("validated mutable receiver")
        };
        let mut amounts = arguments
            .into_vec()
            .into_iter()
            .map(|argument| match argument {
                crate::runtime::invoke::InvocationValue::Owned(value) => {
                    match value.downcast::<Amount>() {
                        Ok(value) => value.0,
                        Err(_) => panic!("validated Amount"),
                    }
                }
                _ => panic!("validated owned argument"),
            });
        let sum =
            amounts.next().expect("first argument") + amounts.next().expect("second argument");
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
static TWO_ARGUMENT_SPECIALIZATIONS: [ResolvedMethodSpecialization; 1] =
    [ResolvedMethodSpecialization {
        arguments: ResolvedMethodArguments::Static(&EMPTY_ARGUMENTS),
        availability: InvocationAvailability::Callable,
        adapters: ADD_BOTH_ADAPTERS,
    }];
static EMPTY_ARGUMENTS: [GenericArgumentIdentity; 0] = [];
static SPECIALIZATION_SET: [ResolvedMethodSpecialization; 1] = [ResolvedMethodSpecialization {
    arguments: ResolvedMethodArguments::Static(&EMPTY_ARGUMENTS),
    availability: InvocationAvailability::Callable,
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
    fn invoke<'call>(
        &self,
        _invocation: ValidatedInvocation<'call, Local>,
    ) -> InvocationOutput<'call, Local> {
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
static NO_CATCH_SPECIALIZATIONS: [ResolvedMethodSpecialization; 1] =
    [ResolvedMethodSpecialization {
        arguments: ResolvedMethodArguments::Static(&EMPTY_ARGUMENTS),
        availability: InvocationAvailability::Callable,
        adapters: NO_CATCH_ADAPTERS,
    }];
static PANIC_SPECIALIZATIONS: [ResolvedMethodSpecialization; 1] = [ResolvedMethodSpecialization {
    arguments: ResolvedMethodArguments::Static(&EMPTY_ARGUMENTS),
    availability: InvocationAvailability::Callable,
    adapters: PANICKING_ADAPTERS,
}];
static BUILT_PANIC_SPECIALIZATIONS: [ResolvedMethodSpecialization; 1] =
    [ResolvedMethodSpecialization {
        arguments: ResolvedMethodArguments::Static(&EMPTY_ARGUMENTS),
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

    assert!(catalog
        .lookup_method("absent")
        .expect("context")
        .is_missing());
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
    let failure = match method
        .invoke(Invocation::<Local>::on_mut(&mut counter).arg(Wrong(String::from("wrong"))))
    {
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
        failure.error().kind(),
        InvocationErrorKind::UnsafeMethod
    ));
    let recovered = failure.into_invocation();
    assert_eq!(recovered.argument_count(), 1);
    drop(recovered);
    assert_eq!(counter.0, 10);

    let mut counter = Counter(10);
    {
        let output = unsafe {
            method.invoke_unchecked(Invocation::<Local>::on_mut(&mut counter).arg(Amount(3)))
        };
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
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::VisibilityDenied
    ));
    drop(failure.into_invocation());
    {
        let output = unsafe {
            method.invoke_unchecked(Invocation::<Local>::on_mut(&mut counter).arg(Amount(2)))
        };
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

    assert!(
        outcome.is_err(),
        "invoke does not intercept the method panic"
    );
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
    let failure =
        match method.invoke_catching(Invocation::<Local>::on_mut(&mut counter).arg(Amount(1))) {
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
    let failure = match method
        .invoke_catching(Invocation::<Local>::on_mut(&mut counter).arg(Wrong(String::from("x"))))
    {
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
    let failure =
        match method.invoke_catching(Invocation::<Local>::on_mut(&mut counter).arg(Amount(4))) {
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
    let failure =
        match method.invoke_catching(Invocation::<Local>::on_mut(&mut counter).arg(Amount(1))) {
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
    assert!(
        ordinary.is_err(),
        "ordinary invoke must keep propagating panics"
    );

    let mut counter = Counter(11);
    let caught =
        match method.invoke_catching(Invocation::<Local>::on_mut(&mut counter).arg(Amount(1))) {
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
    let method = MethodCatalog::new(
        TypeDescriptor::new(&PINNED_COUNTER_TYPE, &context),
        &methods,
    )
    .method("pinned_read")
    .expect("method");
    let mut pinned = Box::pin(PinnedCounter {
        value: 19,
        _pin: PhantomPinned,
    });
    {
        let output = method
            .invoke_pinned(PinnedInvocation::<Local, _>::mutable(pinned.as_mut()).arg(Amount(5)));
        match output {
            Ok(output) => match output.into_unit() {
                Ok(()) => {}
                Err(_) => panic!("pinned adapter returns unit"),
            },
            Err(_) => panic!("valid pinned invocation succeeds"),
        }
    }
    {
        let failure = match method.invoke_pinned(
            PinnedInvocation::<Local, _>::mutable(pinned.as_mut())
                .arg(Wrong(String::from("wrong"))),
        ) {
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
        assert_eq!(
            recovered.receiver_type_id(),
            std::any::TypeId::of::<PinnedCounter>()
        );
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
    let method = MethodCatalog::new(
        TypeDescriptor::new(&PINNED_COUNTER_TYPE, &context),
        &methods,
    )
    .method("pinned_read")
    .expect("method");
    let mut pinned = Box::pin(OtherPinnedCounter {
        value: 41,
        _pin: PhantomPinned,
    });
    let failure = match method
        .invoke_pinned(PinnedInvocation::<Local, _>::mutable(pinned.as_mut()).arg(Amount(7)))
    {
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
    let (receiver, arguments) =
        PinnedInvocation::<Local, _>::shared(trait_object.as_ref()).into_parts();
    assert_eq!(arguments.argument_count(), 0);
    let receiver = receiver
        .downcast_ref::<dyn PinnedProbe>()
        .expect("typed erased holder preserves trait-object receiver");
    assert_eq!(receiver.probe(), 73);

    let mut values = [4_u32, 9_u32];
    let pinned_slice = Pin::new(&mut values[..]);
    let (receiver, arguments) = PinnedInvocation::<Local, _>::mutable(pinned_slice).into_parts();
    assert_eq!(arguments.argument_count(), 0);
    let mut receiver = receiver
        .downcast_mut::<[u32]>()
        .expect("typed erased holder preserves slice metadata");
    receiver.as_mut().get_mut()[1] = 17;
    drop(receiver);
    drop(arguments);
    assert_eq!(values, [4, 17]);
}
