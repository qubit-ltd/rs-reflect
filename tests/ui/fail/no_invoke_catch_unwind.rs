use qubit_reflect::Reflect;
use qubit_reflect::reflect_impl;

#[derive(Reflect)]
struct Worker;

#[reflect_impl]
impl Worker {
    #[reflect(no_invoke, catch_unwind)]
    fn run() {}
}

fn main() {}
