# `qubit-reflect` Evolution History

- Date: 2026-09-07
- Related design: [Design](2026-09-03-qubit-reflect-design.md) · [简体中文设计](2026-09-03-qubit-reflect-design.zh_CN.md)

This record preserves dated implementation and review notes. The current
contracts are maintained in the design document; this page explains how the
September changes reached that contract.

## 2026-09-05 review corrections

- Type-dependent capabilities are cached by concrete `TypeId`. Factories run
  outside the cache-map lock, and regressions cover multiple struct/enum
  monomorphs and custom model providers, including concurrent calls.
- `ImplDefinitionDescriptor::implemented_trait()` exposes only declaration-
  known links. Symbolic links resolve through `implemented_trait_in(&registry)`
  or `ReflectRegistry::impl_definition_trait`; links belong to snapshot
  indexes, so failed builds and other snapshots cannot mutate declarations.
- Facade macros select an owned provider name with
  `#[reflect(definition_provider_v2 = identifier)]`. The v2 provider is a
  parameterless function returning `&'static TypeDefinitionDescriptor`, with
  no concrete monomorphization requirement and no inferred reflect name.
- `scripts/check-downstream.sh` validates the real `rs-model-metadata`
  workspace, including `derive/`, and `rs-platform`. Local CI and a dedicated
  GitHub Actions job run this gate; missing sibling checkouts fail explicitly.
  Baseline uses the manifest's exact SHA, head uses each dependency's `main`,
  and both channels record feature selection. Private repositories may use
  `DEPENDENCY_TOKEN`.
- Descriptor initialization is measured in fresh child processes, excluding
  process startup. Warm and 1/4/8-thread lookups are separate measurements;
  the platform benchmark covers actual linked models, projection,
  relationship validation, allocation requests, and requested bytes.

## 2026-09-07 explicit invocation snapshots and `codegen_v3`

Every ordinary, catching, thread-safe, and pinned entry now accepts an
explicit `&ReflectRegistry`. Callers use `methods_named_in(registry, ...)`
for lookup and pass the same registry to `invoke_*(registry, invocation)`.
Function pointers quantify independent `for<'registry, 'call>` lifetimes;
outputs, futures, and recovery retain no registry borrow, while input
lifetimes, Local/ThreadSafe, and Pin constraints remain. Invocation neither
initializes the global registry nor falls back to global capabilities.

Safely generated special receivers always have static adapters; inventory
probing no longer determines entry availability. A missing, mistyped, or
fact-only receiver capability in the selected snapshot yields
`ReceiverAdapterUnavailable` and preserves all caller-ordered inputs. Adapter
rejection and intrinsic conflicts remain distinct structured causes, while
statically unsupported signatures still have no entry point.

`TypeDescriptor` debug output prints structural facts without capability
queries or provider execution. Frozen member, name, and method indexes do not
execute providers; capability queries for unregistered concrete types may
still lazily initialize intrinsic facts. Providers depend only on static type
facts and must not re-enter capability or registry initialization. Downstream
custom capabilities/providers may carry model metadata; reflect does not define
or interpret domain semantics.

This breaking change removed `codegen_v2`; facades must expose exactly
`codegen_v3`. Model `__private::v4` and `definition_provider_v2` retain their
independent contracts. Package version remains 0.1.0 and unpublished.

The acceptance and coverage gates established in this period are described in
the design's verification matrix and remain part of the current contract.
