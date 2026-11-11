# RF-002 registration selection measurement

## Decision

Production keeps the original scan selector. Corrected adversarial reverse-chain and branch fixtures show strong depth growth at 512 fragments, but the indexed candidate was not retained: corrected paired runs consistently showed about 9% higher actual-inventory phase cost and about 50–55% higher 32-fragment chain selection cost; full linked-build results pointed in opposite directions between runs. The candidate therefore did not establish a reliable overall improvement within the stated small-case and total-cost gates.

The actual-inventory phase and full-builder values below use the same linked inventory and process conditions, but the former includes inventory collection, sorting, and selection while the latter also includes fallback, validation, factory materialization, and context building. Their ratio is an upper bound on selection-related work, not a pure selector share.

## Fixture correction

Review found that the previously committed reverse-chain ordering did not cause repeated passes. The initial rotation left targets ascending after the root and allowed propagation in one pass. All reverse-chain and branch measurements from that fixture, including their candidate comparisons, are invalid and superseded here. The corrected helper reverses fragment order so active targets descend, root target 0 is last, and each dependency fragment precedes its source. The builder checks the ordering invariants outside the timed closure; a unit test checks descending targets and root-last ordering. Candidate paired measurements below were rerun on this corrected fixture.

## Environment and method

- Baseline repository commit: `18a0717` (`fix(derive): honor HRTB trait binders in lifetime analysis`).
- Toolchain: `rustc 1.94.0 (4a4ef493e 2026-03-02)`, `cargo 1.94.0`.
- Host: x86_64 Linux, Intel Xeon E5-2680 v4 @ 2.40 GHz, 2 sockets / 28 physical cores / 56 logical CPUs; shared-host build load varied across rounds.
- Each Criterion command used 10 samples, 100 ms warmup, and 100 ms measurement target. Fixtures are created outside timed closures and selector results pass through `black_box`.
- Matrix: 32/128/512 fragments, depth 1/8/32, and chain, reverse chain, branch, cycle, shared dependency, and unrelated graphs.
- First explicit linked build is a single process observation; repeated builder measurements construct a fresh builder/context per iteration.
- Full logs: `/tmp/rf002-fixed-run1.log`, `/tmp/rf002-fixed-run2.log`, and `/tmp/rf002-fixed-run3-valid.log`. The last run completed with captured exit code 0. The earlier `/tmp/rf002-fixed-run3.log` had no captured exit status and is not counted.

## Baseline build and linked-inventory measurements

Command for all three valid runs (exit 0 each):

```text
cargo bench --locked --bench registry_init --features bench-internals,derive -- --sample-size 10 --measurement-time 0.1 --warm-up-time 0.1
```

| Measure | Run 1 | Run 2 | Run 3 |
|---|---:|---:|---:|
| First explicit build, one observation (µs) | 44.807 | 42.511 | 54.556 |
| Repeated fresh-builder total median (µs) | 7.9910 | 7.6043 | 8.0904 |
| Linked inventory collect + sort + selection median (µs) | 3.9917 | 4.0832 | 4.2494 |
| Phase / full-build ratio | 50.0% | 53.7% | 52.5% |

## Corrected synthetic selector medians

Criterion median estimates in microseconds. This table replaces the earlier invalid reverse-chain/branch entries. Logs retain Criterion intervals and outlier details.

| Graph / fragments / depth | Run 1 median (µs) | Run 2 median (µs) | Run 3 median (µs) |
|---|---:|---:|---:|
| `chain/32/1` | 1.4968 | 1.4618 | 1.3902 |
| `chain/32/8` | 1.8983 | 1.9311 | 1.8899 |
| `chain/32/32` | 3.3147 | 3.2589 | 3.3728 |
| `chain/128/1` | 5.2902 | 5.3113 | 5.1995 |
| `chain/128/8` | 6.1870 | 5.7764 | 6.0872 |
| `chain/128/32` | 7.3709 | 7.3114 | 7.3390 |
| `chain/512/1` | 20.941 | 20.321 | 20.321 |
| `chain/512/8` | 21.701 | 21.614 | 21.572 |
| `chain/512/32` | 23.112 | 23.366 | 23.267 |
| `reverse_chain/32/1` | 1.4022 | 1.4154 | 1.4036 |
| `reverse_chain/32/8` | 6.0314 | 6.1129 | 6.0702 |
| `reverse_chain/32/32` | 15.001 | 15.184 | 15.092 |
| `reverse_chain/128/1` | 5.2022 | 5.1625 | 5.1903 |
| `reverse_chain/128/8` | 23.378 | 23.339 | 24.035 |
| `reverse_chain/128/32` | 80.646 | 86.700 | 80.393 |
| `reverse_chain/512/1` | 20.289 | 20.279 | 20.403 |
| `reverse_chain/512/8` | 92.270 | 92.653 | 92.201 |
| `reverse_chain/512/32` | 340.65 | 343.63 | 380.30 |
| `branch/32/1` | 1.3828 | 1.4151 | 1.3944 |
| `branch/32/8` | 4.4293 | 4.7763 | 4.4854 |
| `branch/32/32` | 10.128 | 10.191 | 10.109 |
| `branch/128/1` | 5.3240 | 5.1563 | 5.2139 |
| `branch/128/8` | 15.976 | 15.918 | 16.488 |
| `branch/128/32` | 60.512 | 45.848 | 45.800 |
| `branch/512/1` | 20.868 | 20.461 | 20.333 |
| `branch/512/8` | 62.030 | 62.108 | 63.119 |
| `branch/512/32` | 186.78 | 223.87 | 272.28 |
| `cycle/32/1` | 1.4368 | 1.4356 | 1.4784 |
| `cycle/32/8` | 1.9114 | 1.9631 | 1.9282 |
| `cycle/32/32` | 3.3363 | 3.3490 | 3.3650 |
| `cycle/128/1` | 5.2689 | 5.3299 | 5.2571 |
| `cycle/128/8` | 5.8524 | 5.9541 | 6.1125 |
| `cycle/128/32` | 7.9169 | 7.3278 | 7.3310 |
| `cycle/512/1` | 20.972 | 20.426 | 20.845 |
| `cycle/512/8` | 21.593 | 21.610 | 21.593 |
| `cycle/512/32` | 23.415 | 23.413 | 23.434 |
| `shared/32/1` | 1.4278 | 1.3830 | 1.3946 |
| `shared/32/8` | 1.4265 | 1.3931 | 1.4083 |
| `shared/32/32` | 1.3864 | 1.3860 | 1.4052 |
| `shared/128/1` | 5.2154 | 5.2892 | 5.1725 |
| `shared/128/8` | 5.3032 | 5.1708 | 5.3132 |
| `shared/128/32` | 7.6937 | 5.2313 | 5.1686 |
| `shared/512/1` | 20.915 | 20.401 | 20.288 |
| `shared/512/8` | 20.298 | 20.320 | 20.780 |
| `shared/512/32` | 20.778 | 21.166 | 20.376 |
| `unrelated/32/1` | 1.4224 | 1.3922 | 1.4034 |
| `unrelated/32/8` | 1.3882 | 1.3888 | 1.4000 |
| `unrelated/32/32` | 1.4204 | 1.3844 | 1.4242 |
| `unrelated/128/1` | 5.3240 | 5.2119 | 5.2979 |
| `unrelated/128/8` | 5.2765 | 5.1965 | 5.3612 |
| `unrelated/128/32` | 5.2724 | 5.1810 | 5.1670 |
| `unrelated/512/1` | 20.880 | 20.566 | 20.277 |
| `unrelated/512/8` | 20.787 | 21.131 | 20.877 |
| `unrelated/512/32` | 20.389 | 22.499 | 20.307 |

At 512 fragments, reverse-chain medians were 20.289 / 20.279 / 20.403 µs at depth 1, 92.270 / 92.653 / 92.201 µs at depth 8, and 340.65 / 343.63 / 380.30 µs at depth 32. Branch medians were 20.868 / 20.461 / 20.333 µs, 62.030 / 62.108 / 63.119 µs, and 186.78 / 223.87 / 272.28 µs respectively. This confirms repeated-scan depth cost for corrected adverse orders, with run-to-run spread increasing for branch depth 32. The ordinary forward chain changes little with depth.

## Corrected candidate paired exploration

Candidate comparisons were measured adjacent in one Criterion process with the same fixtures and settings. The indexed candidate code has been removed; it is reported only as an experiment. Corrected paired logs: `/tmp/rf002-candidate-fixed-run1.log` and `/tmp/rf002-candidate-fixed-run2.log`, both exit 0.

| Measure, median (µs) | Run 1 scan / indexed | Run 2 scan / indexed |
|---|---:|---:|
| Full linked build | 7.8906 / 8.7552 | 15.493 / 8.7806 |
| Actual inventory phase | 4.3533 / 4.7741 | 4.3651 / 4.7461 |
| 512 reverse chain, depth 32 | 319.43 / 29.545 | 318.43 / 28.366 |
| 512 branch, depth 32 | 174.48 / 28.804 | 174.62 / 29.313 |
| 32 chain, depth 1 | 1.3260 / 1.9889 | 1.2983 / 2.0104 |

The candidate greatly reduced synthetic adverse-case time, but its actual phase was about 9% slower in both runs, and its small chain case was about 50–55% slower. The full-build comparison changed direction under host noise. Given those results, total production benefit is not established; the candidate is removed and the original scan remains.

An earlier candidate exploration ran during substantial shared-host contention, and one exploratory matrix was stopped with exit 130. Those data and all candidate comparisons using the defective fixture are excluded. No cold-construction candidate comparison is claimed; the single first-build values above are baseline observations only.

## Focused verification

- `cargo test --locked -p qubit-reflect --lib runtime::context` — exit 0, 34 passed.
- `cargo test --locked -p qubit-reflect --lib reverse_chain_and_branch_use_descending_target_order` — exit 0, 1 passed.
- `cargo test --locked -p qubit-reflect --lib runtime::tests::context` — exit 0, 20 passed.
- `cargo bench --locked --bench registry_init --features bench-internals,derive --no-run` — exit 0, benchmark executable built.
- `rustfmt --edition 2024 src/runtime/context/builder.rs src/private/codegen_v4/registration.rs benches/registry_init.rs src/runtime/tests/context.rs` — exit 0.
- `rustfmt --edition 2024 --check src/runtime/context/builder.rs src/private/codegen_v4/registration.rs benches/registry_init.rs src/runtime/tests/context.rs` — exit 0 after formatting.
- `git diff --check` — exit 0.

An earlier repository-wide `cargo fmt --all -- --check` exited 1 on existing formatting differences in unrelated files. The scoped format check above passes.

Production behavior remains the original selector. The private selector seam is shared by production and synthetic benchmarks. Existing tests cover global preselection, cycles, shared and missing dependencies, stable edge/materialization order, fallback, and factory behavior. No timing thresholds were added to CI.
