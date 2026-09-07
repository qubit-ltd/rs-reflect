# rs-reflect refactor performance evidence

This report records the reproducible build and runtime measurements completed
on 2026-09-07 with Rust 1.94.0. The machine-readable medians, ranges, and
representative runtime/platform metrics are in the [performance summary](2026-09-06-refactor-performance-summary.json).

Raw benchmark samples are not versioned. The summary deliberately records
`raw_samples_available: false`; it is an evidence index, not a replacement for
the missing 36 raw before/after build samples. No raw samples or unobserved
percentiles are inferred from the summary.

The build comparison used independent temporary trees and Cargo target
directories with `CARGO_INCREMENTAL=1` and:

```text
cargo +1.94.0 build --locked --release -p qubit-platform-testkit --bin model-inventory
```

Six scenarios are represented in the summary: clean build, Cargo cached no-op,
method-edit warmup and incremental build, and field-edit warmup and
incremental build. Each layout ran three observations per scenario. All build
steps completed successfully, and the source trees retained their original
SHA-256. The ranges are wide because release builds include LTO on a shared
machine; the before and after layouts also include intended source and
dependency changes.

Runtime Criterion and allocator-separated platform measurements are recorded
only as representative metrics in the summary. They cover the scalar property
output groups and the 131-model `model_registry` workload. Allocation counters
come from the benchmark counting allocator and are not retained heap sizes.

These observations do not establish a production optimization conclusion or a
latency budget. The refactor did not produce a systematic build failure, but
the samples are insufficient to attribute changes to a single source edit or
to justify an architecture change. Re-run on a controlled runner with raw
artifacts retained before using these numbers as a regression baseline.
