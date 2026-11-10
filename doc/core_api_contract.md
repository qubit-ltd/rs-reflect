# Core API Contract

This document is an English contract summary for the vNext public API. The detailed normative specification remains [the Chinese core API redesign](2026-10-20-core-api-redesign.zh_CN.md).

## Context ownership

`ReflectionContext::global()` provides the process-wide view. Applications can build explicit contexts to select linked declarations and registrations. A descriptor created with `TypeDescriptor::of_in::<T>(&context)` retains that context; fields, methods, generic definitions, and capability queries reached from it stay bound to the same view. Static declaration facts can be reused, while runtime indexes and registrations belong to their context.

## Lookup and type information

Singular lookup such as `field()` and `method()` succeeds only when exactly one matching declaration exists. Missing and ambiguous results remain structured errors. Collection access and explicit `lookup_*` methods provide optional or complete lookup states where appropriate.

A field's `type_expression()` preserves the source-level type shape, including generic parameters and wrappers. `find_concrete_type()` returns a descriptor only when the context resolves a concrete reflected type. Callers must handle `None`; a source expression is not proof that its concrete type is registered.

`TypeId`, descriptor addresses, and diagnostic Rust names are process-local identities. Persisted or exchanged identifiers are the application's responsibility.

## Checked operations and recovery

Typed access keeps owner and value types visible to Rust. Dynamic access checks exact runtime types. Failed updates return the rejected input with the error. Invocation failures before adapter execution retain receivers and arguments; once user code begins, reflection cannot roll back side effects. Panic handling is an explicit caller policy.

`TypeMismatch::expected()` and `actual()` return `std::any::TypeId`; `expected_name()` and `actual_name()` return diagnostic type names. The names are for display and are not stable identities.

The 2026-11-01 contract revision records the current `TypeMismatch` accessors and context selection behavior: explicit contexts aggregate selected types and selected binding targets, while malformed linked capabilities fail context construction.

## Capabilities

Capability queries specify both a capability type and its adapter contract through `CapabilitySpec`. Registrations target concrete types or generic definitions with typed adapters. A name or diagnostic string does not establish compatibility. Capability descriptors expose registration origin and remain scoped to the querying context.

## Generated-code boundary

There are three distinct support tiers:

1. **Application API:** application code should use the documented public descriptors, contexts, and typed registration APIs.
2. **Restricted generated-code protocol:** derive crates and downstream metadata generators may consume the versioned `__private::codegen_v4` protocol when they coordinate runtime and generator versions. This is protocol use, not ordinary application API use. Its supported expression symbols are limited to the inventory in [the architecture protocol section](architecture.md#generated-code-integration-protocol); that inventory reflects the built-in derive and audited consumers in `rs-model-metadata` and `rs-model-metadata-mm-implementation`.
3. **Other implementation details:** symbols outside the documented protocol inventory, including other `__private` and `codegen_v4` items, have no downstream compatibility promise.

Generated code checks `PROTOCOL_VERSION` with `protocol::assert_protocol_version`; a mismatch fails compilation. Keep the runtime and generator versions coordinated. Existing v4 protocol symbols remain available throughout the current release line. An incompatible protocol change requires a new versioned namespace and a coordinated upgrade of the runtime, derive crate, and affected downstream generators.

## Application responsibilities

Reflection supplies metadata and checked adapters. Applications remain responsible for parsing, authorization, business validation, persistence, transaction boundaries, and compensation for effects produced by called code.

See the [architecture](architecture.md), [English user guide](user_guide.md), and [中文用户手册](user_guide.zh_CN.md).
