use qubit_reflect::{Reflect, ReflectImpl};
use std::pin::Pin;

#[derive(Default)]
#[Reflect]
pub struct Worker {
    pub value: i32,
}

#[ReflectImpl]
impl Worker {
    pub fn add(&mut self, amount: i32) {
        self.value += amount;
    }

    pub fn sum(&self, left: i32, right: i32) -> i32 {
        self.value + left + right
    }

    pub fn echo(&self, value: String) -> String {
        value
    }

    pub fn panic(&self) {
        panic!("worker panic payload");
    }

    fn private_method(&self) {}

    pub unsafe fn unsafe_method(&self) {}

    pub fn pinned_value(self: Pin<&mut Self>) -> i32 {
        self.value
    }

    pub async fn async_value(&self) -> i32 {
        self.value
    }
}

pub fn method(name: &str) -> qubit_reflect::MethodDescriptor<'static> {
    qubit_reflect::ReflectionContext::global()
        .type_of::<Worker>()
        .method(name)
        .expect("the reflected worker method must be registered")
}
