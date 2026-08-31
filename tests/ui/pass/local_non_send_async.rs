use std::cell::Cell;
use std::rc::Rc;

use qubit_reflect::Reflect;
use qubit_reflect::reflect_impl;

#[derive(Reflect)]
struct Worker;

#[reflect_impl]
impl Worker {
    async fn run(value: u8) -> u8 {
        let local = Rc::new(Cell::new(value));
        std::future::ready(()).await;
        local.get()
    }
}

fn main() {}
