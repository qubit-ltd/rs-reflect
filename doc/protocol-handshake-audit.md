# RF-004 protocol handshake audit

## Conclusion

No missing handshake was demonstrated in either `rs-model-metadata` or `rs-model-metadata-mm-implementation`. Their model outputs reach `qubit_reflect::Reflect` through the resolved runtime facade, and the proc-macro expansion for `Reflect` unconditionally emits the codegen v4 protocol assertion. `ModelImpl` outputs similarly retain `#[runtime::__private::ReflectImpl]`, whose expansion path is part of the reflection codegen. The generic metadata and field-expression tokens are emitted alongside the reflected type declaration, rather than as an independent macro output path. Recommendation: no production change and no additional duplicated assertion.

## Output class matrix

| Output class | `rs-model-metadata` | `rs-model-metadata-mm-implementation` | Handshake evidence |
| --- | --- | --- | --- |
| Type declaration | `derive/src/expand/pipeline.rs:64-85` appends `#[runtime::__private::Reflect]` to the transformed type. | Same implementation at `derive/src/expand/pipeline.rs:64-85`. | `rs-reflect/derive/src/expand/dispatcher.rs:72-75` wraps every `dispatch_with_context` expansion in `const _: () = facade::__private::codegen_v4::protocol::assert_protocol_version(4);`. |
| Generic metadata | Generated from `expand_metadata` in the same pipeline output. Generic declarations run under `runtime::__private::v8::with_generic_feature!`; `expand_generic_registration` emits registration providers in that output. | Same path; `derive/src/expand/metadata.rs:327-407` shows generic metadata/capability registration. | This code is not emitted from a separate proc-macro invocation: pipeline emits it together with the type carrying `Reflect`. The reflection derive's assertion is unconditional, including generic declarations. Registration helper macros consume `runtime::__private::codegen_v4`, which is the same facade re-export. |
| Field expressions | `derive/src/expand/fields.rs:89-125` constructs `runtime::__private::codegen_v4::expression::*` data while `expand_metadata` builds that type's metadata. | Same path; field expression generation uses the runtime path passed from pipeline. | These expressions are metadata providers attached to the reflected declaration's pipeline output; no standalone field-expression output route was found. Therefore the enclosing `Reflect` expansion supplies the assertion before this generated output is accepted. |
| Impl output | `derive/src/expand/model_impl.rs:142-148` retains the impl with `#[runtime::__private::ReflectImpl]`; generated property adapters/providers are emitted from the same macro invocation. | Same path. | `ReflectImpl` is a re-export of `qubit_reflect::ReflectImpl` in `src/__private.rs:13-17`. The proc-macro's reflection codegen path is the owning protocol boundary. No model-impl branch directly constructs an independent v4 expression without the retained attribute. |

## Runtime facade and dependency renaming

Both generators resolve the runtime crate once in `derive/src/expand/pipeline.rs:58-80` and pass that token path into metadata, field, and model-impl expansion. `rs-model-metadata/derive/src/runtime_path.rs:20-47` documents and implements `proc_macro_crate::crate_name` resolution, including Cargo-renamed dependencies (using the canonical crate alias for the runtime package itself); the mm implementation has the same function and logic. Generated paths consistently use `#runtime::__private::...`. The handshake is emitted by the resolved facade's `Reflect` expansion, rather than a hard-coded `qubit_reflect` path in downstream output.

The target reflection derive has a focused unit test asserting that the protocol path uses a renamed facade: `rs-reflect/derive/src/expand/dispatcher.rs:132-150`. Thus the indirect assertion is covered for renamed facade expansion as well as the normal code path.

## Existing consumer coverage and verification

Both metadata repositories have `tests/facade_ui_tests.rs`, which discovers `tests/ui/pass/*.rs` and `tests/ui/fail/*.rs`; the mm worktree already contains user changes, which were left untouched. This audit was source-path based and did not run tests: the evidence is the unconditional wrapper in `dispatch_with_context`, the downstream pipeline's guaranteed `Reflect` insertion, and the `ReflectImpl` retained attribute, not grep absence. A deliberately mismatched fake runtime fixture is not warranted absent an independent output route; adding one would test a synthetic route rather than an observed gap.

Consumer verification:

- In `/data/working/qubit/rust-common/rs-model-metadata`, `cargo test --locked --test facade_ui_tests --all-features` exited 0. The facade harness passed, including its pass fixture and three expected compile-fail fixtures.
- In `/data/working/qubit/rust-common/rs-model-metadata-mm-implementation`, the same command exited 0 with the same facade harness and fixtures passing. The repository's pre-existing dirty files were preserved.

The audit also used `rg`/`nl` source inspection in both downstream repositories and `rs-reflect`. No generated-output convergence file is affected, so no isolated regression or implementation worktree is proposed.
