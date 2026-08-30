//! Baseline executable for descriptor-bound dynamic operation lookup.

use std::hint::black_box;

use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::construct::NamedConstructionInput;
use qubit_reflect::value::ReflectedOwned;

#[derive(Reflect)]
struct BenchmarkRecord {
    id: u64,
}

fn main() {
    let descriptor = TypeDescriptor::of::<BenchmarkRecord>();
    for value in 0..100_000_u64 {
        let constructed = descriptor
            .construct_struct(NamedConstructionInput::new([("id", ReflectedOwned::new(value))]))
            .expect("benchmark construction input is valid");
        black_box(constructed);
        black_box(descriptor.field("id"));
    }
}
