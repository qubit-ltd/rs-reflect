# Reflection and downstream model performance

Measured on 2026-09-05 with Rust 1.94.0, x86_64 Linux, Intel Core i5-9600K
(six physical cores, nominal 3.7 GHz). These are diagnostic measurements,
not performance thresholds or an application throughput guarantee.

## Descriptor lookup

Run `cargo +1.94.0 bench --bench descriptor_lookup -- --quick` in rs-reflect.

| Operation | Observed time |
| --- | ---: |
| First `Vec<Option<String>>` descriptor initialization | 2,938 ns median |
| Warm nested descriptor lookup | 58.99 ns |
| Warm builtin pair | 30.64 ns per pair |
| Concurrent warm lookup, one worker | 47.41 ns |
| Concurrent warm lookup, four workers | 278.19 ns |
| Concurrent warm lookup, eight workers | 508.80 ns |

First initialization uses 20 fresh child processes. Each child times only
descriptor initialization, excluding process startup. Criterion's repeated
nested-shape case measures warm lookup; it is no longer labelled cold.
Concurrent cases warm every worker before a barrier and report the slowest
worker's batch duration divided by that worker's iteration count. They exclude
thread creation and are not aggregate throughput measurements. Eight workers
also exceed this machine's physical core count. Quick-mode estimates require
longer runs before drawing small percentage comparisons.

## Real platform model graph

Run `cargo +1.94.0 bench -p qubit-platform-testkit --bench model_registry`
in rs-platform. The executable links 131 actual platform models and validates
their relationships. Allocation instrumentation delegates to the system
allocator and counts allocation, zeroed-allocation, and reallocation requests.

| Operation | Requests per operation | Requested bytes per operation |
| --- | ---: | ---: |
| Cold reflection linking and model projection | 10,504 | 1,979,709 |
| Warm model projection | 1,908 | 224,476 |
| Relationship validation | 3,630 | 391,662 |

Cold linking is measured once per process. The two warm operations are measured
100 times, including destruction of temporary results. Byte counts describe
cumulative requested allocation sizes, including reallocations, rather than
retained heap size. The counting allocator itself adds atomic-operation costs.

Concurrent CI builds made wall-clock times unstable: observed cold runs ranged
from about 1.6 to 10.1 ms, warm projection from 0.45 to 4.87 ms, and relationship
validation from 0.40 to 5.40 ms. Allocation counts were stable across these runs.
Do not use these loaded-machine timings as a regression baseline; repeat on an
idle controlled runner before setting latency budgets.

## Decision

Keep the existing descriptor interner for now. Contention is measurable, but
there is no demonstrated downstream latency or throughput requirement that
justifies a thread-local cache and its additional memory and consistency costs.
The generic capability correctness fix uses concrete `TypeId` keys, while
registry-dependent trait links and model properties remain snapshot-local.
Future optimization should compare end-to-end model workloads and allocation
costs, not only a synthetic repeated descriptor lookup.

## 2026-09-24 effective method view baseline

The downstream application source contains 22 `#[ModelImpl]` blocks declaring
46 methods, with a mean of about 2.1 methods per block and a maximum of 9. The
per-block source counts are: 13 blocks with 1 method, 5 with 2, 2 with 4, 1
with 6, and 1 with 9. Counts were derived from the method declarations inside
macro-annotated impl blocks in `rs-platform`, excluding the test-only fixture.
They are a source-level workload estimate, not counts read from a successfully
linked runtime registry; trait defaults or method overrides may change the
effective-view count. The attempted downstream
`ci-check.sh` is currently blocked by an unrelated exhaustive-match error in
`modules/core/src/mixin/validation_violation.rs` for the non-exhaustive
`qubit_validator::PathSegment` enum.

Run the isolated effective-view construction benchmark in rs-reflect:

```bash
cargo +1.94.0 bench --locked --features bench-internals --bench registry_init -- --quick
```

Criterion runs on the same Rust 1.94.0 x86_64 Linux Intel Core i5-9600K machine
described above. The group constructs distinct synthetic inherent method
instances before timing and times only `EffectiveTypeView::new` through a
benchmark-only hook. The four sizes are the zero-method baseline, the
downstream mean rounded to 2, the observed maximum of 9, and twice that maximum.
Quick mode is diagnostic and the estimates are not a controlled-runner
regression baseline.

| Effective method instances | Estimate | Quick-mode interval |
| ---: | ---: | ---: |
| 0 | 11.87 ns | 11.85–11.91 ns |
| 2 | 116.50 ns | 116.45–116.52 ns |
| 9 | 565.71 ns | 562.94–576.81 ns |
| 18 | 1.431 µs | 1.429–1.438 µs |

The method view at the observed maximum is below one microsecond in this
synthetic isolated measurement. The 18-method exploratory case is about 1.4 µs.
These measurements do not meet the design decision gate of 1 ms, and the
downstream runtime registry could not be profiled because its build is blocked.
There is no evidence that duplicate search dominates cold registry startup, so
keep the current implementation. Revisit only if a working downstream build
and profile show method-view construction reaches 10% of cold registry
construction or 1 ms at a real workload size.

## 2026-09-06 contract-refactoring measurements

The following measurements were taken after the capability-error and struct-shape
changes, with all three repositories' `align-ci.sh` and `ci-check.sh` passing.
The machine and compiler match the environment above: Rust 1.94.0 (LLVM 21.1.8),
x86_64 Linux, Intel Core i5-9600K, six physical cores. Benchmarks ran serially
after compiling all benchmark executables; no concurrent CI was launched during
sampling. The initial system load averages were 4.21/3.07/2.56, following CI.
This was not a dedicated runner with controlled CPU frequency or affinity.

All commands used `--locked` and each package's default feature selection.
Unlike the earlier quick run, Criterion used its default 3-second warmup,
100 samples and approximately 5-second measurement window per case. Bracketed
ranges below are Criterion's reported estimate intervals. Cached Criterion
percentage comparisons against earlier runs are intentionally not used: the
sampling method and machine conditions differ, so they cannot establish an
improvement or regression caused by this refactor.

### Property output conversion

Run in `rs-model-metadata`:

```bash
cargo +1.94.0 bench --locked --bench property_output
```

Each fixture is a `u64` or a slice of `u64`. Native access reads one middle
entry. Direct access constructs a `BorrowedPropertySlice` and reads one entry.
Full conversion constructs the wrapper and materializes `InvocationOutput`.
Conversion-only uses `iter_batched` to exclude wrapper construction from the
measured routine. Every group includes destruction of its returned output;
fixture allocation happens outside the measured routine.

| Slice length | Native access | Direct access | Full conversion | Conversion-only |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 0.788 ns | 1.446 ns | 33.92 ns | 38.42 ns |
| 1 | 0.784 ns | 1.375 ns | 48.10 ns | 52.92 ns |
| 32 | 0.784 ns | 1.379 ns | 199.3 ns | 205.4 ns |
| 1,024 | 0.793 ns | 1.366 ns | 5.342 µs | 5.401 µs |
| 65,536 | 0.780 ns | 1.370 ns | 333.2 µs | 333.4 µs |

The full-conversion intervals were 33.69–34.17 ns, 47.78–48.48 ns,
198.1–200.5 ns, 5.308–5.379 µs and 331.2–335.2 µs respectively.
The scalar conversion estimates were 15.23 ns for borrowed, 13.99 ns for
optional-present, 14.63 ns for optional-absent, and 21.20 ns for owned. The owned
case excludes initial owned-value allocation through `iter_batched` and includes
output destruction.

These results support the expected O(n) cost of explicit slice materialization.
They do not prove that general property access is allocation-free: the slice
wrapper uses a boxed adapter, and optimization can eliminate a non-escaping
wrapper in this simple direct-access case. Nor can full and conversion-only
numbers be subtracted to estimate allocation cost: batching changes setup,
cache state and optimizer visibility. The benchmarks do not invoke a model's
property getter and are not an end-to-end request latency measurement.

### Descriptor lookup and contention

Run in `rs-reflect`:

```bash
cargo +1.94.0 bench --locked --bench descriptor_lookup
```

First initialization in 20 fresh child processes had a median of 2,461 ns.
The measurement boundaries for child startup and concurrent worker batches
remain those described above.

| Operation | Estimate | Reported interval |
| --- | ---: | ---: |
| Warm nested descriptor | 21.78 ns | 21.65–21.94 ns |
| Warm builtin pair | 21.73 ns | 21.62–21.83 ns |
| Concurrent warm lookup, one worker | 21.65 ns | 21.51–21.80 ns |
| Concurrent warm lookup, four workers | 365.3 ns | 363.3–367.1 ns |
| Concurrent warm lookup, eight workers | 780.2 ns | 775.9–784.0 ns |

### Actual platform workload

Run the following command three times in `rs-platform`, each starting a fresh
process:

```bash
cargo +1.94.0 bench --locked -p qubit-platform-testkit --bench model_registry
```

All three runs linked 131 models and produced identical allocation counts.
Times below span the three per-process measurements, not confidence intervals.
Warm operations perform 100 iterations per process, including result destruction.

| Operation | Time range | Requests/op | Requested bytes/op |
| --- | ---: | ---: | ---: |
| Cold linking and projection | 1.584–1.867 ms | 10,504 | 1,984,949 |
| Warm model projection | 425.1–465.2 µs | 1,908 | 224,476 |
| Relationship validation | 400.1–416.4 µs | 3,630 | 391,662 |

The cold requested-byte total differs from the earlier report; it includes
initialization allocations and is not a retained-memory measurement. No
attribution to a specific structure-size change is established by these data.

### Build and artifact size

A newly created temporary target directory was used for the release
`model-inventory` binary, with `CARGO_INCREMENTAL=1`. Existing user targets were
preserved. Run in `rs-platform`, with a fresh target for each cold measurement:

```bash
REFLECT_MEASURE_TARGET=$(mktemp -d /tmp/reflect-build-measure.XXXXXX)
CARGO_TARGET_DIR="$REFLECT_MEASURE_TARGET" CARGO_INCREMENTAL=1 \
    cargo +1.94.0 build --locked --release -p qubit-platform-testkit --bin model-inventory
CARGO_TARGET_DIR="$REFLECT_MEASURE_TARGET" CARGO_INCREMENTAL=1 \
    cargo +1.94.0 build --locked --release -p qubit-platform-testkit --bin model-inventory
```

| Measurement | Result |
| --- | ---: |
| Clean-target build wall time, including Cargo startup | 59.74 s |
| Repeated build with unchanged sources | 77.77 ms |
| Release binary size, without an additional strip step | 6,437,056 bytes |

The second command measures Cargo's cached no-op build, **not** source-edit
incremental compilation latency. The cold target excludes compiled dependencies,
but downloaded sources and the toolchain were already cached. These are single
build observations, not a compiler-performance baseline or a cross-version
comparison.

### Refactoring decision

Keep explicit output conversion, the descriptor interner, and snapshot-local
model overlays. The measurements establish a linear materialization cost and
concurrent interner contention, but the inspected downstream call sites provide
no production conversion workload or latency budget that justifies replacing
these APIs or introducing another cache. Correctness changes are supported by
regressions for three-state capability queries, frozen lookups, concrete generic
identity, empty struct shapes, and recoverable downstream errors. Performance
architecture changes require a separate reproducible workload and budget.
