use std::rc::Rc;

struct LocalOnly(Rc<()>);

impl qubit_reflect::__private::codegen_v4::descriptor::Reflect for LocalOnly {
    fn __reflect_registration()
    -> &'static qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<Self> {
        static REGISTRATION: qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<
            LocalOnly,
        > = qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration::new();
        &REGISTRATION
    }
}

#[qubit_reflect::ReflectImpl]
impl LocalOnly {
    fn local_method(&self) -> usize {
        Rc::strong_count(&self.0)
    }
}

fn invalid_thread_safe_binding() {
    let _ = qubit_reflect::bind_method!(
        LocalOnly::local_method as thread_safe fn(&LocalOnly) -> usize
    );
}

fn main() {}
