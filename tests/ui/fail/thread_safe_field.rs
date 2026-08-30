use qubit_reflect::Reflect;

#[derive(Reflect)]
struct Invalid {
    #[reflect(thread_safe)]
    value: u32,
}

fn main() {}
