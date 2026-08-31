use std::cell::Cell;
use std::rc::Rc;

use qubit_reflect::Reflect;
use qubit_reflect::reflect_impl;

#[derive(Reflect)]
#[reflect(opaque)]
struct Fragile {
    state: Cell<u8>,
}

#[derive(Reflect)]
struct Worker;

#[reflect_impl]
impl Worker {
    #[reflect(catch_unwind)]
    fn run() -> Rc<Fragile> {
        Rc::new(Fragile {
            state: Cell::new(1),
        })
    }
}

fn main() {}
