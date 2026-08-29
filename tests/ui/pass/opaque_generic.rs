use std::rc::Rc;

use qubit_reflect::Reflect;

#[derive(Reflect)]
#[reflect(opaque)]
struct Opaque<T> {
    value: T,
}

fn main() {
    let _ = Opaque { value: Rc::new(()) };
}
