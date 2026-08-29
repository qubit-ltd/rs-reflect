use qubit_reflect::Reflect;

#[derive(Reflect)]
struct Record<T> {
    value: T,
}

fn main() {
    let _ = Record { value: 7_u8 };
}
