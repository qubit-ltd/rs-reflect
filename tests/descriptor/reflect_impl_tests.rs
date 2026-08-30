// qubit-style: allow explicit-imports
//! Integration coverage for implementation registration expansion.
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::OnceLock;
use std::task::Context;
use std::task::Poll;

use qubit_reflect as reflect;
use reflect::Reflect;
use reflect::descriptor::MethodQualifier;
use reflect::descriptor::StructKind;
use reflect::invoke::Invocation;
use reflect::invoke::InvocationArg;
use reflect::invoke::InvocationOutput;
use reflect::reflect;
use reflect::reflect_impl;
use reflect::registry::ReflectRegistry;
use reflect::value::DynamicOwned;

struct Sample;

impl Reflect for Sample {
    fn type_descriptor() -> &'static reflect::TypeDescriptor {
        static DESCRIPTOR: OnceLock<reflect::TypeDescriptor> = OnceLock::new();
        DESCRIPTOR
            .get_or_init(|| reflect::__private::descriptor::struct_type::<Sample>("Sample", StructKind::Named, &[]))
    }
}

trait External {
    fn value(&self) -> u8;
}

#[reflect]
trait Reflected {
    fn reflected_value(&self) -> u8;

    fn default_value(&self) -> u8 {
        13
    }
}

#[reflect_impl]
impl Reflected for Sample {
    fn reflected_value(&self) -> u8 {
        11
    }
}

#[reflect_impl(external_trait_id = "example.external")]
impl External for Sample {
    fn value(&self) -> u8 {
        7
    }
}

#[reflect_impl]
impl Sample {
    fn private_method(&self) -> u8 {
        3
    }

    pub fn public_method() -> u8 {
        5
    }

    fn reflected_associated() -> u8 {
        17
    }

    fn reflected_owned_argument(value: u8) -> u8 {
        value + 1
    }

    fn reflected_shared_argument(value: &u8) -> u8 {
        value + 2
    }

    fn reflected_mutable_argument(value: &mut u8) -> u8 {
        *value += 2;
        *value
    }

    fn reflected_two_owned_arguments(first: u8, second: u8) -> u8 {
        first + second
    }

    #[reflect(specialize(T = u8))]
    #[allow(dead_code)]
    fn reflected_generic<T>(value: T) -> T {
        value
    }

    async fn reflected_async_argument(value: u8) -> u8 {
        value + 3
    }

    #[reflect(thread_safe)]
    fn reflected_thread_safe_argument(value: u8) -> u8 {
        value + 4
    }

    #[reflect(catch_unwind)]
    fn reflected_panicking() -> u8 {
        panic!("caught panic")
    }

    fn reflected_shared(&self) -> u8 {
        19
    }

    fn reflected_borrowed_path(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed("borrowed")
    }

    extern "C" fn reflected_c_abi() -> u8 {
        29
    }
}

struct Counter(u8);

impl Reflect for Counter {
    fn type_descriptor() -> &'static reflect::TypeDescriptor {
        static DESCRIPTOR: OnceLock<reflect::TypeDescriptor> = OnceLock::new();
        DESCRIPTOR
            .get_or_init(|| reflect::__private::descriptor::struct_type::<Counter>("Counter", StructKind::Named, &[]))
    }
}

#[reflect_impl]
impl Counter {
    fn reflected_borrowed(&self) -> &u8 {
        &self.0
    }

    fn reflected_borrowed_mut(&mut self) -> &mut u8 {
        &mut self.0
    }

    fn reflected_mut(&mut self) -> u8 {
        self.0 += 1;
        self.0
    }

    fn reflected_owned(self) -> u8 {
        self.0
    }
}

struct SmartReceiver(u8);

impl Reflect for SmartReceiver {
    fn type_descriptor() -> &'static reflect::TypeDescriptor {
        static DESCRIPTOR: OnceLock<reflect::TypeDescriptor> = OnceLock::new();
        DESCRIPTOR.get_or_init(|| {
            reflect::__private::descriptor::struct_type::<SmartReceiver>("SmartReceiver", StructKind::Named, &[])
        })
    }
}

#[reflect_impl]
impl SmartReceiver {
    #[allow(clippy::boxed_local)]
    fn boxed(self: Box<Self>) -> u8 {
        self.0
    }

    fn rc(self: Rc<Self>) -> u8 {
        self.0
    }

    fn arc(self: Arc<Self>) -> u8 {
        self.0
    }

    fn pinned_box(self: Pin<Box<Self>>) -> u8 {
        self.0
    }

    #[allow(dead_code)]
    fn pinned_ref(self: Pin<&Self>) -> u8 {
        self.0
    }
}

#[test]
fn test_reflect_impl_generates_callable_adapter_for_shared_receiver() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_shared",
    ) else {
        panic!("generated shared method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("safe shared method needs adapter");
    let sample = Sample;
    let output = adapter
        .invoke_local(Invocation::borrowed(
            reflect::value::DynamicRef::<reflect::value::Local>::new(&sample),
            [],
        ))
        .expect("local adapter must be present")
        .expect("validated invocation must call method");
    let InvocationOutput::Owned(value) = output else {
        panic!("shared method must return an owned value");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
        panic!("generated value must retain type");
    };
    assert_eq!(value, 19);
}

#[test]
fn test_reflect_impl_registers_explicit_generic_method_specialization() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_generic",
    ) else {
        panic!("registered generic specialization must be discoverable");
    };
    assert_eq!(instance.arguments().len(), 1);
    assert!(instance.adapter().is_none());
    assert_eq!(
        instance.unavailable_reasons(),
        &[reflect::descriptor::InvocationUnavailableReason::UnsupportedSpecialization],
    );
}

#[test]
fn test_reflect_impl_generates_callable_adapter_for_mutable_receiver() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Counter::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) =
        reflect::descriptor::ImplDescriptor::lookup_method(implementations, MethodQualifier::Inherent, "reflected_mut")
    else {
        panic!("generated mutable method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("safe mutable method needs adapter");
    let mut counter = Counter(3);
    let value = {
        let output = adapter
            .invoke_local(Invocation::borrowed_mut(
                reflect::value::DynamicMut::<reflect::value::Local>::new(&mut counter),
                [],
            ))
            .expect("local adapter must be present")
            .expect("validated invocation must call method");
        let InvocationOutput::Owned(value) = output else {
            panic!("mutable method must return an owned value");
        };
        let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
            panic!("generated value must retain type");
        };
        value
    };
    assert_eq!(value, 4);
    assert_eq!(counter.0, 4);
}

#[test]
fn test_reflect_impl_generates_callable_adapter_for_owned_receiver() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Counter::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_owned",
    ) else {
        panic!("generated owned method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("safe owned method needs adapter");
    let output = adapter
        .invoke_local(Invocation::owned(
            DynamicOwned::<reflect::value::Local>::new(Counter(23)),
            [],
        ))
        .expect("local adapter must be present")
        .expect("validated invocation must call method");
    let InvocationOutput::Owned(value) = output else {
        panic!("owned method must return an owned value");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
        panic!("generated value must retain type");
    };
    assert_eq!(value, 23);
}

#[test]
fn test_reflect_impl_generates_callable_adapters_for_owned_smart_receivers() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(SmartReceiver::type_descriptor().type_id());
    let cases: [(&str, DynamicOwned<reflect::value::Local>); 4] = [
        (
            "boxed",
            DynamicOwned::<reflect::value::Local>::new(Box::new(SmartReceiver(11))),
        ),
        (
            "rc",
            DynamicOwned::<reflect::value::Local>::new(Rc::new(SmartReceiver(12))),
        ),
        (
            "arc",
            DynamicOwned::<reflect::value::Local>::new(Arc::new(SmartReceiver(13))),
        ),
        (
            "pinned_box",
            DynamicOwned::<reflect::value::Local>::new(Box::pin(SmartReceiver(14))),
        ),
    ];

    for (expected, (name, receiver)) in cases.into_iter().enumerate() {
        let reflect::descriptor::MethodLookup::Unique(instance) =
            reflect::descriptor::ImplDescriptor::lookup_method(implementations, MethodQualifier::Inherent, name)
        else {
            panic!("smart receiver method must be discoverable");
        };
        let adapter = instance.adapter().expect("supported smart receiver needs an adapter");
        let output = adapter
            .invoke_local(Invocation::owned(receiver, []))
            .expect("local adapter must be present")
            .expect("validated invocation must call method");
        let InvocationOutput::Owned(value) = output else {
            panic!("smart receiver method must return an owned value");
        };
        let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
            panic!("generated value must retain type");
        };
        assert_eq!(value, (expected as u8) + 11);
    }
}

#[test]
fn test_reflect_impl_reports_pinned_borrow_receiver_without_adapter() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(SmartReceiver::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) =
        reflect::descriptor::ImplDescriptor::lookup_method(implementations, MethodQualifier::Inherent, "pinned_ref")
    else {
        panic!("pinned receiver method must be discoverable");
    };
    assert!(instance.adapter().is_none());
    assert_eq!(
        instance.unavailable_reasons(),
        &[reflect::descriptor::InvocationUnavailableReason::UnsupportedReceiver],
    );
}

#[test]
fn test_reflect_impl_generates_callable_adapter_for_shared_borrowed_output() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Counter::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_borrowed",
    ) else {
        panic!("generated borrowed-output method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("shared borrowed output needs adapter");
    let counter = Counter(23);
    let output = adapter
        .invoke_local(Invocation::borrowed(
            reflect::value::DynamicRef::<reflect::value::Local>::new(&counter),
            [],
        ))
        .expect("local adapter must be present")
        .expect("validated invocation must call method");
    let InvocationOutput::Ref { value, origins } = output else {
        panic!("shared borrowed output must be represented as a dynamic reference");
    };
    assert_eq!(value.downcast_ref::<u8>(), Some(&23));
    assert_eq!(origins.as_ref(), &[reflect::invoke::BorrowOrigin::Receiver]);
}

#[test]
fn test_reflect_impl_generates_callable_adapter_for_mutable_borrowed_output() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Counter::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_borrowed_mut",
    ) else {
        panic!("generated mutable borrowed-output method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("mutable borrowed output needs adapter");
    let mut counter = Counter(23);
    {
        let output = adapter
            .invoke_local(Invocation::borrowed_mut(
                reflect::value::DynamicMut::<reflect::value::Local>::new(&mut counter),
                [],
            ))
            .expect("local adapter must be present")
            .expect("validated invocation must call method");
        let InvocationOutput::Mut { mut value, origin } = output else {
            panic!("mutable borrowed output must be represented as a dynamic mutable reference");
        };
        assert_eq!(origin, reflect::invoke::BorrowOrigin::Receiver);
        *value
            .downcast_mut::<u8>()
            .expect("mutable output must retain the exact field type") = 42;
    }
    assert_eq!(counter.0, 42);
}

#[test]
fn test_reflect_impl_does_not_generate_adapter_for_non_rust_abi() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_c_abi",
    ) else {
        panic!("non-Rust ABI method instance must remain describable");
    };
    assert!(instance.adapter().is_none());
    assert!(
        instance
            .unavailable_reasons()
            .contains(&reflect::descriptor::InvocationUnavailableReason::UnsupportedAbi)
    );
}

#[test]
fn test_reflect_impl_registers_inherent_and_external_fragments() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    assert!(registry.get(Sample::type_descriptor().type_id()).is_none());
    assert_eq!(Sample::public_method(), 5);
    assert_eq!(Sample.value(), 7);
    assert_eq!(Sample.reflected_value(), 11);
    assert_eq!(Sample.default_value(), 13);
    assert_eq!(Sample.private_method(), 3);
    assert_eq!(Sample::reflected_c_abi(), 29);
    assert_eq!(Sample.reflected_borrowed_path(), "borrowed");
}

#[test]
fn test_reflect_impl_generates_callable_adapter_for_safe_associated_function() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_associated",
    ) else {
        panic!("generated associated method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("safe associated method needs adapter");
    let output = adapter
        .invoke_local(Invocation::associated([]))
        .expect("local adapter must be present")
        .expect("validated invocation must call method");
    let InvocationOutput::Owned(value) = output else {
        panic!("associated method must return an owned value");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
        panic!("generated value must retain type");
    };
    assert_eq!(value, 17);
}

#[test]
fn test_reflect_impl_generates_callable_adapter_for_owned_argument() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_owned_argument",
    ) else {
        panic!("generated associated method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("owned argument method needs adapter");
    let output = adapter
        .invoke_local(Invocation::associated([InvocationArg::Owned(DynamicOwned::<
            reflect::value::Local,
        >::new(41_u8))]))
        .expect("local adapter must be present")
        .expect("validated invocation must call method");
    let InvocationOutput::Owned(value) = output else {
        panic!("associated method must return an owned value");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
        panic!("generated value must retain type");
    };
    assert_eq!(value, 42);
}

#[test]
fn test_reflect_impl_generates_callable_adapter_for_shared_argument() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_shared_argument",
    ) else {
        panic!("generated associated method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("shared argument method needs adapter");
    let input = 40_u8;
    let output = adapter
        .invoke_local(Invocation::associated([InvocationArg::Ref(
            reflect::value::DynamicRef::<reflect::value::Local>::new(&input),
        )]))
        .expect("local adapter must be present")
        .expect("validated invocation must call method");
    let InvocationOutput::Owned(value) = output else {
        panic!("associated method must return an owned value");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
        panic!("generated value must retain type");
    };
    assert_eq!(value, 42);
}

#[test]
fn test_reflect_impl_generates_callable_adapter_for_mutable_argument() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_mutable_argument",
    ) else {
        panic!("generated associated method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("mutable argument method needs adapter");
    let mut input = 40_u8;
    let value = {
        let output = adapter
            .invoke_local(Invocation::associated([InvocationArg::Mut(
                reflect::value::DynamicMut::<reflect::value::Local>::new(&mut input),
            )]))
            .expect("local adapter must be present")
            .expect("validated invocation must call method");
        let InvocationOutput::Owned(value) = output else {
            panic!("associated method must return an owned value");
        };
        let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
            panic!("generated value must retain type");
        };
        value
    };
    assert_eq!(value, 42);
    assert_eq!(input, 42);
}

#[test]
fn test_reflect_impl_preserves_all_owned_arguments_after_validation_failure() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_two_owned_arguments",
    ) else {
        panic!("two-argument method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("safe owned-argument method needs adapter");
    let result = adapter
        .invoke_local(Invocation::associated([
            InvocationArg::Owned(DynamicOwned::<reflect::value::Local>::new(7_u8)),
            InvocationArg::Owned(DynamicOwned::<reflect::value::Local>::new(String::from("wrong"))),
        ]))
        .expect("local adapter must be present");
    let Err(failure) = result else {
        panic!("the second argument must fail exact type validation");
    };
    assert!(matches!(
        failure.error.kind(),
        reflect::invoke::InvocationErrorKind::ArgumentTypeMismatch { index: 1, .. }
    ));
    let (_, arguments) = failure.recovery.into_parts();
    let mut arguments = arguments.into_vec().into_iter();
    let (Some(InvocationArg::Owned(first)), Some(InvocationArg::Owned(second)), None) =
        (arguments.next(), arguments.next(), arguments.next())
    else {
        panic!("validation failure must preserve both owned arguments in source order");
    };
    let Ok(first) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(first) else {
        panic!("first value must remain intact");
    };
    assert_eq!(first, 7);
    assert!(DynamicOwned::<reflect::value::Local>::downcast::<String>(second).is_ok());
}

#[test]
fn test_reflect_impl_generates_callable_adapter_for_async_method() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_async_argument",
    ) else {
        panic!("generated async method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("safe async method needs adapter");
    let output = adapter
        .invoke_local(Invocation::associated([InvocationArg::Owned(DynamicOwned::<
            reflect::value::Local,
        >::new(39_u8))]))
        .expect("local adapter must be present")
        .expect("validated invocation must start method");
    let InvocationOutput::Future(mut future) = output else {
        panic!("async method must return a reflected future");
    };
    let Poll::Ready(InvocationOutput::Owned(value)) = poll_once(&mut future) else {
        panic!("simple async method must complete when polled");
    };
    let Ok(value) = DynamicOwned::<reflect::value::Local>::downcast::<u8>(value) else {
        panic!("async output must retain its owned value");
    };
    assert_eq!(value, 42);
}

#[test]
fn test_reflect_impl_generates_explicit_thread_safe_adapter() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_thread_safe_argument",
    ) else {
        panic!("generated thread-safe method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("explicit thread-safe method needs adapter");
    assert!(adapter.invoke_local(Invocation::associated([])).is_none());
    let output = adapter
        .invoke_thread_safe(Invocation::associated([InvocationArg::Owned(
            reflect::value::DynamicOwned::<reflect::value::ThreadSafe>::new(38_u8),
        )]))
        .expect("thread-safe adapter must be present")
        .expect("validated invocation must call method");
    let InvocationOutput::Owned(value) = output else {
        panic!("thread-safe method must return an owned output");
    };
    let Ok(value) = reflect::value::DynamicOwned::<reflect::value::ThreadSafe>::downcast::<u8>(value) else {
        panic!("thread-safe output must retain its exact type");
    };
    assert_eq!(value, 42);
}

#[test]
fn test_reflect_impl_generates_explicit_catching_adapter() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let reflect::descriptor::MethodLookup::Unique(instance) = reflect::descriptor::ImplDescriptor::lookup_method(
        implementations,
        MethodQualifier::Inherent,
        "reflected_panicking",
    ) else {
        panic!("generated catching method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("catching method needs adapter");
    let caught = match adapter
        .invoke_catching_local(Invocation::associated([]))
        .expect("catching adapter must be present")
        .expect("validated invocation must begin")
    {
        Ok(_) => panic!("the panic must be reported separately from validation failure"),
        Err(caught) => caught,
    };
    assert_eq!(caught.payload().downcast_ref::<&str>(), Some(&"caught panic"));
}

fn poll_once<F: Future + Unpin>(future: &mut F) -> Poll<F::Output> {
    let waker = std::task::Waker::noop();
    let mut context = Context::from_waker(waker);
    Pin::new(future).poll(&mut context)
}
