use qubit_reflect::Reflect;

#[derive(Reflect)]
struct Invalid {
    #[reflect(skip, no_construct)]
    value: u32,
}

fn main() {}
