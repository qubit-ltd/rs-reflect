use qubit_reflect::Reflect;
use qubit_reflect::reflect_impl;

#[derive(Reflect)]
struct Worker(u8);

#[reflect_impl]
impl Worker {
    #[reflect(catch_unwind)]
    fn run(&mut self) {
        self.0 += 1;
    }
}

fn main() {}
