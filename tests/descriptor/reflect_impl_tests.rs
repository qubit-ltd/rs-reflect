//! Integration coverage for implementation registration expansion.

use std::sync::OnceLock;

use qubit_reflect::descriptor::{MethodQualifier, StructKind};
use qubit_reflect::invoke::{Invocation, InvocationOutput};
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::{Reflect, reflect, reflect_impl};
use qubit_reflect::registry::ReflectRegistry;

struct Sample;

impl Reflect for Sample {
    fn type_descriptor() -> &'static qubit_reflect::TypeDescriptor {
        static DESCRIPTOR: OnceLock<qubit_reflect::TypeDescriptor> = OnceLock::new();
        DESCRIPTOR.get_or_init(|| {
            qubit_reflect::__private::descriptor::struct_type::<Sample>(
                "Sample",
                StructKind::Named,
                &[],
            )
        })
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
    fn type_descriptor() -> &'static qubit_reflect::TypeDescriptor {
        static DESCRIPTOR: OnceLock<qubit_reflect::TypeDescriptor> = OnceLock::new();
        DESCRIPTOR.get_or_init(|| {
            qubit_reflect::__private::descriptor::struct_type::<Counter>(
                "Counter",
                StructKind::Named,
                &[],
            )
        })
    }
}

#[reflect_impl]
impl Counter {
    fn reflected_mut(&mut self) -> u8 {
        self.0 += 1;
        self.0
    }

    fn reflected_owned(self) -> u8 {
        self.0
    }
}

#[test]
fn test_reflect_impl_generates_callable_adapter_for_shared_receiver() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let qubit_reflect::descriptor::MethodLookup::Unique(instance) =
        qubit_reflect::descriptor::ImplDescriptor::lookup_method(
            implementations,
            MethodQualifier::Inherent,
            "reflected_shared",
        )
    else {
        panic!("generated shared method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("safe shared method needs adapter");
    let sample = Sample;
    let output = adapter
        .invoke_local(Invocation::borrowed(
            qubit_reflect::value::DynamicRef::<qubit_reflect::value::Local>::new(&sample),
            [],
        ))
        .expect("local adapter must be present")
        .expect("validated invocation must call method");
    let InvocationOutput::Owned(value) = output else {
        panic!("shared method must return an owned value");
    };
    let Ok(value) = DynamicOwned::<qubit_reflect::value::Local>::downcast::<u8>(value) else {
        panic!("generated value must retain type");
    };
    assert_eq!(value, 19);
}

#[test]
fn test_reflect_impl_generates_callable_adapter_for_mutable_receiver() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Counter::type_descriptor().type_id());
    let qubit_reflect::descriptor::MethodLookup::Unique(instance) =
        qubit_reflect::descriptor::ImplDescriptor::lookup_method(
            implementations,
            MethodQualifier::Inherent,
            "reflected_mut",
        )
    else {
        panic!("generated mutable method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("safe mutable method needs adapter");
    let mut counter = Counter(3);
    let value = {
        let output = adapter
            .invoke_local(Invocation::borrowed_mut(
                qubit_reflect::value::DynamicMut::<qubit_reflect::value::Local>::new(&mut counter),
                [],
            ))
            .expect("local adapter must be present")
            .expect("validated invocation must call method");
        let InvocationOutput::Owned(value) = output else {
            panic!("mutable method must return an owned value");
        };
        let Ok(value) = DynamicOwned::<qubit_reflect::value::Local>::downcast::<u8>(value) else {
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
    let qubit_reflect::descriptor::MethodLookup::Unique(instance) =
        qubit_reflect::descriptor::ImplDescriptor::lookup_method(
            implementations,
            MethodQualifier::Inherent,
            "reflected_owned",
        )
    else {
        panic!("generated owned method instance must be discoverable");
    };
    let adapter = instance.adapter().expect("safe owned method needs adapter");
    let output = adapter
        .invoke_local(Invocation::owned(
            DynamicOwned::<qubit_reflect::value::Local>::new(Counter(23)),
            [],
        ))
        .expect("local adapter must be present")
        .expect("validated invocation must call method");
    let InvocationOutput::Owned(value) = output else {
        panic!("owned method must return an owned value");
    };
    let Ok(value) = DynamicOwned::<qubit_reflect::value::Local>::downcast::<u8>(value) else {
        panic!("generated value must retain type");
    };
    assert_eq!(value, 23);
}

#[test]
fn test_reflect_impl_does_not_generate_adapter_for_non_rust_abi() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    let implementations = registry.implementations(Sample::type_descriptor().type_id());
    let qubit_reflect::descriptor::MethodLookup::Unique(instance) =
        qubit_reflect::descriptor::ImplDescriptor::lookup_method(
            implementations,
            MethodQualifier::Inherent,
            "reflected_c_abi",
        )
    else {
        panic!("non-Rust ABI method instance must remain describable");
    };
    assert!(instance.adapter().is_none());
    assert!(instance
        .unavailable_reasons()
        .contains(&qubit_reflect::descriptor::InvocationUnavailableReason::UnsupportedAbi));
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
    let qubit_reflect::descriptor::MethodLookup::Unique(instance) =
        qubit_reflect::descriptor::ImplDescriptor::lookup_method(
            implementations,
            MethodQualifier::Inherent,
            "reflected_associated",
        )
    else {
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
    let Ok(value) = DynamicOwned::<qubit_reflect::value::Local>::downcast::<u8>(value) else {
        panic!("generated value must retain type");
    };
    assert_eq!(value, 17);
}
