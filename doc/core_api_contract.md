# Core API Contract (English Summary)

This document summarizes the application-facing vNext contract of `qubit-reflect` 0.2.0. The detailed normative specification remains [the core API redesign in Chinese](2026-10-20-core-api-redesign.zh_CN.md). Public Rust signatures and Rustdoc are authoritative when details change.

## Context ownership

`ReflectionContext::global()` provides the lazily initialized process-wide context. `ReflectionContext::builder()` creates an independently owned context; include linked registrations with `include_linked()` and add explicit registrations with `register(...)` or the builder's typed helpers before `build()`. A failed build returns an error and does not expose a partially built context.

Static declaration facts can be retained independently. Runtime handles such as `TypeDescriptor<'ctx>`, field and method descriptors, type-definition descriptors, `TypeExpression<'ctx>`, and `Capability<'ctx, C>` are tied to the context borrow. Keep handles within that context's lifetime and do not reuse a handle as if it belonged to another context. `TypeDescriptor::of::<T>()` uses the global context; `context.type_of::<T>()` uses the supplied context.

## Strict lookup and recovery

Singular accessors such as `field(name)`, `method(name)`, and `type_by_name(name)` require an unambiguous match and return structured errors for missing or ambiguous names (and context initialization errors where applicable). Use optional collection lookup or the `lookup_*` APIs when absence or ambiguity is a normal branch. `Lookup::Missing`, `Lookup::Unique`, and `Lookup::Ambiguous` let the caller decide what action to take.

Typed operations preserve Rust types at the call site. Dynamic field operations check exact Rust type identity. A failed field replacement returns a `FieldSetFailure` containing the diagnostic and rejected value. Invocation validates receiver, arguments, modes, and selected specialization before generated user code starts; `InvocationFailure::into_parts()` retains the diagnostic and recoverable inputs. Once user code starts, reflection does not roll back its side effects. Ordinary invocation does not imply panic recovery; use a catching API only under an application-defined panic policy.

## Type expressions and concrete descriptors

`TypeExpression<'ctx>` represents reflected type syntax, including generic, associated, and opaque forms. It does not promise that the expression denotes a concrete reflected Rust type. Use `find_concrete_type()` when a concrete descriptor is optional, or the corresponding required accessor when absence is an error. Do not infer callability or type identity from a display name. `TypeId` and diagnostic Rust names are process-local and are not persistence or wire-format identifiers.

## Typed capabilities and provenance

Custom capabilities implement `CapabilitySpec`, whose identifier and associated `Adapter` type form the contract. Register an adapter for a concrete type with `CapabilityRegistration::for_type::<T, C>(adapter)` or for a reflected definition with `for_definition::<C>(definition, adapter)`, then add it to a `ReflectionContextBuilder`. Query through a context-bound descriptor using `find_capability::<C>()`, `has_capability::<C>()`, or `capability::<C>()`. Handle missing capability and registration/context errors explicitly. The returned typed capability exposes its adapter through the contract and reports its registration provenance with `origin()`.

A matching textual ID alone is insufficient: capability type and adapter type must agree. Built-in capabilities also rely on their Rust trait bounds. Registration conflicts are reported during context construction rather than silently choosing one provider.

## Generated-code boundary

Derive output targets `qubit_reflect::__private::codegen_v4`. That module is a versioned implementation protocol for generated code, not an application extension API. Applications should use public descriptors, `ReflectionContextBuilder`, `CapabilityRegistration`, registration macros, and `bind_method!`. No compatibility guarantee is made for direct dependencies on `__private`; do not pin application logic to generated internals.

## Application responsibilities

Reflection supplies metadata, checked operations, context validation, and generated adapters. The application remains responsible for parsing external input, authorization, business validation, repository loading and persistence, transaction policy, concurrency policy, and compensation for side effects after user code begins.

## Further reading

- [English user guide](user_guide.md) · [中文用户手册](user_guide.zh_CN.md)
- [English architecture](architecture.md) · [中文架构](architecture.zh_CN.md)
- [中文核心 API 设计规范](2026-10-20-core-api-redesign.zh_CN.md)
