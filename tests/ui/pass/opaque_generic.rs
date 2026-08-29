// qubit-style: allow explicit-imports
use qubit_reflect as reflect;
use std::rc::Rc;

use reflect::Reflect;

#[derive(Reflect)]
#[reflect(opaque)]
struct Opaque<T> {
    value: T,
}

fn main() {
    let _ = Opaque { value: Rc::new(()) };
}
