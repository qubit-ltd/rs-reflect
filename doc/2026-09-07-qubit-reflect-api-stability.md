# `qubit-reflect` API Stability

- Date: 2026-09-07
- Status: normative internal compatibility policy
- Scope: the reflection runtime, derive output, extension capabilities, and implementation details

This policy makes compatibility expectations explicit for the four audiences that
consume or extend `qubit-reflect`. A path in the table is representative: the
stated boundary, rather than a filename alone, determines whether an item is
stable.

| Layer | Representative paths | Compatibility promise | Allowed changes | Migration requirement |
| --- | --- | --- | --- | --- |
| Stable Application | `src/lib.rs`, `src/descriptor/`, `src/value/`, `tests/descriptor/` | Public descriptors, typed views, checked operations, structured error categories, and documented macro entry points remain source-compatible within a minor release line. Stable identities are process-local unless the API explicitly says otherwise. | Additive public APIs, new typed views, new error detail, and bug fixes that preserve safety and existing valid results. | Applications must match on public categories rather than Display text, keep `type_name()`/`TypeId` out of persisted protocols, and adopt deprecations before the next breaking line. |
| Extension Author | `src/capability/`, `src/identity/capability_id.rs`, `tests/descriptor/capability_tests.rs` | Namespaced `CapabilityId`, typed `CapabilityKey`, adapter contracts, and deterministic capability enumeration are the extension boundary. An extension never receives unchecked access to private descriptor state. | Add new namespaced capabilities and safe adapters; add diagnostics and opt-in helpers without changing existing IDs or adapter contracts. | Keep IDs permanently owned by the extension, preserve adapter type contracts, and publish a compatibility note when an adapter or capability is superseded. |
| Versioned Codegen | `derive/src/`, `src/private/codegen_v3/`, `derive/tests/`, `test-crates/model-facade-app/` | Generated code and its private protocol are versioned together. A protocol such as `__private::codegen_v3` is compatible only with the runtime generation that declares it; public macro behavior remains covered by compile-pass/fail tests. | Introduce a new private protocol generation, improve diagnostics, and add generated metadata while retaining an old generation for an announced migration window. | Re-run derive and downstream facade tests, update the pinned runtime/derive pair, and migrate generated artifacts when the protocol generation changes. |
| Internal | `src/private/`, `src/registry/interner.rs`, `tests/internal/`, `benches/` | Internal caches, interning, registration plumbing, benchmark layout, and test-only helpers carry no downstream compatibility promise. They must still preserve documented public safety and determinism. | Refactor, split, replace, or remove internals when public behavior and safety contracts remain intact. | No consumer migration is required; maintainers must update internal tests, benchmarks, and traceability evidence in the same change. |

## Reading this policy

The Stable Application boundary is the default for users. Extension authors
must treat capability IDs and adapter types as their own compatibility surface,
while Versioned Codegen explicitly permits a coordinated runtime/derive break.
Internal names may change freely, but a change that leaks into a public path or
generated token stream must be reviewed at the corresponding higher boundary.

The requirements and traceability matrices record the executable evidence for
these boundaries. In particular, `REQ-TYPE-030` requires strict capability
lookups to preserve missing, fact-only, adapter-mismatch, and found states;
legacy `Option` convenience methods may still intentionally collapse them.
