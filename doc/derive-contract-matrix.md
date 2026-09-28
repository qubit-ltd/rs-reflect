# Derive contract and coverage matrix

This matrix separates compiler-facing macro contracts from runtime coverage. The derive line percentages are observations from `scripts/check-derive-coverage.sh`; they have no percentage threshold yet. A missing or unexecuted configure/parse/validate/expand stage fails the report instead of being reported as full coverage. `cargo-llvm-cov` instruments derive library tests; UI tests remain the evidence for proc-macro behavior executed by rustc.

| Contract | Success/failure fixture | Feature or configuration | Expected result |
| --- | --- | --- | --- |
| Conditional trait and impl members | `tests/descriptor/conditional_compilation_tests.rs`; `tests/ui/pass/conditional_compilation_tests.rs`; `test-crates/conditional-feature` | host target, feature disabled/enabled, `cfg(any())`, active pointer-width `cfg`, `cfg_attr` | Disabled items disappear; active `no_invoke` facts remain without an invocation adapter. |
| Invalid helper on a disabled member | conditional compilation UI pass fixture | helper under `cfg(any())` | Disabled helper is not semantically validated. |
| Invalid helper on an active member | existing `tests/ui/fail/` helper diagnostics | default features | Compilation fails at the source helper. |
| Signature and invocation policies | `tests/ui/fail/` invocation, unwind, and thread-safety fixtures | all-features CI | Unsupported signatures fail with source-oriented diagnostics; supported policies compile. |
| Generic specialization | `tests/ui/fail/generic_impl_unsatisfied_specialization_tests.rs`; integration fixtures | `generic-models` feature | Invalid specialization fails; registered concrete specialization preserves method/adapter mapping. |
| Facade and renamed/direct dependencies | `test-crates/model-facade-*`; `test-crates/macro-runtime-only` | facade; direct derive with runtime defaults disabled | Internal support macro resolves through the documented protocol. |
| Runtime-only feature boundary | `test-crates/macro-runtime-only` | no runtime default features | Direct derive macro remains usable without enabling runtime derive exports. |

The coverage collector reports per-stage source files, executed line counts, percentages, HTML, the raw llvm-cov JSON, and a machine-readable summary under `target/derive-coverage/`. It does not claim that coverage percentages prove every macro expansion path or replace the UI contract fixtures.
