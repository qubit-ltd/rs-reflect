use qubit_reflect::invoke::InvocationArg;
use qubit_reflect::Invocation;
use qubit_reflect::InvocationOutput;
use qubit_reflect::ReflectRegistry;
use qubit_reflect::ReflectedRef;
use qubit_reflect::descriptor::MethodInstanceDescriptor;
use qubit_reflect::value::Local;

fn escape_receiver(method: &MethodInstanceDescriptor, registry: &ReflectRegistry) -> InvocationOutput<'static, Local> {
    let receiver = 7_u8;
    method.invoke_local(registry, Invocation::borrowed(ReflectedRef::new(&receiver), [])).unwrap().unwrap()
}

fn escape_argument(method: &MethodInstanceDescriptor, registry: &ReflectRegistry) -> InvocationOutput<'static, Local> {
    let argument = String::from("borrowed");
    method.invoke_local(registry, Invocation::associated([
        InvocationArg::Ref(ReflectedRef::new(&argument)),
    ])).unwrap().unwrap()
}

fn main() {}
