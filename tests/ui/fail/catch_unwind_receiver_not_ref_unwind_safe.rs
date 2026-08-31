use std::cell::Cell;

use qubit_reflect::Reflect;
use qubit_reflect::reflect_impl;

#[derive(Reflect)]
#[reflect(opaque)]
struct Worker {
    state: Cell<u8>,
}

#[reflect_impl]
impl Worker {
    #[reflect(catch_unwind)]
    fn run(&self) -> u8 {
        self.state.get()
    }
}

fn main() {}
