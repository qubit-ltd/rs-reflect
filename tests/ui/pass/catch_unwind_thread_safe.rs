use qubit_reflect::Reflect;
use qubit_reflect::reflect_impl;

#[derive(Reflect)]
struct Worker;

#[reflect_impl]
impl Worker {
    #[reflect(thread_safe, catch_unwind)]
    fn run(value: u32) -> u32 {
        value + 1
    }
}

fn main() {}
