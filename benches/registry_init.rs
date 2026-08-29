//! Baseline executable for hot registry initialization benchmarking.

use std::hint::black_box;

use qubit_reflect::registry::ReflectRegistry;

fn main() {
    for _ in 0..100_000 {
        black_box(ReflectRegistry::initialize().expect("registry initialization"));
    }
}
