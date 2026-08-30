use qubit_reflect::Reflect;

#[derive(Reflect)]
struct Invalid(#[reflect(rename = "value")] u32);

fn main() {}
