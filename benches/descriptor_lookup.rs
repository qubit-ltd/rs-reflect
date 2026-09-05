// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Criterion benchmarks for descriptor lookup.

use std::sync::Barrier;
use std::time::Duration;
use std::time::Instant;

use criterion::Criterion;
use criterion::black_box;
use criterion::criterion_group;
use qubit_reflect::TypeDescriptor;

/// Registers cold-shape and hot-interner descriptor cases.
fn descriptor_lookup(criterion: &mut Criterion) {
    criterion.bench_function("descriptor/hot_nested_shape", |bench| {
        bench.iter(|| black_box(TypeDescriptor::of::<Vec<Option<String>>>()));
    });
    criterion.bench_function("descriptor/hot_builtin_pair", |bench| {
        bench.iter(|| {
            black_box(TypeDescriptor::of::<u64>());
            black_box(TypeDescriptor::of::<Vec<String>>());
        });
    });
    for workers in [1_usize, 4, 8] {
        criterion.bench_function(&format!("descriptor/concurrent_hot/{workers}"), |bench| {
            bench.iter_custom(|iterations| {
                let barrier = Barrier::new(workers);
                std::thread::scope(|scope| {
                    let handles: Vec<_> = (0..workers)
                        .map(|_| {
                            scope.spawn(|| {
                                // Warm each thread before starting the timed batch.
                                black_box(TypeDescriptor::of::<Vec<Option<String>>>());
                                barrier.wait();
                                let start = Instant::now();
                                for _ in 0..iterations {
                                    black_box(TypeDescriptor::of::<Vec<Option<String>>>());
                                }
                                start.elapsed()
                            })
                        })
                        .collect();
                    handles
                        .into_iter()
                        .map(|handle| handle.join().expect("benchmark worker"))
                        .max()
                        .unwrap_or(Duration::ZERO)
                })
            });
        });
    }
}

criterion_group!(benches, descriptor_lookup);
/// Measures first initialization in a fresh child, excluding process startup.
fn main() {
    if std::env::var_os("QUBIT_REFLECT_COLD_SAMPLE").is_some() {
        let start = Instant::now();
        let descriptor = TypeDescriptor::of::<Vec<Option<String>>>();
        black_box(descriptor);
        println!("{}", start.elapsed().as_nanos());
        return;
    }
    let executable = std::env::current_exe().expect("benchmark executable");
    let mut samples = Vec::new();
    for _ in 0..20 {
        let output = std::process::Command::new(&executable)
            .env("QUBIT_REFLECT_COLD_SAMPLE", "1")
            .output()
            .expect("cold sample child");
        assert!(output.status.success());
        samples.push(
            String::from_utf8(output.stdout)
                .expect("sample UTF-8")
                .trim()
                .parse::<u128>()
                .expect("sample nanos"),
        );
    }
    samples.sort_unstable();
    println!(
        "descriptor/first_initialization: median={} ns, samples={}",
        samples[samples.len() / 2],
        samples.len()
    );
    benches();
    Criterion::default().configure_from_args().final_summary();
}
