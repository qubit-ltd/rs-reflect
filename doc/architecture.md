# Architecture

This document describes the vNext reflection path and the boundary available to application code. See the [English core API contract](core_api_contract.md) for a concise public contract summary; the [Chinese core API redesign](2026-10-20-core-api-redesign.zh_CN.md) remains the detailed normative specification. Generated metadata under `__private` is a versioned implementation protocol.

## From declarations to descriptors

`#[Reflect]` records structural facts from a type or trait declaration. `#[ReflectImpl]` records implementation facts and emits adapters for supported methods. `bind_method!` supplies concrete compile-time bindings for selected generic method specializations. Generated registrations are collected into a `ReflectionContext`, where conflicts are validated before aggregate indexes are published.

```mermaid
flowchart LR
  A[Reflect / ReflectImpl] --> B[Static metadata and adapters]
  C[bind_method!] --> D[Concrete generic binding]
  B --> E[Linked registrations]
  D --> E
  E --> F[ReflectionContext]
  F --> G[Context-bound descriptors]
  G --> H[Checked access, construction, and invocation]
```

The derive crate owns parsing, validation, and token generation. Its output uses `qubit_reflect::__private::codegen_v4`; application code should use the public descriptors and registration macros instead of depending on this protocol directly. `#[reflect(skip)]` is the only member helper attribute: skipped declarations remain in Rust but are omitted from generated reflection metadata.

## Generated-code integration protocol

The `__private::codegen_v4` namespace is a versioned integration protocol for derive crates and downstream metadata generators that coordinate their runtime and generator versions. Its supported expression surface is limited to `ExpressionData`/`TypeExpressionData`, `GenericExpressionArgument`, `TraitExpressionData`, `LifetimeExpression`, `ConstExpression`, `FunctionPointerData`, `TraitObjectData`, `OpaqueTypeData`, `PredicateData`, `TraitBoundModifier`, `GenericParameterMetadata`, `GenericArgumentMetadata`, `GenericDefinitionMetadata`, `TypeDefinitionMetadata`, `TypeDefinitionKind`, `StructKind`, `TypeExpression`, plus `type_expression_in` for binding static data to an explicit `ReflectionContext`. Compatibility checks use `PROTOCOL_VERSION` and `protocol::assert_protocol_version`. This list reflects symbols emitted by the built-in derive and consumed by the audited downstream derive; it does not promise other `expression` or `codegen_v4` items. This is not a general application API, and other codegen modules are not thereby promised as supported. Keep runtime and derive versions aligned. An incompatible protocol change requires a new versioned namespace and coordinated updates to the derive and downstream generators; v4 symbols remain available throughout this release line.

Composite descriptors retain their complete diagnostic name for display. Context short-name indexes keep the historical final-segment alias, so a composite display name is not itself a lookup key. This preserves existing short-name lookup behavior while keeping the full type shape visible.

## Context and descriptor ownership

`ReflectionContext::global()` is a process-wide, lazily initialized context. `ReflectionContext::builder()` creates an independent context for explicit registration and isolation. The context validates registration facts as a whole; a failed build does not expose a partial context.

Linked registration callbacks run as part of global context initialization, outside the context state lock. A callback must not query or initialize `ReflectionContext::global()` recursively: the runtime panics with `recursive global reflection context initialization` instead of waiting on the same thread. Keep callback work limited to producing static registration data; move context queries to application code after initialization, or use a separate explicit context when the callback needs to inspect registrations.

Descriptors contain static declaration metadata and a borrow of their context. A field or method reached from a descriptor therefore uses that same context. Callers do not pass a second registry to method lookup or invocation. This prevents a method resolved in one context from being invoked against another context's adapters.

Fields and variants are local declaration facts, so their views do not require aggregate context initialization. Methods, type-name discovery, trait and impl applications, and capabilities use context data. `field()` and `method()` require one result and return structured lookup errors; `fields().get()` is optional, while `lookup_method()` exposes Missing, Unique, or Ambiguous.

## Checked operations and recovery

Typed field access makes the target and field types explicit at the call site. Dynamic values are used when a framework has erased those types; the runtime checks the exact Rust type before accessing or replacing a field. A failed dynamic downcast or replacement retains the original value for recovery.

Invocation validates specialization, receiver, argument count, argument modes, and exact types before entering generated user code. Failures at this stage preserve receiver and argument inputs. Once an adapter begins, reflection cannot roll back user side effects. Panics propagate through ordinary invocation; panic capture is an explicit call-site policy.

The default value mode is `Local`. For reflected structs and enums, `#[Reflect(local_only)]` explicitly keeps generated field and construction adapters local when fields are not `Send + Sync`; the option does not change the default behavior of bare `#[Reflect]`. Local values remain non-transferable across threads, and `#[ReflectImpl]` method registration is independent of this type option. ThreadSafe adapters are generated only by paths whose Rust bounds establish that contract.

## Generic methods and capabilities

Runtime type identities cannot ask rustc to generate a new monomorphization. A generic method declaration is reflected independently from its concrete callable specializations. Applications register the specializations they support at compile time and choose one through its typed generic arguments.

Capability queries are also context-bound. Their contract includes the capability type and the adapter type, so a matching display name alone is not enough to call an adapter. Built-in capabilities rely on Rust trait bounds; custom providers use the typed registration API.

## Identity and application responsibilities

`TypeId`, descriptor addresses, context IDs, and diagnostic Rust names are process-local. They are not storage or wire-format identifiers. Applications that persist or exchange reflection references need their own stable identifiers and versioning rules.

The library provides metadata, checked dynamic operations, context validation, and generated call adapters. Applications remain responsible for input parsing, authorization, business invariants, persistence, concurrency policy, and compensation after user code has run.

## Migration notes

- Explicit contexts now include only selected types and explicitly selected binding targets. If an application relied on unrelated linked registrations appearing through catalog leakage, select the required target with the context builder.
- Invalid linked capability registrations are reported while the context is built. Handle that `ContextError` instead of relying on a later lookup to hide the invalid registration.
- A reflected type with non-`Send + Sync` fields can use `#[Reflect(local_only)]` to retain local field access and construction. This does not make the value thread-safe; `#[reflect(skip)]` remains available when a member should be omitted entirely.
- Rc and Arc descriptors now expose their pointee generic argument; downstream production code does not need a migration for this metadata addition.
- For pinned generic methods, select the exact specialization with `specialization(&MethodArguments)` and call `MethodSpecializationDescriptor::invoke_pinned`. Rejected calls retain the pinned receiver and arguments. Bindings such as `echo<T>(Pin<&mut Self>, T) -> T` are discoverable, but invoking one remains unavailable while generic parameter metadata cannot identify the concrete argument type.

## Further reading

- [English core API contract](core_api_contract.md) summarizes the public contract.
- [Core API redesign](2026-10-20-core-api-redesign.zh_CN.md) is the detailed normative API specification.
- [User guide](user_guide.md) follows a field update and method invocation from an application perspective.
- [中文架构](architecture.zh_CN.md)
- [中文用户手册](user_guide.zh_CN.md)
