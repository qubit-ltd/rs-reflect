use std::rc::Rc;

use qubit_reflect::Reflect;
use qubit_reflect::reflect_impl;

#[derive(Reflect)]
struct Worker;

#[reflect_impl]
impl Worker {
    #[reflect(thread_safe)]
    fn run() -> Rc<u8> {
        Rc::new(1)
    }
}

fn main() {}
