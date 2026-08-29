// qubit-style: allow explicit-imports
use qubit_reflect as reflect;
use reflect::Reflect;

#[derive(Reflect)]
struct Record<T> {
    value: T,
}

fn main() {
    let _ = Record { value: 7_u8 };
}
