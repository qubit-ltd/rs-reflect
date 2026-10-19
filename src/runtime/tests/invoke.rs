use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::method::{
    EffectiveMethod, FunctionAbi, GenericDefinitionMetadata, InvocationAdapters, MethodCatalog,
    MethodImplementationSource, MethodMetadata, MethodNamespaceMetadata, MethodQualifiers,
    MethodVisibility, ParameterMetadata, ParameterPattern, PassingMode, ReceiverDescriptor,
    ReceiverKind, ResolvedMethodSpecialization, ReturnKind, ReturnMetadata, TypeMetadata,
};
use crate::runtime::descriptor::{Reflect, TypeDescriptor, TypeRegistration};
use crate::runtime::expression::ExpressionData;
use crate::runtime::identity::GenericArgumentIdentity;
use crate::runtime::invoke::{
    Invocation, InvocationAdapter, InvocationAvailability, InvocationErrorKind, InvocationOutput,
    InvocationReceiver, Local, ThreadSafe, ValidatedInvocation,
};

struct Counter(u64);
impl Reflect for Counter {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        static REGISTRATION: TypeRegistration<Counter> = TypeRegistration::new();
        &REGISTRATION
    }
}
struct Amount(u64);
struct Wrong(String);
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

static COUNTER_TYPE: TypeMetadata = TypeMetadata::new::<Counter>();
static AMOUNT_TYPE: TypeMetadata = TypeMetadata::new::<Amount>();
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
    type_metadata: &COUNTER_TYPE,
};
static VALUE_PARAMETER: ParameterMetadata = ParameterMetadata {
    index: 0,
    pattern: ParameterPattern::Identifier("amount"),
    passing: PassingMode::Owned,
    expression: &U64_EXPRESSION,
    concrete_type: Some(&AMOUNT_TYPE),
};
static FIRST_PARAMETER: ParameterMetadata = ParameterMetadata {
    index: 0,
    pattern: ParameterPattern::Identifier("first"),
    passing: PassingMode::Owned,
    expression: &U64_EXPRESSION,
    concrete_type: Some(&AMOUNT_TYPE),
};
static SECOND_PARAMETER: ParameterMetadata = ParameterMetadata {
    index: 1,
    pattern: ParameterPattern::Identifier("second"),
    passing: PassingMode::Owned,
    expression: &U64_EXPRESSION,
    concrete_type: Some(&AMOUNT_TYPE),
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

static ADD_ADAPTER: AddAdapter = AddAdapter;
static ADD_THREAD_SAFE_ADAPTER: AddThreadSafeAdapter = AddThreadSafeAdapter;
static ADAPTERS: InvocationAdapters = InvocationAdapters {
    local: Some(&ADD_ADAPTER),
    thread_safe: Some(&ADD_THREAD_SAFE_ADAPTER),
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
};
static TWO_ARGUMENT_SPECIALIZATIONS: [ResolvedMethodSpecialization; 1] =
    [ResolvedMethodSpecialization {
        arguments: &EMPTY_ARGUMENTS,
        availability: InvocationAvailability::Callable,
        adapters: &ADD_BOTH_ADAPTERS,
    }];
static EMPTY_ARGUMENTS: [GenericArgumentIdentity; 0] = [];
static SPECIALIZATION_SET: [ResolvedMethodSpecialization; 1] = [ResolvedMethodSpecialization {
    arguments: &EMPTY_ARGUMENTS,
    availability: InvocationAvailability::Callable,
    adapters: &ADAPTERS,
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
static PANICKING_ADAPTERS: InvocationAdapters = InvocationAdapters {
    local: Some(&PANICKING_ADAPTER),
    thread_safe: None,
};
static PANIC_SPECIALIZATIONS: [ResolvedMethodSpecialization; 1] = [ResolvedMethodSpecialization {
    arguments: &EMPTY_ARGUMENTS,
    availability: InvocationAvailability::Callable,
    adapters: &PANICKING_ADAPTERS,
}];

fn effective() -> EffectiveMethod {
    effective_for(&METHOD)
}

fn effective_for(metadata: &'static MethodMetadata) -> EffectiveMethod {
    EffectiveMethod {
        target: &COUNTER_TYPE,
        metadata,
        source: MethodImplementationSource::Declared,
        availability: crate::runtime::descriptor::method::MethodAvailability::Callable,
        specializations: &SPECIALIZATION_SET,
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
        target: &COUNTER_TYPE,
        metadata: &TWO_ARGUMENT_METHOD,
        source: MethodImplementationSource::Declared,
        availability: crate::runtime::descriptor::method::MethodAvailability::Callable,
        specializations: &TWO_ARGUMENT_SPECIALIZATIONS,
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

    assert!(
        catalog
            .lookup_method("absent")
            .expect("context")
            .is_missing()
    );
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
}

#[test]
fn ordinary_invoke_leaves_user_panics_unwinding() {
    let context = ReflectionContext::empty();
    let method = EffectiveMethod {
        target: &COUNTER_TYPE,
        metadata: &PANIC_METHOD,
        source: MethodImplementationSource::Declared,
        availability: crate::runtime::descriptor::method::MethodAvailability::Callable,
        specializations: &PANIC_SPECIALIZATIONS,
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
