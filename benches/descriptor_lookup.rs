// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Criterion benchmarks for descriptor lookup.
//!
//! The first, last, and missing-field cases measure lookup position costs;
//! they do not represent production query frequencies.

use std::hint::black_box;
use std::sync::Barrier;
use std::time::Duration;
use std::time::Instant;

use criterion::Criterion;
use criterion::criterion_group;
use qubit_reflect::TypeDescriptor;

macro_rules! define_lookup_record {
    ($name:ident; $($field:ident),+ $(,)?) => {
        #[derive(qubit_reflect::Reflect)]
        struct $name {
            $($field: u8,)+
        }
    };
}

#[derive(qubit_reflect::Reflect)]
struct FieldLookupRecord {
    f00: u8,
    f01: u8,
    f02: u8,
    f03: u8,
    f04: u8,
    f05: u8,
    f06: u8,
    f07: u8,
    f08: u8,
    f09: u8,
    f10: u8,
    f11: u8,
    f12: u8,
    f13: u8,
    f14: u8,
    f15: u8,
}

// Widths reflect production model counts: 1/2 cover small shapes; 9/17/51
// cover the platform p50, nearest-rank p90, and maximum. Keep the original
// 16-field case as a historical comparison.
define_lookup_record!(FieldLookupOne; f00);
define_lookup_record!(FieldLookupTwo; f00, f01);
define_lookup_record!(FieldLookupNine; f00, f01, f02, f03, f04, f05, f06, f07, f08);
define_lookup_record!(FieldLookupSeventeen;
    f00, f01, f02, f03, f04, f05, f06, f07, f08, f09, f10, f11, f12, f13, f14, f15, f16,
);
define_lookup_record!(FieldLookupFiftyOne;
    f00, f01, f02, f03, f04, f05, f06, f07, f08, f09, f10, f11,
    f12, f13, f14, f15, f16, f17, f18, f19, f20, f21, f22, f23,
    f24, f25, f26, f27, f28, f29, f30, f31, f32, f33, f34, f35,
    f36, f37, f38, f39, f40, f41, f42, f43, f44, f45, f46, f47,
    f48, f49, f50,
);

fn benchmark_field_cases(
    criterion: &mut Criterion,
    descriptor: &'static TypeDescriptor,
    width: usize,
    legacy_labels: bool,
) {
    let first = "f00";
    let last = Box::leak(format!("f{:02}", width - 1).into_boxed_str());
    let missing = "absent";
    assert!(descriptor.field(first).is_some());
    assert!(descriptor.field(last).is_some());
    assert!(descriptor.field(missing).is_none());

    for (label, name) in [("first", first), ("last", last), ("missing", missing)] {
        let id = if legacy_labels {
            format!("field_lookup/{label}")
        } else {
            format!("field_lookup/{width}_fields/{label}")
        };
        criterion.bench_function(&id, |bench| {
            bench.iter(|| black_box(descriptor.field(black_box(name))));
        });
    }
}

/// Registers cold-shape and hot-interner descriptor cases.
fn descriptor_lookup(criterion: &mut Criterion) {
    let fields = TypeDescriptor::of::<FieldLookupRecord>();
    let field_descriptor_bytes = std::mem::size_of_val(fields.field("f00").expect("first field"));
    for width in [1_usize, 2, 9, 16, 17, 51] {
        let descriptor_bytes = width * field_descriptor_bytes;
        // Approximate a HashMap<&str, usize> table: fat key + index per bucket,
        // one control byte per bucket, 7/8 maximum occupancy, and power-of-two
        // capacity. This estimates table storage; it does not measure an index.
        let required_buckets = (width * 8).div_ceil(7).next_power_of_two().max(4);
        let estimated_index_bytes =
            required_buckets * std::mem::size_of::<(&str, usize)>() + required_buckets + 16;
        eprintln!(
            "field_memory width={width} descriptor_array_bytes={descriptor_bytes} estimated_index_bytes={estimated_index_bytes}"
        );
    }
    benchmark_field_cases(criterion, fields, 16, true);
    benchmark_field_cases(criterion, TypeDescriptor::of::<FieldLookupOne>(), 1, false);
    benchmark_field_cases(criterion, TypeDescriptor::of::<FieldLookupTwo>(), 2, false);
    benchmark_field_cases(criterion, TypeDescriptor::of::<FieldLookupNine>(), 9, false);
    benchmark_field_cases(
        criterion,
        TypeDescriptor::of::<FieldLookupSeventeen>(),
        17,
        false,
    );
    benchmark_field_cases(
        criterion,
        TypeDescriptor::of::<FieldLookupFiftyOne>(),
        51,
        false,
    );

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
                                // Warm each thread before starting the
                                // timed batch.
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
