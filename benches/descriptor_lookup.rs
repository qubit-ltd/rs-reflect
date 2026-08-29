//! Baseline executable for descriptor lookup benchmarking.

use std::hint::black_box;

use qubit_reflect::TypeDescriptor;

fn main() {
    for _ in 0..100_000 {
        black_box(TypeDescriptor::of::<u64>());
        black_box(TypeDescriptor::of::<Vec<String>>());
    }
}
