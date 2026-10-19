#[path = "../access/tests.rs"]
mod access;
#[path = "../capability/tests.rs"]
mod capability;
#[path = "construct.rs"]
mod construct;
#[path = "../context/tests.rs"]
mod context;
#[path = "expression.rs"]
mod expression;
#[path = "invoke.rs"]
mod invoke;
mod lookup_error;
mod seam;
#[path = "trait_impl.rs"]
mod trait_impl;
#[path = "value.rs"]
mod value;

impl crate::runtime::descriptor::Reflect for String {
    fn __reflect_registration() -> &'static crate::runtime::descriptor::TypeRegistration<Self> {
        static REGISTRATION: crate::runtime::descriptor::TypeRegistration<String> =
            crate::runtime::descriptor::TypeRegistration::new();
        &REGISTRATION
    }
}

impl crate::runtime::descriptor::Reflect for u32 {
    fn __reflect_registration() -> &'static crate::runtime::descriptor::TypeRegistration<Self> {
        static REGISTRATION: crate::runtime::descriptor::TypeRegistration<u32> =
            crate::runtime::descriptor::TypeRegistration::new();
        &REGISTRATION
    }
}
