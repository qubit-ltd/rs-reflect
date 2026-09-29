# `qubit-reflect` API Stability

<!-- reflect-contract: dispatch.input_recovery=original-input -->

- Date: 2026-09-07
- Status: internal `0.1` source-development compatibility policy; registry publication is not verified here
- Scope: the reflection runtime, derive output, extension capabilities, and implementation details

This policy makes compatibility expectations explicit for the four audiences that
consume or extend `qubit-reflect`. This policy treats the internal `0.1` source-development line as requiring
coordinated breaking changes. Until a stable release policy is explicitly
announced, changes may break the boundaries below, provided the runtime, derive crate, downstream
facades, and their contract tests are updated together. A path in the table is
representative: the stated boundary, rather than a filename alone, determines
the intended stability level. A migration window or continued support for an
old protocol is promised only when the policy for a future stable release line
explicitly announces it.

| Layer | Representative paths | Compatibility promise | Allowed changes | Migration requirement |
| --- | --- | --- | --- | --- |
| Stable Application | `src/lib.rs`, `src/descriptor/`, `src/value/`, `tests/descriptor/` | In an announced stable release line, public descriptors, typed views, checked operations, structured error categories, and documented macro entry points remain source-compatible within a minor release line. Stable identities are process-local unless the API explicitly says otherwise. | Before a public stable release, coordinated breaking changes are allowed; in a stable line, additive public APIs, new typed views, new error detail, and bug fixes must preserve safety and existing valid results. | Applications must match on public categories rather than Display text, keep `type_name()`/`TypeId` out of persisted protocols, and adopt deprecations before the next breaking line. |
| Extension Author | `src/capability/`, `src/identity/capability_id.rs`, `tests/descriptor/capability_tests.rs` | Namespaced `CapabilityId`, typed `CapabilityKey`, adapter contracts, and deterministic capability enumeration are the extension boundary. An extension never receives unchecked access to private descriptor state. | Before a public stable release, coordinate changes to existing IDs or adapter contracts with downstream users; in a stable line, add new namespaced capabilities, safe adapters, diagnostics, and opt-in helpers without changing existing contracts. | Keep IDs owned by the extension, preserve adapter type contracts within an announced stable line, and publish a compatibility note when an adapter or capability is superseded. |
| Versioned Codegen | `derive/src/`, `src/private/codegen_v3/`, `derive/tests/`, `test-crates/model-facade-app/` | Generated code and its private protocol are versioned together. A protocol such as `__private::codegen_v3` is compatible only with the runtime generation that declares it; public macro behavior remains covered by compile-pass/fail tests. | Before a public stable release, a coordinated change may replace or remove an old generated-code protocol without a compatibility shim. Retain an old generation for a migration window only when the release policy explicitly announces one. | Update the runtime, derive crate, downstream `rs-model-metadata` facade, and their contract tests together; re-run derive and downstream facade tests, update the pinned runtime/derive pair, and migrate generated artifacts when the protocol generation changes. |
| Internal | `src/private/`, `src/registry/interner.rs`, `tests/internal/`, `benches/` | Internal caches, interning, registration plumbing, benchmark layout, and test-only helpers carry no downstream compatibility promise. They must still preserve documented public safety and determinism. | Refactor, split, replace, or remove internals when public behavior and safety contracts remain intact. | No consumer migration is required; maintainers must update internal tests, benchmarks, and traceability evidence in the same change. |

## 2026-09-29: breaking invocation return types

This change is source-breaking. `descriptor::InvocationAdapter` and
`MethodInstanceDescriptor` each change these six consuming methods from outer
`Option` to `InvocationDispatchResult<I, R>` (twelve methods in total):
`invoke_local`, `invoke_thread_safe`, `invoke_catching_local`,
`invoke_catching_thread_safe`, `invoke_pinned_ref_local`, and
`invoke_pinned_mut_local`. Existing `None`/`Some` matches and `Option` helpers must
migrate; this is not an additive compatibility shim.

Outer `Err(InvocationUnavailable<I>)` retains the complete original input before
binding or execution; `reason()` reports the dispatch reason and
`into_invocation()` or `into_parts()` retrieves the input. Outer `Ok` retains the
existing validation and method-panic result layers: ordinary validation failures
still use `InvocationRecovery` or pinned recovery, and catching still separates
`InvocationPanic` from validation. Pinned receiver `T` must match exactly; a
mismatch preserves the original `Pin` and caller-ordered bindings. Consumed
method inputs and side effects are not restored after entry.

The runtime function-pointer aliases (including `invoke::InvocationAdapter`),
generated adapter ABI, `__private::codegen_v3`, model ABI v7, and
`definition_provider_v2` remain unchanged. The descriptor adapter type and the
runtime function-pointer alias are distinct APIs. This note describes the source
checkout and does not verify registry publication of the candidate.

## Reading this policy

The Stable Application boundary is the default for users. Extension authors
must treat capability IDs and adapter types as their own compatibility surface,
while Versioned Codegen explicitly permits a coordinated runtime/derive/facade break.
Internal names may change freely, but a change that leaks into a public path or
generated token stream must be reviewed at the corresponding higher boundary.

The requirements and traceability matrices record the executable evidence for
these boundaries. In particular, `REQ-TYPE-030` requires strict capability
lookups to preserve `Missing`, `FactOnly`, `AdapterTypeMismatch`, and `Found`;
fallible convenience queries map `Missing` to `Ok(None)` and `Found` to
`Ok(Some(adapter))`, while `FactOnly` and `AdapterTypeMismatch` remain errors. The
registry's `capability_origin` and `capability_source` APIs preserve whether a
fact is intrinsic or came from a registration fragment.
