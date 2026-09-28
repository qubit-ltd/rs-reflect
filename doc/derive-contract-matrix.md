# Derive contract and coverage matrix

This matrix separates compiler-facing macro contracts from runtime coverage. `scripts/check-derive-coverage.sh` runs all targets for both `qubit-reflect-derive` and the `qubit-reflect` consumer package, so its profile includes derive unit tests and proc-macro invocations exercised by the consumer's integration/UI tests. The report only aggregates files under the derive crate's configure, parse, validate, and expand stages. Its line-coverage floors are configure 70%, parse 85%, validate 80%, and expand 85%. Missing or unexecuted stages and stages below their floor fail the report.

| Contract | Success/failure fixture | Feature or configuration | Expected result |
| --- | --- | --- | --- |
| Conditional trait and impl members | `tests/descriptor/conditional_compilation_tests.rs`; `tests/ui/pass/conditional_compilation_tests.rs`; `test-crates/conditional-feature` | host target, feature disabled/enabled, `cfg(any())`, active pointer-width `cfg`, `cfg_attr` | Disabled items disappear; active `no_invoke` facts remain without an invocation adapter. |
| Invalid helper on a disabled member | conditional compilation UI pass fixture | helper under `cfg(any())` | Disabled helper is not semantically validated. |
| Invalid helper on an active member | existing `tests/ui/fail/` helper diagnostics | default features | Compilation fails at the source helper. |
| Signature and invocation policies | `tests/ui/fail/` invocation, unwind, and thread-safety fixtures | all-features CI | Unsupported signatures fail with source-oriented diagnostics; supported policies compile. |
| Generic specialization | `tests/ui/fail/generic_impl_unsatisfied_specialization_tests.rs`; integration fixtures | `generic-models` feature | Invalid specialization fails; registered concrete specialization preserves method/adapter mapping. |
| Facade and renamed/direct dependencies | `test-crates/model-facade-*`; `test-crates/macro-runtime-only` | facade; direct derive with runtime defaults disabled | Internal support macro resolves through the documented protocol. |
| Runtime-only feature boundary | `test-crates/macro-runtime-only` | no runtime default features | Direct derive macro remains usable without enabling runtime derive exports. |

The coverage collector reports per-stage source files, executed line counts, percentages and configured floors, HTML, the raw llvm-cov JSON, and a machine-readable summary under `target/derive-coverage/`. Passing a floor does not prove every macro expansion path or replace the UI contract fixtures.
