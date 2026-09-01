# `qubit-reflect` Design

- Date: 2026-09-01
- Status: breaking boundary redesign implemented and verified in staged checks; the full release matrix remains continuously enforced by CI
- Source requirements: [final requirements specification](2026-08-28-qubit-reflect-requirements.zh_CN.md)
- Repository: `rs-reflect`
- Protocol: `qubit-reflect 0.1` / `__private::codegen_v1`

## 1. Purpose and boundary

`qubit-reflect` provides domain-neutral static descriptors, checked dynamic access, invocation, construction, and
link-time registration. Model metadata, validation, codecs, and redaction are downstream semantics. The dependency
direction is always `rs-model-* -> qubit-reflect`.

Five invariants drive the design:

1. Public APIs cannot construct structurally invalid expressions or descriptors.
2. Macros parse, validate, and analyze semantics before emitting tokens.
3. The runtime kernel does not implicitly bind ecosystem or Qubit-specific types.
4. Generated code depends on a narrow, versioned hidden protocol.
5. Every advertised feature combination is executed at package scope.

The redesign intentionally breaks public fields and the old hidden ABI without compatibility aliases. A break is
accepted only when it removes an invalid state, dependency leak, or code-generation coupling. Stable dynamic-value,
registry-query, and failure-recovery semantics are not redesigned.

## 2. Repository and dependency architecture

### 2.1 Two crates

```text
rs-reflect/
├── Cargo.toml                 # qubit-reflect runtime and workspace
├── src/
├── derive/                    # qubit-reflect-derive proc macro
├── tests/
├── test-crates/               # facade and cross-crate fixtures
├── fuzz/
├── benches/
└── doc/
```

Rust requires procedural macros to live in a `proc-macro` crate, so runtime and derive remain two publication units.
The derive crate does not depend on the runtime; it emits paths through the caller's facade and therefore avoids a
Cargo dependency cycle.

There is no third codegen crate. This decision is reconsidered only when at least two independent macro crates share
the same pure analysis logic, contain measurable duplicate implementations, and extraction reduces rather than
moves diagnostic-span and release complexity. The model macro currently consumes runtime factories; it does not
share the reflect derive analyzer, so that threshold is not met.

### 2.2 Features and dependency budget

| Configuration | Meaning | Direct effect |
| --- | --- | --- |
| `default = ["derive"]` | ordinary macro use | enables `qubit-reflect-derive` |
| `default-features = false` | runtime only | compiles neither derive nor external type families |
| `ecosystem-types` | ecosystem reflection | explicitly enables `bigdecimal`, `chrono`, and `uuid` |
| `qubit-types` | Qubit type reflection | explicitly enables `qubit-datatype` and `qubit-id` |
| `bench-internals` | internal benchmark hooks | does not change the ordinary user API |

The runtime kernel directly depends on `inventory` and `thiserror`; all external type dependencies are optional.
Rust's orphan rule requires implementations of `Reflect` for external types to live in the crate that owns the
trait. Feature modules satisfy that rule without forcing those implementations or dependencies into the default
kernel.

The derive dependency budget is `proc-macro2`, `quote`, `syn`, and `proc-macro-crate`. The repository uses
Rust 2024 with MSRV 1.94. Runtime and generated code both forbid unsafe code.

## 3. Runtime modules

```text
src/
├── access/                     # field and variant access with recovery
├── builtin/
│   ├── ecosystem.rs            # cfg(feature = "ecosystem-types")
│   ├── qubit.rs                # cfg(feature = "qubit-types")
│   └── internal/               # shared built-in implementation macro
├── capability/                 # stable IDs, adapters, and capability sets
├── construct/                  # named, tuple, unit construction and update
├── descriptor/                 # type, field, variant, trait, impl, method
├── error/                      # registry and exact-type errors
├── expression/                 # checked structural type/const/predicate trees
├── identity/                   # fragment, member, trait, visibility identity
├── invoke/                     # invocation, future, and failure recovery
├── private/
│   └── codegen_v1/             # sole generated-code protocol
├── registry/
│   ├── registry_builder.rs     # conflict checks and pre-freeze aggregation
│   ├── registry.rs             # immutable queries
│   └── effective_type_view.rs  # resolved impl and method view
└── value/                      # local and thread-safe dynamic modes
```

Descriptor and expression fields are private. Public constructors validate empty paths, parallel-slice lengths,
duplicate identities, and related invariants at the boundary. Read-only accessors expose navigation facts.
Diagnostic text does not participate in `Eq` or `Hash`, so formatting cannot change structural identity.

The registry collects all fragments, checks duplicate identities and type/trait/impl/capability conflicts, then
publishes indexes once. Failure cannot expose partial state. Candidate and index order follows stable fragment order.
Effective method views are built during freezing, and query paths do not execute user code.

## 4. Derive pipeline

```text
proc-macro entry
    │
    ▼
parse ──> validate ──> domain IR
                           │
                           ▼
                    expansion dispatcher
                           │
          ┌────────────────┴────────────────┐
          ▼                                 ▼
 descriptor orchestration          invocation analysis
                                             │
                                             ▼
                                      InvocationPlan
                                             │
                                             ▼
                                        token emitter
```

- `derive/src/entry.rs` owns entry-point error aggregation.
- `parse/` converts `syn` input into domain IR.
- `validate/` checks declaration shapes and attribute combinations.
- `expand/context.rs` is the single owner of facade resolution and fragment fingerprints.
- `expand/dispatcher.rs` routes structs, enums, traits, and impls.
- `expand/invocation/analysis.rs` decides receiver, parameter, output, thread-safe, catching, async, and unavailable semantics.
- `expand/invocation/emit.rs` emits shared semantic fragments from an `InvocationPlan`: unavailable reasons,
  argument bindings, and thread-safe/catching assertions.

Trait default methods and concrete impls share the same invocation analysis and semantic emitters. Their execution
shells stay with their natural owners: the former emits a `Self`-bounded default-method hook, while the latter emits a
concrete-target adapter and registration. Combining those ownership shells behind mode flags would move complexity
without creating a useful abstraction. Style checking distinguishes
host Rust from generated-token scopes, so paths intentionally emitted inside `quote!` do not distort host-source
rules.

## 5. Versioned generated-code protocol and facades

Generated reflection code reaches runtime factories only through:

```text
facade::__private::codegen_v1::{descriptor, expression, registration}
```

The `__private` root does not flatten descriptor or registration factories. Public struct fields are not a codegen
ABI; factories construct the runtime's validated data model. An incompatible protocol introduces a sibling version
instead of silently widening v1.

A downstream facade explicitly re-exports every public symbol it promises and exposes exactly
`__private::codegen_v1`. It must not use `pub use qubit_reflect::*` or
`pub use qubit_reflect::__private::*`.

`qubit-model-metadata` separately owns its `__private::v2` model-metadata ABI. It gives the reflection protocol the
exact alias `reflect_codegen_v1`, keeping ownership, versioning, and migration reasons orthogonal.

## 6. Dynamic safety boundary

Dynamic values have local and thread-safe modes. Erasure remains constrained by exact `TypeId`, borrow lifetimes,
and `Send + Sync` bounds. Field access, invocation, and construction follow the same transaction:

1. validate target descriptors, receivers, argument counts, and exact types;
2. validate access policy, executability, and thread mode;
3. enter a generated adapter only after every check succeeds;
4. return unconsumed owned inputs when pre-adapter validation fails;
5. generate panic catching only when explicitly requested and unwind-safety bounds are provable.

The library performs no numeric conversion, string parsing, or `Into` inference. Reflection describes and executes
exact Rust semantics rather than creating a second implicit type system.

## 7. Errors and diagnostics

Expected failures use structured errors. Registry conflicts retain both fragments, mismatches retain expected and
actual types, and invocation/construction failures retain the stage and recoverable inputs. Macro diagnostics keep
the original span and aggregate independent errors when possible.

Static descriptor factories receive generator-validated data. User-controlled data crosses checked constructors.
An internal `expect` may only state a fact proven earlier by the same generator; it cannot replace input validation.

## 8. Verification matrix

| Layer | Primary verification |
| --- | --- |
| runtime only | package check/test/doctest; dependency tree excludes derive and external type families |
| default | derive facade, ordinary doctests, integration tests |
| all features | ecosystem/Qubit types, workspace tests, Clippy, Rustdoc |
| derive | parser/analysis unit tests, trybuild pass/fail, invocation integration |
| registry | cross-crate aggregation, conflicts, freeze, stable ordering, concurrent initialization |
| ABI/facade | renamed dependencies, explicit facade, `codegen_v1`, and model `v2` |
| robustness | coverage, bounded fuzz smoke, benchmark compile, Miri/sanitizers when available |

`.rs-ci-cargo-matrix.json` is the executable source of truth for supported feature combinations. The
[requirements traceability matrix](2026-08-29-qubit-reflect-requirements-traceability.zh_CN.md) keeps all 284
requirement IDs mapped one-to-one and validates every referenced code and test path.

## 9. Explicit non-goals

- No model, codec, validator, or redaction semantics.
- No implicit external-type implementations; features define dependency and trait-implementation boundaries.
- No third codegen crate.
- No public-field compatibility, flattened `__private`, or deprecated shim.
- No one-use helpers or one-type-per-file fragmentation created solely to reduce line counts.
- No `no_std` promise; the runtime currently depends on `std`.
