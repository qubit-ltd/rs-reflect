# qubit-reflect User Guide

[简体中文](2026-08-29-qubit-reflect-user-guide.zh_CN.md) · [README](../README.md) · API documentation: `cargo doc --all-features`

This guide targets framework and library authors using `qubit-reflect` 0.1 on
Rust 1.94 or later. It explains how to expose a Rust declaration to a
schema-driven tool without giving that tool unrestricted access to values or
layout. `qubit-reflect` is opt-in: macros generate ordinary safe Rust code at
the declaration site, and its immutable descriptors are valid only within the
current process.

## Conceptual Model

The crate has four cooperating parts:

```text
Rust declaration --macro--> TypeDescriptor / member descriptors
                                  |
application value --dynamic wrapper--> checked adapter --> result or recovery
                                  |
linked registration fragments --> ReflectRegistry --> effective type view
```

- `TypeDescriptor` is the unique immutable root for a concrete reflected type.
  It exposes structural views, fields, variants, and construction. Effective
  capabilities are resolved by `ReflectRegistry`.
- `TypeDefinitionDescriptor` is the non-executable root for one generic source
  declaration; concrete instances link to it and retain resolved arguments.
- `ReflectedRef`, `ReflectedMut`, and `ReflectedOwned` carry a shared borrow,
  mutable borrow, or owned value across a checked dynamic boundary.
- Field, construction, and invocation adapters validate policy and exact
  `TypeId` before user code runs.
- `ReflectRegistry` joins statically linked inventory fragments once. It either
  publishes one frozen registry or returns a structured initialization error.

The reflection metadata is not a replacement for a domain model. It does not
infer validation rules, persistence IDs, codecs, relations, or wire formats.
Likewise, query names, `TypeId`, descriptor addresses, and reflected trait
markers are not portable identifiers.

## Scenario

Consider a configuration editor. A host application owns a `User` value. The
editor receives the field name `"name"`, must show its current value, and may
replace it only with another `String`. Success means that the host observes the
new name; an incorrect target, policy, or replacement must fail before a field
is changed.

## Installation and Minimal Configuration

Add the crate with its default features to use the macros:

```toml
[dependencies]
qubit-reflect = { path = "../rs-reflect" }
```

This crate is currently an internal Qubit dependency and is not published to
crates.io. Use the workspace path shown above or an approved internal Git
revision, and keep `qubit-reflect` and `qubit-reflect-derive` on that same
revision.

The default `derive` feature re-exports `Reflect`, `reflect`, and
`reflect_impl` macros. `default-features = false` keeps the runtime and
handwritten registration APIs, but no longer re-exports those macros.

Choose the narrowest dependency profile that matches the integration:

```toml
# Runtime descriptors, dynamic values, and handwritten registration only.
qubit-reflect = { path = "../rs-reflect", default-features = false }

# Macros plus BigDecimal, chrono, and UUID reflection implementations.
qubit-reflect = { path = "../rs-reflect", features = ["ecosystem-types"] }

# Macros plus Qubit DataType and Id reflection implementations.
qubit-reflect = { path = "../rs-reflect", features = ["qubit-types"] }
```

`ecosystem-types` and `qubit-types` are independent opt-ins. Neither belongs to
the default feature set, so a runtime-only consumer does not compile those
dependency families or silently acquire their trait implementations.
If a facade or metadata crate generates descriptors for one of these external
types, that crate must enable the matching feature on its own
`qubit-reflect` dependency; re-exporting the macros does not enable type-family
implementations by itself.

## Core Workflow

### 1. Derive the structural descriptor

```rust
use qubit_reflect::{Reflect, TypeDescriptor};

#[derive(Reflect)]
struct User {
    id: u64,
    name: String,
}

fn main() {
    let descriptor = TypeDescriptor::of::<User>();
    assert_eq!(descriptor.query_name(), "User");
}
```

`TypeDescriptor::of::<T>()` returns the same immutable root for repeated
queries of the same concrete type. Recursive relationships are resolved lazily,
so a path such as `Node -> Vec<Node>` does not recursively initialize forever.

`#[derive(Reflect)]` supports structs and enums. It retains source order for
fields and variants, and records generic definitions separately from their
concrete arguments. `TypeRef` navigation is resolved only when generated Rust
code has static proof; it never guesses a type from a string name.

### 2. Read and replace one field

```rust
use qubit_reflect::{Reflect, ReflectedMut, ReflectedOwned, ReflectedRef, TypeDescriptor};

#[derive(Reflect)]
struct User {
    id: u64,
    name: String,
}

fn main() {
    let name = TypeDescriptor::of::<User>().field("name").expect("field exists");
    let mut user = User { id: 7, name: String::from("Ada") };

    let value = name.get(ReflectedRef::new(&user)).expect("shared read is allowed");
    assert_eq!(value.downcast_ref::<String>().map(String::as_str), Some("Ada"));

    name.set(
        ReflectedMut::new(&mut user),
        ReflectedOwned::new(String::from("Grace")),
    )
    .expect("the replacement has the exact field type");
    assert_eq!(user.name, "Grace");

    let failure = name
        .set(ReflectedMut::new(&mut user), ReflectedOwned::new(9_u64))
        .expect_err("a u64 cannot replace a String field");
    let recovered = failure
        .into_recovery()
        .expect("pre-execution rejection retains ownership")
        .into_value()
        .downcast::<u64>()
        .unwrap_or_else(|_| unreachable!("the original type is retained"));
    assert_eq!(recovered, 9);
}
```

`get` needs a shared borrow; `get_mut` and `set` need an exclusive borrow.
Before entering generated code, the adapter checks the receiver type, operation
policy, and replacement `TypeId`. A `set` rejected during those pre-execution
checks returns `FieldSetFailure` with recovery containing the field identity
and the untouched owned replacement. The target borrow is released when the
failed call ends; it is not stored in `FieldSetRecovery`. If an adapter accepts
ownership and then reports an execution error,
`FieldSetFailure::recovery()` returns `None`; do not treat every failure as
retryable.

### 3. Construct a new value from editor input

For a named struct, supply every constructible field by query name:

```rust
use qubit_reflect::{NamedConstructionInput, Reflect, ReflectedOwned, TypeDescriptor};

#[derive(Reflect)]
struct User {
    id: u64,
    name: String,
}

fn main() {
    let value = TypeDescriptor::of::<User>()
        .construct_struct(NamedConstructionInput::new([
            ("id", ReflectedOwned::new(7_u64)),
            ("name", ReflectedOwned::new(String::from("Ada"))),
        ]))
        .expect("complete, exactly typed input")
        .downcast::<User>()
        .unwrap_or_else(|_| unreachable!("the descriptor constructs User"));
    assert_eq!(value.name, "Ada");
}
```

Use `construct_tuple` or `construct_unit` for the corresponding struct shape;
an enum `VariantDescriptor` provides the same three construction methods.
Construction validates shape, names or indices, duplicates, missing inputs,
policy, and exact types before it consumes owned values. On failure,
`ConstructionRecovery` returns the supplied inputs in caller order. A struct
updater follows the same all-or-nothing validation rule, including for types
that implement `Drop`.

## Advanced Usage

### Integrate through a downstream facade or macro

A facade that directly hosts `qubit-reflect` derives exposes the versioned
generated-code protocol under the path expected by the derive. Public
application exports are an independent choice; this minimal example exports
the two types used by its callers:

```rust
pub use qubit_reflect::Reflect;
pub use qubit_reflect::TypeDescriptor;

#[doc(hidden)]
pub mod __private {
    pub use qubit_reflect::__private::codegen_v2;
}
```

Declarations can then use `#[reflect(crate = my_facade)]`. Generated code needs
only the `codegen_v2` export; the facade does not need to re-export runtime
modules such as `descriptor`, `construct`, or `value`. Do not glob-re-export
`qubit_reflect` or its `__private` module: that turns unrelated implementation
details into the facade's API. A downstream procedural macro may give the same
module through exact item re-exports. `codegen_v2` is a
compiler-to-runtime protocol, not a supported handwritten construction API; a
future incompatible protocol receives a new versioned module.

### Declare traits and callable implementations

- `#[reflect]` reflects a trait declaration, including supertraits, default
  methods, associated types, and associated constants.
- `#[reflect_impl]` reflects an inherent or trait implementation and generates
  invocation adapters for methods whose receiver, parameters, ABI, and output
  can safely cross the dynamic boundary.
- `#[reflect(rename = "...")]` changes the lookup name only; `rust_name()`
  preserves the original source identity. `skip`, `read_only`, `no_construct`,
  `no_invoke`, and `opaque` preserve the applicable structural fact while
  disabling or limiting the associated dynamic operation.

Look up a `MethodInstanceDescriptor` through the registry or an effective type
view, then call `invoke_local` with an `Invocation`. Positional arguments are
the canonical form. The runtime validates receiver, argument count, passing
mode, and exact types in that order; a failure before user code returns the
complete `InvocationRecovery`.

Generic and blanket implementations register definition metadata. To make a
finite concrete generic case callable or effective, declare
`#[reflect(specialize(...))]`. `#[reflect(thread_safe)]` requests a
thread-safe adapter and is accepted only when the generated Rust bounds prove
the receiver, inputs, owned output, and future boundary. A thread-safe value
can be downgraded to local mode, never upgraded by a runtime flag.

### Capabilities and registry discovery

Call `ReflectRegistry::initialize()` after the relevant crates are linked. The
registry transactionally aggregates fragments: conflicts yield `RegistryError`
and do not publish a partial result. Once frozen, its type, name, trait, impl,
capability, and effective-method indexes do not change. Static built-ins are
available before a lookup; on-demand composite descriptors use a separate
interner and do not mutate the public frozen registry.

```rust
use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::registry::ReflectRegistry;

#[derive(Reflect)]
struct Service;

fn main() {
    let snapshot = ReflectRegistry::initialize().expect("all fragments validate");
    let descriptor = TypeDescriptor::of::<Service>();
    let _methods = descriptor.methods_in(snapshot);
    assert!(snapshot.get(descriptor.type_id()).is_some());
    let clone = snapshot.capability_by_id(descriptor, "qubit.reflect.clone")
        .expect("valid capability declarations");
    assert!(clone.is_none());
}
```

Passing the snapshot to `impls_in`, `methods_in`, or `methods_named_in` makes
the lookup dependency explicit. The snapshot is immutable; a failed global
initialization never exposes a partially built registry.

`snapshot.definitions()` enumerates generic declarations even when no concrete
instance is registered. Query them by `TypeDefinitionId`, Rust path, or query
name, and use `definition_capability` or `definition_capability_by_id` for
definition-level extensions. Definition fields contain `TypeExpression`
values and intentionally provide no value-access adapters.

`Clone` and `Default` are typed capabilities. Register them only where their
Rust bounds hold, then query with `clone_key()` or `default_key()`. Other
arbitrary-self receiver forms require an exact `ReceiverAdapter` registered by
`register_type_capabilities!`; otherwise the method remains discoverable but
reports a stable unavailable reason.

### Build an isolated registry snapshot

`ReflectRegistry::initialize()` is the process-global inventory entry point.
Use `RegistrySnapshotBuilder` when a library, fixture, or test owns an explicit
set of fragments and must keep it independent from linked inventory and global
initialization state. The builder starts empty, does not execute providers while
facts are being added, and freezes one immutable `ReflectRegistry` only when
`build()` succeeds.

```rust
use qubit_reflect::capability::{CapabilityDescriptor, CapabilityKey};
use qubit_reflect::identity::{CapabilityId, FragmentIdentity};
use qubit_reflect::registry::RegistrySnapshotBuilder;
use qubit_reflect::TypeDescriptor;

fn source(kind: &str, line: u32) -> FragmentIdentity {
    FragmentIdentity::new("example", "fixture", line, 1, kind, u64::from(line))
}

fn main() -> Result<(), qubit_reflect::RegistryError> {
    let target = TypeDescriptor::of::<u32>();
    let key = CapabilityKey::<u32>::new(
        CapabilityId::new("example.limit").expect("valid capability ID"),
    );
    let mut builder = RegistrySnapshotBuilder::new();
    builder
        .add_type(target, source("type", 10))
        .add_type_capabilities(
            target,
            vec![CapabilityDescriptor::with_adapter(key, 7_u32)],
            source("capability", 11),
        );
    let snapshot = builder.build()?;

    assert!(snapshot.get(target.type_id()).is_some());
    assert_eq!(
        snapshot
            .capability(target, key)
            .expect("valid capability declarations"),
        Some(&7),
    );
    assert!(target.methods_in(&snapshot).is_empty());
    Ok(())
}
```

The membership and capability payloads are separate. An empty builder yields a
snapshot with no registered roots. Calling only `add_type_capabilities` creates
a capability-only snapshot: `types()` remains empty, while `capability()` and
`capability_by_id()` can still resolve the target. The other typed inputs are
`add_definition`, `add_trait`, `add_impl_definition`, `add_impl`, and
`add_definition_capabilities`.

Pass the resulting snapshot explicitly to `impls_in`, `methods_in`, or
`methods_named_in` when a property or method query must use that exact set of
facts. `build()` validates all identities, links, and capability conflicts as
one transaction. It returns a `RegistryError` without publishing partial
state; provide stable `FragmentIdentity` values so duplicate or changed source
identities are diagnosable. For a conflict, inspect
`conflicting_fragments()`, `capability_details()`, `capability_target()`, and
`capability_id()`. Intrinsic provider failures remain available through
`intrinsic_conflict()` and `Error::source()`.

This API does not change the generated-code protocols. Existing facades keep
exposing `__private::codegen_v2`, and `qubit-model-metadata` keeps its separate
model ABI v4. Intrinsic capability providers must use static type facts only;
they must not depend on a snapshot or re-enter registry initialization. The
provider runs outside the capability cache lock, and its panic is not converted
into absence or `CapabilityConflict`.

### Migrating effective capability queries

The existing query names now return `Result`; no error-swallowing compatibility entry remains.
`capabilities` returns `Result<&TypeCapabilities, CapabilityConflict>`; typed/textual single
queries return `Result<Option<_>, CapabilityConflict>`. Handle failure before testing for absence.
Conflicts retain their kind, capability ID, and both adapter TypeIds. Registration failures also expose
the original conflict through `RegistryError::intrinsic_conflict()` and `Error::source()`.

`Ok(None)` can mean an absent ID, a typed key with a different adapter type, or a fact-only descriptor.
These are distinct from an invalid set. `types_with_capability` and definition queries read frozen
indexes without executing factories and do not gain `Result`. Effective queries for unregistered
concrete instances may execute an intrinsic factory, without inserting the instance into the snapshot.

Providers must depend only on static type facts, never on snapshots, time, or mutable external
configuration, and must not re-enter registry initialization. Generic intrinsic factories run outside
the cache-map lock; successes and conflicts are cached by concrete `TypeId` and shared by concurrent
queries. Provider panics still propagate; they do not become absence or `CapabilityConflict`.
Generated invocation adapters preserve registry/receiver-capability failures as structured invocation
errors and restore the receiver, values, names, and original caller ordering.

Downstream `ModelRegistry::metadata_for` similarly returns `Result<Option<_>, ModelMetadataError>`.
Property queries propagate `PropertyResolutionError`; resolver diagnostics retain the original
`cause()` and path context. `qubit-platform-testkit::link_all::validate_all_models` returns
`ModelRegistryError` and validates registration availability only.

### Constructing empty structs

| Declaration | Descriptor shape | Construction entry |
| --- | --- | --- |
| `struct A;` | `StructKind::Unit` | `construct_unit()` |
| `struct B {}` | `StructKind::Named` | `construct_struct(NamedConstructionInput::new([]))` |
| `struct C();` | `StructKind::Tuple` | `construct_tuple(TupleConstructionInput::new([]))` |

The distinction also applies to empty const-generic structs. A wrong shape returns a construction
error, never a value of a different type or an internal assertion failure.

### Choose transparent, opaque, and thread-safe boundaries

Use the narrowest boundary that matches what downstream code must do:

| Boundary | What it exposes | Important constraint |
| --- | --- | --- |
| Ordinary reflected field | A resolved `TypeRef` and navigation into the field type | The concrete field type must implement `Reflect`. |
| `#[reflect(opaque)]` field | Whole-value read, replacement, argument passing, and outer construction | Operations still require exact `TypeId`; internal structure and independent root construction stay unavailable. |
| `#[reflect(opaque)]` type | One opaque root descriptor and explicitly registered capabilities | It exposes no fields, variants, or per-member construction. |
| Local dynamic wrapper | Checked operations on ordinary local values and borrows | It is not upgraded to `Send` or `Sync` by registry metadata. |
| `SendReflected*` wrapper | A thread-safe erased boundary created under compile-time bounds | It can be consumed with `into_local`; a local wrapper cannot be upgraded at runtime. |

Keep model semantics downstream. A model or schema layer may associate a
`FieldDescriptor` with validation, persistence, codec, relation, or redaction
metadata, but those facts do not become `qubit-reflect` capabilities or
descriptor attributes. This preserves the dependency direction from model
crates to `qubit-reflect`.

For a type-level thread-safe contract, the same mode covers owned-to-borrow
bridges, field access, construction, updates, and generated method adapters:

```rust
use qubit_reflect::Reflect;
use qubit_reflect::SendReflectedMut;
use qubit_reflect::SendReflectedOwned;
use qubit_reflect::SendReflectedRef;
use qubit_reflect::TypeDescriptor;

#[derive(Reflect)]
#[reflect(thread_safe)]
struct SharedCounter {
    value: u64,
}

fn main() {
    let field = TypeDescriptor::of::<SharedCounter>()
        .field("value")
        .expect("derived field");
    let mut counter = SharedCounter { value: 1 };
    let current = field
        .get_thread_safe(SendReflectedRef::new(&counter))
        .expect("thread-safe read adapter");
    assert_eq!(current.downcast_ref::<u64>(), Some(&1));
    field
        .set_thread_safe(
            SendReflectedMut::new(&mut counter),
            SendReflectedOwned::new(2_u64),
        )
        .expect("thread-safe set adapter");
    assert_eq!(counter.value, 2);
}
```

## Errors and Diagnostics

The API avoids implicit conversion: it does not coerce numeric values, parse
strings, infer `Into`, or manufacture `Send`/`Sync` after type erasure.

- Field access returns `FieldAccessError`. A replacement rejected before the
  adapter runs preserves the untouched owned value in `FieldSetFailure`; an
  error after ownership crosses the adapter boundary has no recovery payload.
- Construction returns `ConstructionRecovery` with the error and caller-owned
  input values.
- Pre-execution invocation failures retain their receiver and arguments in
  `InvocationRecovery`.
- An inactive enum variant field produces a structured access error. Fieldless
  integer-`repr` enums expose normalized representation and discriminants;
  data-carrying enums do not invent an integer mapping.

Handle these errors by matching their structured categories, not their
`Display` text. Inspect recovery before retrying: construction and invocation
recovery preserve caller order, while `FieldSetFailure::recovery()` explicitly
distinguishes a retryable pre-execution rejection from a post-boundary error.

Normal invocation does not catch panics. `#[reflect(catch_unwind)]` adds an
explicit catching entry point when supported; a `panic=abort` build reports it
as unavailable. Async adapters return a future tied to the call lifetime; they
do not choose an executor or poll it, and async methods cannot use
`catch_unwind`.

## Troubleshooting

| Symptom | Check |
| --- | --- |
| `field("...")` returns `None` | Use the query name; `rename` changes it while `rust_name()` retains the source spelling. |
| A field operation fails | Verify the target wrapper (`ReflectedRef` versus `ReflectedMut`), the field policy, and the replacement's exact type. |
| A method is visible but not callable | Inspect its unavailable reason; generic methods need a supported specialization, and unsafe, variadic, unsupported ABI, opaque output, and some borrowed/unsized forms cannot cross the boundary. |
| Registry initialization fails | Inspect `RegistryError`; initialization errors are cached, so start a new process after fixing conflicting registrations. |
| Cross-thread invocation is unavailable | Use a method explicitly marked `thread_safe` and construct `SendReflected*` values only where Rust's bounds are satisfied. |
| An external type has no `Reflect` implementation | Enable `ecosystem-types` or `qubit-types` on the crate that owns the reflection boundary; these implementations are not enabled by default. |
| A facade-based derive cannot resolve generated helpers | Preserve the facade path passed to `#[reflect(crate = ...)]`, expose exactly the matching `__private::codegen_v2`, and ensure the facade and derive use compatible `qubit-reflect` protocol versions. |

## Limitations and Best Practices

Keep reflection attributes close to the declaration that owns the contract.
Use opaque boundaries for types whose internals should not be traversed, and
treat descriptors as immutable process-local metadata. Do not use reflection to
infer domain rules or to bypass Rust ownership, privacy, type, or thread-safety
checks. Unsafe functions, unsupported ABIs, variadics, unsafely erasable
unsized values, unspecialized generics, and opaque `impl Trait` returns may be
described, but are not dynamically callable.
Tuple and portable function-pointer descriptors support arities 0 through 32.
Arity 33 and above is intentionally unsupported and has no `Reflect` impl.

## Further Reading

- [README](../README.md) and [简体中文 README](../README.zh_CN.md)
- [简体中文用户指南](2026-08-29-qubit-reflect-user-guide.zh_CN.md)
- API documentation generated internally with `cargo doc --all-features`
- [English design](2026-09-03-qubit-reflect-design.md) and [简体中文设计](2026-09-03-qubit-reflect-design.zh_CN.md)
- [English requirements](2026-09-03-qubit-reflect-requirements.md) and [traceability matrix](2026-09-03-qubit-reflect-requirements-traceability.md)
- [中文版需求规范](2026-08-28-qubit-reflect-requirements.zh_CN.md) and [追踪矩阵](2026-08-29-qubit-reflect-requirements-traceability.zh_CN.md)
