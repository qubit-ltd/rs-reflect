# qubit-reflect User Guide

`qubit-reflect` is for framework authors who need to discover and operate on user-defined Rust types on stable Rust. Macros generate safe Rust code at the declaration site and submit immutable descriptors to a process-local registry. The crate does not inspect private layout, depend on rustc-private APIs, or use `unsafe`.

## Installation and features

```toml
[dependencies]
qubit-reflect = "0.1"
```

Default features include the three macros. With default features disabled, the runtime and handwritten registration APIs remain available, but the derive and attribute macros are not re-exported.

## Start with a domain object

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
let name = descriptor.field("name").expect("reflected field");
let mut user = User { id: 7, name: String::from("Ada") };

let current = name
    .get(ReflectedRef::new(&user))
    .expect("type and policy validation");
assert_eq!(current.downcast_ref::<String>().map(String::as_str), Some("Ada"));

name.set(
    ReflectedMut::new(&mut user),
    ReflectedOwned::new(String::from("Grace")),
).expect("exactly typed replacement");
assert_eq!(user.name, "Grace");
# }
# #[cfg(not(feature = "derive"))]
# fn main() {}
```

Descriptors are immutable process-local objects. Repeated lookup of one concrete Rust type returns the same descriptor. Recursive relations resolve lazily, so a relation such as `Node -> Vec<Node>` does not recurse during initialization.

## The three macros

- `#[derive(Reflect)]` describes structs, enums, fields, variants, generic arguments, and construction entry points.
- `#[reflect]` describes trait declarations, supertraits, default methods, associated types, and associated constants.
- `#[reflect_impl]` describes inherent or trait implementations and generates invocation adapters for supported methods.

Common helper attributes:

- `rename = "..."` changes only the query name; `rust_name()` retains source identity.
- `skip` retains the member and source index while disabling dynamic operations.
- `read_only` disables mutable field borrowing and replacement.
- `no_construct` and `no_invoke` disable construction and invocation respectively.
- `opaque` stops recursive structural exposure for a root or field.
- `thread_safe` explicitly requests a thread-safe invocation adapter.
- `catch_unwind` explicitly requests a panic-catching adapter. Async methods reject it because their panic occurs while polling the future.
- `specialize(...)` registers a finite concrete specialization for a generic impl or method.
- `dyn_compatible(...)` supplies proof when an attribute macro cannot inspect a supertrait across declarations. rustc still validates the generated real `dyn Trait` type.

## Descriptor navigation

`TypeDescriptor` navigates to typed views for structs, enums, primitives, text, sequences, maps, sets, pointers, tuples, functions, and trait objects. Fields and variants retain source order. Generic definitions are separate from concrete arguments, with bidirectional navigation between runtime and definition indices.

A `TypeRef` may be resolved, symbolic, or opaque. Associated types and concrete generic arguments navigate to `TypeDescriptor` only when the generated Rust code has static proof; the runtime never guesses identity from a type-name string.

## Fields and enum variants

Field reads require a shared borrow. `get_mut` and `set` require an exclusive mutable borrow. Receiver type, policy, and exact `TypeId` are validated before entering the generated adapter.

For enums, a variant descriptor checks whether its branch is active. Accessing a field of an inactive variant returns a structured error. Fieldless integer-`repr` enums also expose normalized representation and discriminants; data-carrying enums do not fabricate an integer mapping.

When `set` fails, `FieldSetFailure` carries recovery containing the original target borrow and owned replacement. Input values are not silently lost.

## Dynamic invocation

Find a `MethodInstanceDescriptor` through the registry or effective type view, then submit a receiver and arguments with `Invocation`:

```rust
use qubit_reflect::invoke::{Invocation, InvocationArg, InvocationOutput};
use qubit_reflect::descriptor::MethodInstanceDescriptor;
use qubit_reflect::{ReflectedOwned, ReflectedRef};

# fn invoke(method: &MethodInstanceDescriptor, service: &String) {
let output = method
    .invoke_local(Invocation::borrowed(
        ReflectedRef::new(service),
        [InvocationArg::Owned(ReflectedOwned::new(String::from("Ada")))],
    ))
    .expect("registered local adapter")
    .expect("pre-call validation");

let InvocationOutput::Owned(value) = output else { unreachable!() };
let greeting = value.downcast::<String>().unwrap_or_else(|_| unreachable!());
# let _ = greeting;
# }
```

Positional arguments are canonical. Named binding accepts only unique simple identifiers; wildcard, destructuring, and `@` patterns remain callable positionally. Validation order is receiver, count, passing mode, and exact type. Any failure before user code returns complete `InvocationRecovery`.

Outputs distinguish unit, owned, shared borrow, mutable borrow, and future. Borrowed outputs record `BorrowOrigin::Receiver` or parameter indices. `&str` and `&mut str` use dedicated safe dynamic variants. Slices and arbitrary `dyn Trait` values cannot be forged through the generic dynamic-value entry point.

Normal invocation does not catch panics. Only a `catch_unwind` method provides a catching entry point; a `panic=abort` build reports that capability as unavailable. An async adapter only returns a future preserving the `'call` lifetime. It does not select an executor or poll the future.

## Dynamic construction and update

Derive generates adapters for constructible structs and enum variants:

```rust
use qubit_reflect::{ConstructionRecovery, NamedConstructionInput, ReflectedOwned, TypeDescriptor};
use qubit_reflect::value::Local;

# struct User { id: u64, name: String }
# fn construct(descriptor: &TypeDescriptor) -> Result<(), ConstructionRecovery<Local>> {

let value = descriptor
    .construct_struct(NamedConstructionInput::new([
        ("id", ReflectedOwned::new(7_u64)),
        ("name", ReflectedOwned::new(String::from("Ada"))),
    ]))?
    .downcast::<User>()
    .unwrap_or_else(|_| unreachable!());
# let _ = value;
# Ok(())
# }
```

Construction validates shape, names or indices, duplicates, missing values, policy, and exact types before consuming owned inputs. `ConstructionRecovery` returns inputs in caller order. Struct update validates every replacement before moving the base or fields, including for types that implement `Drop`.

## Registry, traits, and impls

```rust
use std::any::TypeId;
use qubit_reflect::{ReflectRegistry, RegistryError};

# struct User;
# fn inspect() -> Result<(), RegistryError> {
let registry = ReflectRegistry::initialize()?;
let root = registry.get(TypeId::of::<User>()).expect("static registration");
let effective = registry.effective_view(root.type_id());
# let _ = effective;
# Ok(())
# }
```

The registry transactionally aggregates inventory fragments. Any conflict returns a structured error without publishing a partial registry. After freeze, type, name, trait, impl, capability, and effective-method indexes do not change.

Static builtins are present before lookup. On-demand concrete composites such as `Option<Vec<T>>` use a separate interner and never mutate the public frozen registry.

Reflected trait identity comes from a generated marker `TypeId`. Unreflected external traits use caller-provided stable `ExternalTraitId` values. Generic and blanket impls register definition descriptors only. A concrete impl participates in effective lookup and invocation only after explicit registration or specialization.

Associated types and constants always retain their structured declarations. A default associated constant has a reader only when its trait declaration environment proves the owned-value boundary. An explicit impl override or concrete specialization may prove that boundary in its own concrete environment. The runtime does not probe arbitrary concrete associated types to upgrade capability.

## Local, ThreadSafe, and capabilities

Dynamic values use a mode parameter internally:

- `ReflectedRef`, `ReflectedMut`, and `ReflectedOwned` are local wrappers and make no unconditional `Send`/`Sync` promise.
- `SendReflectedRef`, `SendReflectedMut`, and `SendReflectedOwned` enforce thread-safety through Rust bounds at construction.

ThreadSafe values can be downgraded to Local. The runtime offers no flag-based Local-to-ThreadSafe upgrade. `#[reflect(thread_safe)]` validates receiver, arguments, owned output, and future bounds at the method span.

Built-in `Clone` and `Default` capabilities provide typed operation adapters. Register them only when the concrete bounds are true, then query with `clone_key()` or `default_key()`; the key's adapter type prevents an untyped or mismatched call:

```rust
use qubit_reflect::capability::{clone_key, default_key};
use qubit_reflect::{ReflectedOwned, TypeDescriptor};

# fn use_capabilities(descriptor: &TypeDescriptor, value: &ReflectedOwned) {
if let Some(cloner) = descriptor.get_capability(clone_key()) {
    let copy = cloner.clone_owned(value).expect("exact registered type");
    let _ = copy;
}
if let Some(default) = descriptor.get_capability(default_key()) {
    let initial = default.create();
    let _ = initial;
}
# }
```

Other arbitrary self receivers require an exact `ReceiverAdapter` registered with `register_type_capabilities!`. Without that capability, the full method remains discoverable but invocation reports a stable unavailable reason.

## Model-layer boundary

A downstream model runtime may act as a facade: it can re-export `qubit-reflect` descriptors and the hidden generated-code contract, while its derive facade delegates with an explicit `#[reflect(crate = model_runtime)]` path. Model packages then share the same process-local descriptor graph without making `qubit-reflect` depend on that model layer.

Reflection describes Rust structure, checked dynamic operations, capabilities, and registry links. Domain validation rules, relation semantics, codecs and wire formats, persistence identities, and cross-process model IDs remain responsibilities of the model layer. They must not be inferred from `TypeId`, descriptor addresses, query names, or reflection metadata.

## Errors, recovery, and limits

- Dynamic operations use exact Rust type identity and perform no implicit conversion.
- Failed owned downcasts, field writes, construction, and pre-execution invocation provide recovery.
- `TypeId`, descriptor addresses, and reflected trait markers are process-local. They are not serialization or cross-process model IDs.
- `opaque` intentionally stops recursive navigation. It does not bypass dynamic-value validation.
- Unsafe functions, unsupported ABIs, variadics, unsafely erasable unsized values, unspecialized generics, and opaque `impl Trait` returns remain described but not callable, with ordered structured reasons.
- Registry initialization errors are cached. After correcting a registration conflict, initialize a new process.

## Development and verification

```bash
./align-ci.sh
./ci-check.sh
cargo test --workspace --all-features
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps
cargo bench --workspace --all-features --no-run
```

After changing macro diagnostics, also run `cargo test --test ui_tests`. See the [requirements traceability matrix](2026-08-29-qubit-reflect-requirements-traceability.zh_CN.md) for the requirement-to-code and requirement-to-test mapping.
