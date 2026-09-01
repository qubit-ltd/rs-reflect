# qubit-reflect User Guide

[简体中文](2026-08-29-qubit-reflect-user-guide.zh_CN.md) · [README](../README.md) · [API documentation](https://docs.rs/qubit-reflect)

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
  It exposes structural views, fields, variants, construction, and capabilities.
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
qubit-reflect = "0.1"
```

The default `derive` feature re-exports `Reflect`, `reflect`, and
`reflect_impl` macros. `default-features = false` keeps the runtime and
handwritten registration APIs, but no longer re-exports those macros.

Choose the narrowest dependency profile that matches the integration:

```toml
# Runtime descriptors, dynamic values, and handwritten registration only.
qubit-reflect = { version = "0.1", default-features = false }

# Macros plus BigDecimal, chrono, and UUID reflection implementations.
qubit-reflect = { version = "0.1", features = ["ecosystem-types"] }

# Macros plus Qubit DataType and Id reflection implementations.
qubit-reflect = { version = "0.1", features = ["qubit-types"] }
```

`ecosystem-types` and `qubit-types` are independent opt-ins. Neither belongs to
the default feature set, so a runtime-only consumer does not compile those
dependency families or silently acquire their trait implementations.

## Core Workflow

### 1. Derive the structural descriptor

```rust
# #![allow(proc_macro_derive_resolution_fallback)]
# #[cfg(feature = "derive")]
# fn main() {
use qubit_reflect::{Reflect, ReflectedMut, ReflectedOwned, ReflectedRef, TypeDescriptor};

#[derive(Reflect)]
#[reflect(crate = qubit_reflect)]
struct User {
    id: u64,
    name: String,
}

let descriptor = TypeDescriptor::of::<User>();
assert_eq!(descriptor.query_name(), "User");
# }
# #[cfg(not(feature = "derive"))]
# fn main() {}
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
# #![allow(proc_macro_derive_resolution_fallback)]
# #[cfg(feature = "derive")]
# fn main() {
use qubit_reflect::{Reflect, ReflectedMut, ReflectedOwned, ReflectedRef, TypeDescriptor};

#[derive(Reflect)]
#[reflect(crate = qubit_reflect)]
struct User {
    id: u64,
    name: String,
}

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
# }
# #[cfg(not(feature = "derive"))]
# fn main() {}
```

`get` needs a shared borrow; `get_mut` and `set` need an exclusive borrow.
Before entering generated code, the adapter checks the receiver type, operation
policy, and replacement `TypeId`. A failed `set` returns `FieldSetFailure`,
whose recovery retains both the original target borrow and the owned
replacement instead of silently dropping either input.

### 3. Construct a new value from editor input

For a named struct, supply every constructible field by query name:

```rust
# #![allow(proc_macro_derive_resolution_fallback)]
# #[cfg(feature = "derive")]
# fn main() {
use qubit_reflect::{NamedConstructionInput, Reflect, ReflectedOwned, TypeDescriptor};

#[derive(Reflect)]
#[reflect(crate = qubit_reflect)]
struct User {
    id: u64,
    name: String,
}

let value = TypeDescriptor::of::<User>()
    .construct_struct(NamedConstructionInput::new([
        ("id", ReflectedOwned::new(7_u64)),
        ("name", ReflectedOwned::new(String::from("Ada"))),
    ]))
    .expect("complete, exactly typed input")
    .downcast::<User>()
    .unwrap_or_else(|_| unreachable!("the descriptor constructs User"));
assert_eq!(value.name, "Ada");
# }
# #[cfg(not(feature = "derive"))]
# fn main() {}
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

A facade that directly hosts `qubit-reflect` derives must explicitly re-export
the public symbols it promises and expose the versioned generated-code entry
under the path expected by the derive:

```rust,ignore
pub use qubit_reflect::Reflect;
pub use qubit_reflect::TypeDescriptor;

#[doc(hidden)]
pub mod __private {
    pub use qubit_reflect::__private::codegen_v1;
}
```

Declarations can then use `#[reflect(crate = my_facade)]`. Do not glob-re-export
`qubit_reflect` or its `__private` module: that turns unrelated implementation
details into the facade's API. A downstream procedural macro may instead expose
the same module under an exact private alias such as `reflect_codegen_v1` and
emit only paths through that alias. `codegen_v1` is a compiler-to-runtime
protocol for generated code, not a supported handwritten construction API; a
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

`Clone` and `Default` are typed capabilities. Register them only where their
Rust bounds hold, then query with `clone_key()` or `default_key()`. Other
arbitrary-self receiver forms require an exact `ReceiverAdapter` registered by
`register_type_capabilities!`; otherwise the method remains discoverable but
reports a stable unavailable reason.

## Errors and Diagnostics

The API avoids implicit conversion: it does not coerce numeric values, parse
strings, infer `Into`, or manufacture `Send`/`Sync` after type erasure.

- Field access returns `FieldAccessError`; failed replacement preserves inputs
  in `FieldSetFailure`.
- Construction returns `ConstructionRecovery` with the error and caller-owned
  input values.
- Pre-execution invocation failures retain their receiver and arguments in
  `InvocationRecovery`.
- An inactive enum variant field produces a structured access error. Fieldless
  integer-`repr` enums expose normalized representation and discriminants;
  data-carrying enums do not invent an integer mapping.

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
| A facade-based derive cannot resolve generated helpers | Preserve the facade path passed to `#[reflect(crate = ...)]` and expose exactly `__private::codegen_v1`; do not point the derive at the underlying crate by accident. |

## Limitations and Best Practices

Keep reflection attributes close to the declaration that owns the contract.
Use opaque boundaries for types whose internals should not be traversed, and
treat descriptors as immutable process-local metadata. Do not use reflection to
infer domain rules or to bypass Rust ownership, privacy, type, or thread-safety
checks. Unsafe functions, unsupported ABIs, variadics, unsafely erasable
unsized values, unspecialized generics, and opaque `impl Trait` returns may be
described, but are not dynamically callable.

## Further Reading

- [README](../README.md) and [简体中文 README](../README.zh_CN.md)
- [简体中文用户指南](2026-08-29-qubit-reflect-user-guide.zh_CN.md)
- [API documentation](https://docs.rs/qubit-reflect)
- [Requirements traceability matrix](2026-08-29-qubit-reflect-requirements-traceability.zh_CN.md)
