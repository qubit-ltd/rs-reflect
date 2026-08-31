use qubit_reflect::Reflect;
use qubit_reflect::reflect;
use qubit_reflect::reflect_impl;

#[derive(Reflect)]
#[reflect(opaque)]
struct DefaultStrSample;

#[reflect]
trait DefaultStr {
    fn echo_default(value: &str) -> &str {
        value
    }
}

#[reflect_impl]
impl DefaultStr for DefaultStrSample {}

fn main() {}
