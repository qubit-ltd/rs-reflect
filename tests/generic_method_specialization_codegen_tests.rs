#![cfg(feature = "derive")]

use qubit_reflect::__private::codegen_v4::registration::MethodBinding;
use qubit_reflect::bind_method;
use qubit_reflect::{Reflect, ReflectImpl};
use std::pin::Pin;

struct SpecializationFixture;

impl qubit_reflect::__private::codegen_v4::descriptor::Reflect for SpecializationFixture {
    fn __reflect_registration()
    -> &'static qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<Self> {
        static REGISTRATION: qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<
            SpecializationFixture,
        > = qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration::new();
        &REGISTRATION
    }
}

#[Reflect]
trait DefaultMethodFixture<T: qubit_reflect::__private::codegen_v4::descriptor::Reflect> {
    fn ordinary_default(&self, value: T) -> T {
        value
    }

    fn pinned_default(self: Pin<&mut Self>, value: T) -> T {
        value
    }

    fn generic_default<U>(&self, value: U) -> U {
        value
    }
}

#[ReflectImpl]
impl DefaultMethodFixture<u32> for SpecializationFixture {}

#[ReflectImpl]
impl SpecializationFixture {
    fn map<T>(&self, value: T) -> T {
        value
    }

    fn const_value<const N: usize>(&self) -> usize {
        N
    }
}

fn type_binding() -> MethodBinding<SpecializationFixture> {
    bind_method!(SpecializationFixture::map::<u32> as fn(&SpecializationFixture, u32) -> u32)
}

fn typed_const_binding() -> MethodBinding<SpecializationFixture> {
    bind_method!(
        SpecializationFixture::const_value::<3usize> as fn(&SpecializationFixture) -> usize
    )
}

fn inferred_const_binding() -> MethodBinding<SpecializationFixture> {
    bind_method!(SpecializationFixture::const_value::<3> as fn(&SpecializationFixture) -> usize)
}

fn thread_safe_type_binding() -> MethodBinding<SpecializationFixture> {
    bind_method!(
        SpecializationFixture::map::<u32>
            as thread_safe fn(&SpecializationFixture, u32) -> u32
    )
}

fn thread_safe_default_binding() -> MethodBinding<SpecializationFixture> {
    bind_method!(
        <SpecializationFixture as DefaultMethodFixture<u32>>::ordinary_default
            as thread_safe fn(&SpecializationFixture, u32) -> u32
    )
}

fn thread_safe_pinned_default_binding() -> MethodBinding<SpecializationFixture> {
    bind_method!(
        <SpecializationFixture as DefaultMethodFixture<u32>>::pinned_default
            as thread_safe fn(Pin<&mut SpecializationFixture>, u32) -> u32
    )
}

#[test]
fn generic_method_bindings_expand_to_typed_runtime_specs() {
    let _ = (
        type_binding().effective(),
        typed_const_binding().effective(),
        inferred_const_binding().effective(),
        thread_safe_type_binding().effective(),
        thread_safe_default_binding().effective(),
        thread_safe_pinned_default_binding().effective(),
    );
}
