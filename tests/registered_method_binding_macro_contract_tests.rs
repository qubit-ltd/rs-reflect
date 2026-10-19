use qubit_reflect::{ReflectImpl, register_method_binding};
use qubit_reflect::__private::codegen_v4::descriptor::{Reflect, TypeRegistration};

struct GenericMethodOwner;

impl Reflect for GenericMethodOwner {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        static REGISTRATION: TypeRegistration<GenericMethodOwner> = TypeRegistration::new();
        &REGISTRATION
    }
}

#[ReflectImpl]
impl GenericMethodOwner {
    fn convert<T: Reflect>(&self, value: T) -> T {
        value
    }
}

register_method_binding!(
    GenericMethodOwner::convert::<u64>
    as fn(&GenericMethodOwner, u64) -> u64
);

#[test]
fn test_public_register_method_binding_accepts_typed_method_path() {
    assert_eq!(
        qubit_reflect::__private::codegen_v4::PROTOCOL_VERSION,
        4,
    );
}
