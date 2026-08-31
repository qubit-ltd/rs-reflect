use qubit_reflect::Reflect;
use qubit_reflect::reflect_impl;

#[derive(Reflect)]
struct Worker;

#[reflect_impl]
impl Worker {
    #[reflect(catch_unwind)]
    async fn run() {}
}

fn main() {}
