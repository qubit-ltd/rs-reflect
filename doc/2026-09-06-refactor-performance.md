# rs-reflect refactor performance evidence

This report records the reproducible build measurement completed on 2026-09-07 with Rust 1.94.0. The raw reports are kept in the session evidence directory at `evidence/T8/measure-before/results.json` and `evidence/T8/measure-after/results.json`.

Both layouts were copied into independent temporary trees. Each sample used an independent Cargo target directory, `CARGO_INCREMENTAL=1`, and the fixed command:

```text
cargo +1.94.0 build --locked --release -p qubit-platform-testkit --bin model-inventory
```

Each layout ran three samples for clean/no-op, a behavior-preserving method edit, and a synthetic field edit. Every one of the 18 before and 18 after build steps succeeded, and both source trees retained their original whole-tree SHA-256 after measurement. The field fixture was applicable in both layouts; no business initializer was changed to accommodate the measurement.

| Scenario | Before median (s) | Before range (s) | After median (s) | After range (s) | After median binary bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| clean build | 125.485 | 109.262–182.896 | 76.821 | 69.728–170.259 | 6,441,176 |
| Cargo cached no-op | 0.234 | 0.087–0.310 | 0.079 | 0.064–0.283 | 6,441,176 |
| method edit warmup | 138.421 | 67.456–143.195 | 142.335 | 69.141–192.836 | 6,441,200 |
| method incremental build | 44.311 | 40.260–58.239 | 53.415 | 36.789–96.919 | 6,441,200 |
| field edit warmup | 158.174 | 116.111–189.763 | 81.219 | 70.845–277.109 | 6,441,184 |
| field incremental build | 63.248 | 60.626–221.639 | 43.313 | 40.679–117.173 | 6,445,424 |

The measurements do not establish a production optimization conclusion. The ranges are wide because each release build includes LTO and shared machine load; the before and after layouts also contain the intended source and dependency changes. They provide evidence that the refactor did not introduce a systematic build failure and that the synthetic incremental path remains measurable. The binary grew by roughly 3 KiB in this fixture, which requires a broader release-size study before attributing it to any single change.

The Rust benchmark targets were compiled with `--no-run` for `property_output` and `model_registry`. Runtime Criterion samples and allocator-separated platform runs remain separate from this build-cost report; no production implementation was changed from these measurements.

The runtime benchmark commands also completed successfully. `property_output` covered the scalar, string, optional, slice (0/1/32/1024/65536), conversion, and separate lookup+get groups; representative Criterion medians included scalar borrowed `15.847 ns`, scalar property get `98.510 ns`, scalar property get plus conversion `80.853 ns`, and lookup plus scalar get `82.600 ns`. The full raw Criterion output is retained in the Cargo target benchmark artifacts.

The platform `model_registry` run reported 131 linked models, cold link/projection `1,545,247 ns` with 10,504 allocation requests and 1,984,949 bytes, representative `TypeMetadata::try_of` at `4,054 ns/op` with 33 requests and 1,952 bytes, warm projection `428,422 ns/op`, and relationship validation `417,798 ns/op`. These counters are from the benchmark's counting allocator; they are useful for locating work but should not be compared directly with normal allocator wall-clock measurements.
