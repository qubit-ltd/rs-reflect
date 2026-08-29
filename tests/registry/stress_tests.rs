// qubit-style: allow explicit-imports
//! Concurrency stress coverage for reflection registry and descriptor
//! interning.
use std::sync::Arc;
use std::sync::Barrier;

use qubit_reflect as reflect;
use reflect::Reflect;
use reflect::TypeDescriptor;
use reflect::registry::ReflectRegistry;

#[derive(Reflect)]
struct StressRecord<T> {
    values: Vec<T>,
}

/// Verifies first-time generic descriptor interning and registry lookup
/// converge under a synchronized multi-threaded start.
#[test]
fn test_stress_concurrent_generic_descriptor_and_registry_initialization() {
    const WORKERS: usize = 16;
    let barrier = Arc::new(Barrier::new(WORKERS));
    let workers: Vec<_> = (0..WORKERS)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                let descriptor = TypeDescriptor::of::<StressRecord<u64>>();
                let registry = ReflectRegistry::initialize().expect("registry must initialize once");
                (descriptor as *const TypeDescriptor as usize, registry.types().len())
            })
        })
        .collect();
    let results: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().expect("stress worker must not panic"))
        .collect();

    assert!(results.iter().all(|result| result.0 == results[0].0));
    assert!(results.iter().all(|result| result.1 == results[0].1));
}
