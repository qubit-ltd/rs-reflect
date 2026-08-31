use qubit_reflect::ReflectedRef;
use qubit_reflect::reflect;

#[reflect]
trait Service {
    fn value(&self) -> usize;
}

fn wrap(value: &dyn Service) {
    let _ = ReflectedRef::new(value);
}

fn main() {}
