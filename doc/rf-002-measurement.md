# RF-002 registration selection measurement

## Decision

The original scan selector remains in production. The measured selection stage took about half of the repeated explicit linked build on the existing inventory, and large synthetic reverse-chain/branch graphs showed depth-related growth. An indexed queue candidate was evaluated but did not meet the retention gate: its large-graph gains were inconsistent across paired runs, the actual-inventory selector and total build results reversed between rounds, and the 32-fragment case had a greater than 5% regression in one paired round. The candidate was removed. RF-002 does not establish a production algorithm worth retaining under the stated stability and regression thresholds.

The actual-inventory selection benchmark includes inventory collection, sorting, and selection, but excludes fallback registration, dependency validation, factory materialization, and context construction. Its ratio to a fresh-builder full-build benchmark is therefore an upper bound for selection-related work, not a pure selector percentage.

## Environment and method

- Baseline commit: `18a0717` (`fix(derive): honor HRTB trait binders in lifetime analysis`).
- Toolchain: `rustc 1.94.0 (4a4ef493e 2026-03-02)`, `cargo 1.94.0`.
- Host: x86_64 Linux, Intel Xeon E5-2680 v4 @ 2.40 GHz, 2 sockets / 28 physical cores / 56 logical CPUs. The host reported CPU scaling enabled and shared CI/build load during measurements.
- Profile: Criterion release benchmark; each run used 10 samples, 100 ms warmup, and 100 ms measurement target.
- Fixtures are built outside timed closures. Synthetic selector outputs are passed through `black_box`. The matrix has 32/128/512 ordered fragments; depth 1/8/32; chain, reverse chain, branch, cycle, shared dependency, and unrelated-fragment graphs. Reverse chain and branch place the root fragment last to expose repeated-scan depth cost.
- The `first_explicit_linked_build_ns` value is one first-call observation per process. The repeated benchmark constructs a fresh builder/context on each Criterion iteration; there is no builder cache. These cold observations are noisy and are not compared statistically.

## Baseline totals and actual inventory phase

Command for each run (all exited 0):

```text
cargo bench --locked --bench registry_init --features bench-internals,derive -- --sample-size 10 --measurement-time 0.1 --warm-up-time 0.1
```

| Measure | Run 1 | Run 2 | Run 3 |
|---|---:|---:|---:|
| First explicit build, one observation | 106.606 µs | 65.877 µs | 83.735 µs |
| Repeated fresh-builder total (Criterion median) | 7.859 µs | 7.820 µs | 14.886 µs |
| Actual inventory collect + sort + selector (Criterion median) | 4.114 µs | 4.215 µs | 7.200 µs |
| Selection-stage / repeated-total ratio | 52.3% | 53.9% | 48.4% |

Run 3's build and selector medians rose substantially while the pure lookup controls varied, consistent with host contention. The ratio stayed near one half. The selection phase includes inventory collection and sorting, so the ratio should not be read as the isolated scan loop share.

## Synthetic baseline medians

The entries below are Criterion median estimates in microseconds for runs 1 / 2 / 3. Units are microseconds. Full Criterion intervals and sample data were emitted under `target/criterion` during each run; run logs were `/tmp/rf002-final-run1.log`, `/tmp/rf002-final-run2.log`, and `/tmp/rf002-final-run3.log` in the work environment.

| Graph case | Run 1 median (µs) | Run 2 median (µs) | Run 3 median (µs) |
|---|---:|---:|---:|
| `branch/fragments_128/depth_1` | 5.3766 | 5.2052 | 5.2356 |
| `branch/fragments_128/depth_32` | 10.567 | 10.650 | 12.001 |
| `branch/fragments_128/depth_8` | 8.6930 | 8.5477 | 18.766 |
| `branch/fragments_32/depth_1` | 1.4054 | 1.3933 | 2.2401 |
| `branch/fragments_32/depth_32` | 8.1174 | 4.9840 | 4.8350 |
| `branch/fragments_32/depth_8` | 2.8365 | 2.7935 | 3.9203 |
| `branch/fragments_512/depth_1` | 20.377 | 34.201 | 21.719 |
| `branch/fragments_512/depth_32` | 33.956 | 37.541 | 34.550 |
| `branch/fragments_512/depth_8` | 31.720 | 31.947 | 32.772 |
| `chain/fragments_128/depth_1` | 5.4579 | 5.2134 | 5.8239 |
| `chain/fragments_128/depth_32` | 7.4170 | 7.3288 | 10.891 |
| `chain/fragments_128/depth_8` | 6.0581 | 5.8026 | 6.2488 |
| `chain/fragments_32/depth_1` | 2.6219 | 1.3965 | 2.1158 |
| `chain/fragments_32/depth_32` | 3.4335 | 3.3703 | 3.4516 |
| `chain/fragments_32/depth_8` | 2.3432 | 1.9355 | 2.0058 |
| `chain/fragments_512/depth_1` | 20.498 | 21.671 | 21.097 |
| `chain/fragments_512/depth_32` | 23.205 | 23.492 | 23.396 |
| `chain/fragments_512/depth_8` | 21.718 | 21.619 | 30.807 |
| `cycle/fragments_128/depth_1` | 5.3602 | 5.2316 | 6.0038 |
| `cycle/fragments_128/depth_32` | 7.3848 | 7.3349 | 7.3345 |
| `cycle/fragments_128/depth_8` | 6.0396 | 5.8650 | 20.439 |
| `cycle/fragments_32/depth_1` | 1.5370 | 1.4467 | 2.2733 |
| `cycle/fragments_32/depth_32` | 5.3824 | 3.3591 | 3.3583 |
| `cycle/fragments_32/depth_8` | 3.3610 | 1.9711 | 1.9830 |
| `cycle/fragments_512/depth_1` | 20.488 | 21.055 | 20.534 |
| `cycle/fragments_512/depth_32` | 23.232 | 23.439 | 24.527 |
| `cycle/fragments_512/depth_8` | 21.644 | 21.741 | 21.676 |
| `reverse_chain/fragments_128/depth_1` | 5.2897 | 5.3675 | 5.2480 |
| `reverse_chain/fragments_128/depth_32` | 9.8409 | 9.8911 | 10.002 |
| `reverse_chain/fragments_128/depth_8` | 8.7014 | 8.4393 | 9.8467 |
| `reverse_chain/fragments_32/depth_1` | 1.4263 | 1.3983 | 1.4211 |
| `reverse_chain/fragments_32/depth_32` | 4.1056 | 3.9778 | 3.9926 |
| `reverse_chain/fragments_32/depth_8` | 2.6803 | 2.5681 | 3.1809 |
| `reverse_chain/fragments_512/depth_1` | 20.398 | 20.799 | 27.553 |
| `reverse_chain/fragments_512/depth_32` | 33.317 | 35.288 | 39.223 |
| `reverse_chain/fragments_512/depth_8` | 31.825 | 31.999 | 47.015 |
| `shared/fragments_128/depth_1` | 5.4863 | 5.3598 | 9.5197 |
| `shared/fragments_128/depth_32` | 5.1886 | 5.7944 | 5.1935 |
| `shared/fragments_128/depth_8` | 5.2375 | 5.2121 | 8.1454 |
| `shared/fragments_32/depth_1` | 1.4085 | 1.3986 | 2.2000 |
| `shared/fragments_32/depth_32` | 1.4151 | 1.4308 | 1.4141 |
| `shared/fragments_32/depth_8` | 1.5190 | 1.4126 | 1.7827 |
| `shared/fragments_512/depth_1` | 20.514 | 22.252 | 20.329 |
| `shared/fragments_512/depth_32` | 20.287 | 20.572 | 21.894 |
| `shared/fragments_512/depth_8` | 20.327 | 20.665 | 20.495 |
| `unrelated/fragments_128/depth_1` | 5.2209 | 5.1763 | 7.6920 |
| `unrelated/fragments_128/depth_32` | 5.3479 | 8.5784 | 5.2193 |
| `unrelated/fragments_128/depth_8` | 5.4170 | 5.3144 | 8.8405 |
| `unrelated/fragments_32/depth_1` | 1.4037 | 1.4064 | 2.1980 |
| `unrelated/fragments_32/depth_32` | 1.4075 | 1.4285 | 1.4102 |
| `unrelated/fragments_32/depth_8` | 1.6321 | 1.4117 | 1.4164 |
| `unrelated/fragments_512/depth_1` | 20.820 | 34.469 | 20.368 |
| `unrelated/fragments_512/depth_32` | 21.567 | 20.412 | 21.918 |
| `unrelated/fragments_512/depth_8` | 20.790 | 28.458 | 22.309 |

## Indexed candidate exploration

Two paired Criterion runs measured scan and indexed variants adjacent in the same process. They used the same inventory inputs, full-builder path, and synthetic fixtures. The host remained under concurrent build load; Run 1 began around 55 runnable-load units on a 56-logical-CPU host, and several samples had outliers.

| Paired measure, median | Run 1 scan / indexed | Run 2 scan / indexed |
|---|---:|---:|
| Full actual linked build | 10.978 / 8.539 µs | 7.892 / 16.908 µs |
| Actual inventory phase | 6.583 / 4.591 µs | 4.224 / 4.671 µs |
| 512 reverse chain, depth 32 | 51.759 / 30.008 µs | 36.428 / 31.513 µs |
| 512 branch, depth 32 | 43.226 / 30.594 µs | 32.298 / 31.575 µs |
| 32 chain, depth 1 | 2.248 / 3.050 µs | 2.161 / 2.123 µs |

The candidate's large-case benefit varied substantially: reverse-chain depth 32 improved by about 42% in paired Run 1 but about 13% in Run 2; branch depth 32 improved by about 29% in Run 1 and about 2% in Run 2. Actual inventory and full-build results reversed direction between rounds, and the small case regressed by about 36% in Run 1. Thus neither a stable 20% large-case improvement nor a stable small-case regression bound was established. The candidate was removed.

An earlier candidate-only full-matrix run completed with exit 0 but showed severe resource noise (for example, the 10,000-lookup control measured 10.8–13.5 ms). A first all-cases paired run was stopped at exit 130 to shorten the comparison; it is excluded. The two completed paired runs above are the candidate comparison evidence.

## Focused verification

Final focused commands after the production candidate was removed:

- `cargo test --locked -p qubit-reflect --lib runtime::context` — exit code 0, 34 passed, 0 failed.
- `cargo test --locked -p qubit-reflect --lib runtime::tests::context` — exit code 0, 20 passed, 0 failed, including selected/unselected factory-call assertions.
- `cargo bench --locked --bench registry_init --features bench-internals,derive --no-run` — exit code 0; the registry benchmark executable built.

An all-workspace `cargo fmt --all -- --check` was attempted and exited 1 because it reported pre-existing formatting differences throughout unrelated files. The changed Rust files were formatted with scoped `rustfmt --edition 2024`.

Other exploratory command outcomes: an early selector compile failed (exit 101) on a borrowed dependency-slice signature and was corrected; an early focused test had an incorrect expected dependency-edge order and was corrected to the observed stable scan order. The first paired all-matrix run was stopped intentionally (exit 130) to reduce shared-host load. A paired benchmark compile attempt failed (exit 101) because the hidden benchmark wrapper used a non-re-exported builder path and needed an explicit closure type; this was corrected before the completed paired runs. The candidate-only first run and both completed paired runs exited 0.

## Scope and limitations

Production retains the original scan algorithm. The private slice seam feeds the production selector and synthetic benchmark, avoiding an algorithm copy. Tests cover root fallback and closure behavior, global preselection, cycles, shared and missing dependencies, edge ordering, stable materialization through the existing linked-context tests, and selected/unselected factory call counts. No timing threshold is added to CI. Final repository-wide alignment and CI are owned by the integrating agent.
