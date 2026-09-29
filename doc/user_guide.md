# Qubit Reflect user guide

<!-- reflect-contract: facade.explicit=qubit_reflect -->
<!-- reflect-contract: provider.qualified=custom-provider -->
<!-- reflect-contract: panic.async_poll=not-caught -->
<!-- reflect-contract: panic.abort=catching-unavailable -->
<!-- reflect-contract: dispatch.input_recovery=original-input -->
<!-- reflect-contract: examples.native=cargo-example -->

[Chinese user guide](user_guide.zh_CN.md) · [README](../README.md) · [API reference](https://docs.rs/qubit-reflect)

This guide covers `qubit-reflect` 0.1.0 on Rust 1.94 or later. It is for Rust application and framework developers who need to read and update structs by field name at runtime. Maintainers of dependency facades should read [Facade integration and migration](#facade-integration-and-migration). Reading through [Check the PATCH result](#check-the-patch-result) is enough to integrate field-level PATCH handling in a support console. Later sections cover construction, method invocation, registries, capabilities, and thread-safe boundaries.

## Contents

- [The problem it solves](#the-problem-it-solves)
- [Where to start](#where-to-start)
- [Integrate a support console](#integrate-a-support-console)
  - [Define the editable record type](#define-the-editable-record-type)
  - [Types used on this path](#types-used-on-this-path)
- [Check the PATCH result](#check-the-patch-result)
  - [What success looks like](#what-success-looks-like)
- [Construct a new value from input](#construct-a-new-value-from-input)
  - [Constructing empty structs](#constructing-empty-structs)
- [Invoke methods](#invoke-methods)
  - [Recover unavailable input](#recover-an-input-when-the-selected-entry-is-unavailable)
  - [Explicit generic specialization](#explicit-generic-specialization)
- [Discover types and extension capabilities](#discover-types-and-extension-capabilities)
  - [Capabilities and registry discovery](#capabilities-and-registry-discovery)
  - [Build an isolated registry snapshot](#build-an-isolated-registry-snapshot)
- [Choose features and access boundaries](#choose-features-and-access-boundaries)
  - [Choose dependency features](#choose-dependency-features)
  - [Choose transparent, opaque, and thread-safe boundaries](#choose-transparent-opaque-and-thread-safe-boundaries)
- [Errors, diagnostics, and troubleshooting](#errors-diagnostics-and-troubleshooting)
- [Boundaries and a practice checklist](#boundaries-and-a-practice-checklist)
- [Facade integration and migration](#facade-integration-and-migration)
- [Further reading](#further-reading)

## The problem it solves

Take a back-office support console. An operator sends `PATCH` requests that name the field to change, such as `email` or `credit_limit_cents`, and the API layer has already decoded each value into the field's Rust type. The same screen must later edit orders and every record type added afterward. If the service maintains a separate `match field_name { ... }` for each type, those branches drift away from the struct definitions.

With `qubit-reflect`, each record type derives its descriptor at the declaration site. One generic `apply_patch` looks up fields by name, checks the exact Rust type and declared access policy, and reads or replaces values on structs the application still owns. Primary keys can be declared `#[reflect(read_only)]` so the handler rejects changes before touching the struct.

The application still parses request text, validates business rules, authorizes the caller, and persists changes. Reflection checks Rust types, borrowing modes, and declared policies only. Custom types must derive or implement `Reflect`. A registry holds type and operation metadata, not application objects, and does not load plugins at runtime.

## Where to start

1. [Integrate a support console](#integrate-a-support-console) covers the record type, generic patch logic, and the load–patch–save path.
2. [Check the PATCH result](#check-the-patch-result) shows what a successful field change looks like and how rejected input is returned. The basic integration ends there.
3. Read on as needed: [Construct a new value from input](#construct-a-new-value-from-input), [Invoke methods](#invoke-methods), [Discover types and extension capabilities](#discover-types-and-extension-capabilities), or [Choose features and access boundaries](#choose-features-and-access-boundaries).

Registry snapshots, facades, and migration notes come after the basic path.

## Integrate a support console

<!-- reflect-source: examples/field_patch.rs -->
```rust
use qubit_reflect::FieldAccessError;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectedMut;
use qubit_reflect::ReflectedOwned;
use qubit_reflect::ReflectedRef;
use qubit_reflect::TypeDescriptor;

#[derive(Reflect)]
#[reflect(crate = qubit_reflect)]
struct Customer {
    #[reflect(read_only)]
    id: u64,
    email: String,
    credit_limit_cents: u64,
}

/// Runs the example and panics if a business assertion or reflection operation
/// fails.
fn main() {
    let descriptor = TypeDescriptor::of::<Customer>();
    let mut customer = Customer {
        id: 1001,
        email: String::from("ada@example.com"),
        credit_limit_cents: 50_000,
    };

    let email = descriptor.field("email").expect("derived field");
    email
        .set(
            ReflectedMut::new(&mut customer),
            ReflectedOwned::new(String::from("ada@corp.example")),
        )
        .expect("exactly typed replacement");
    assert_eq!(customer.email, "ada@corp.example");

    let current = email.get(ReflectedRef::new(&customer)).expect("shared read");
    assert_eq!(
        current.downcast_ref::<String>().map(String::as_str),
        Some("ada@corp.example")
    );

    let id = descriptor.field("id").expect("derived field");
    let failure = id
        .set(ReflectedMut::new(&mut customer), ReflectedOwned::new(2002_u64))
        .expect_err("read-only field rejects replacement");
    assert!(matches!(failure.error(), FieldAccessError::ReadOnly { .. }));
    assert_eq!(customer.id, 1001);
}
```


Add the dependency:

```toml
[dependencies]
qubit-reflect = "0.1"
```

The same-package examples explicitly use `#[reflect(crate = qubit_reflect)]`
and `#[reflect_impl(crate = qubit_reflect)]`: automatic crate discovery may
otherwise select the example executable as `crate`. From this source checkout,
Cargo still resolves the optional `qubit-datatype` and `qubit-id` sibling path
manifests when `qubit-types` is disabled. Prepare them before Cargo commands
and verify resolution:

```bash
./.infra/tools/prepare-local-path-dependencies.sh
cargo metadata --locked --format-version 1
```

A successful metadata command confirms Cargo can read both sibling manifests.
This repository setup applies only to source checkouts; users of the published
crate need no preparation script. Now run the native examples:

```bash
cargo run --example field_patch
cargo run --example customer_patch
cargo run --example support_action
```

All three targets declare `required-features = ["derive"]`; a runtime-only build
skips them. Their source files are included in the packaged runtime crate, so the
same commands work in an extracted package when `derive` is enabled. The version
dependency above selects a published release; source-checkout execution does not
prove that this checkout has been published or installed from a registry.

The record type, generic patch logic, and customer service are separate modules. `CustomerRepository` stands for the interface that talks to real storage. Integration has three steps: define the editable record type, write type-agnostic patch logic, and load, apply every change, then save.

### Define the editable record type

The listing below condenses `src/customers/model.rs`, `src/patch.rs`, and `src/customer_service.rs` into one library file for documentation. This is library code without a program entry point, so it is compiled without execution:

```rust,no_run
// src/customers/model.rs
pub mod customers {
    use qubit_reflect::Reflect;

    #[derive(Reflect)]
    pub struct Customer {
        #[reflect(read_only)]
        pub id: u64,
        pub email: String,
        pub display_name: String,
        pub credit_limit_cents: u64,
    }
}

// src/patch.rs
pub mod patch {
    use qubit_reflect::{FieldAccessError, Reflect, ReflectedMut, ReflectedOwned, TypeDescriptor};

    pub struct FieldChange {
        pub field: String,
        pub value: ReflectedOwned,
    }

    pub enum PatchError {
        UnknownField { field: String, value: ReflectedOwned },
        ReadOnly { field: String, value: ReflectedOwned },
        TypeMismatch { field: String, value: ReflectedOwned },
        Failed { field: String, error: FieldAccessError },
    }

    pub fn apply_patch<T: Reflect>(target: &mut T, changes: Vec<FieldChange>) -> Result<(), PatchError> {
        let descriptor = TypeDescriptor::of::<T>();
        for change in changes {
            let field_name = change.field;
            let Some(field) = descriptor.field(&field_name) else {
                return Err(PatchError::UnknownField { field: field_name, value: change.value });
            };
            if let Err(failure) = field.set(ReflectedMut::new(target), change.value) {
                let (error, recovery) = failure.into_parts();
                return Err(match (error, recovery) {
                    (FieldAccessError::ReadOnly { .. }, Some(recovery)) => {
                        PatchError::ReadOnly { field: field_name, value: recovery.into_value() }
                    }
                    (FieldAccessError::ValueTypeMismatch { .. }, Some(recovery)) => {
                        PatchError::TypeMismatch { field: field_name, value: recovery.into_value() }
                    }
                    (error, _) => PatchError::Failed { field: field_name, error },
                });
            }
        }
        Ok(())
    }
}

// src/customer_service.rs
pub mod customer_service {
    use crate::customers::Customer;
    use crate::patch::{FieldChange, PatchError, apply_patch};

    pub trait CustomerRepository {
        fn load(&self, id: u64) -> Result<Customer, Box<dyn std::error::Error>>;
        fn save(&self, customer: &Customer) -> Result<(), Box<dyn std::error::Error>>;
    }

    pub enum UpdateError {
        Patch(PatchError),
        Repository(Box<dyn std::error::Error>),
    }

    pub fn update_customer(
        repository: &dyn CustomerRepository,
        id: u64,
        changes: Vec<FieldChange>,
    ) -> Result<(), UpdateError> {
        let mut customer = repository.load(id).map_err(UpdateError::Repository)?;
        apply_patch(&mut customer, changes).map_err(UpdateError::Patch)?;
        repository.save(&customer).map_err(UpdateError::Repository)
    }
}
```

`#[derive(Reflect)]` generates structural metadata and access adapters at the declaration site. `apply_patch` works for every type that derives `Reflect`. The API layer authorizes the caller, decodes request text into `ReflectedOwned::new(the_field_rust_type)`, and calls `update_customer`. When it returns before `save`, nothing has been persisted. Reflection does not parse strings, coerce numbers, or infer `Into`.

### Types used on this path

| Type | Role in the support PATCH path |
| --- | --- |
| `Reflect` | Supplies reflection descriptors from a declaration; derive supports structs and enums. |
| `TypeDescriptor` | Immutable structural metadata and field lookup for a concrete type. |
| `FieldDescriptor` | Checked read, mutable borrow, or replacement for one field. |
| `ReflectedRef` / `ReflectedMut` | Pass a shared or exclusive borrow of the target into adapters. |
| `ReflectedOwned` | Pass ownership of a replacement or construction input. |
| `FieldSetFailure` | Reports a failed replacement and, on pre-execution rejection, returns the unused owned value. |

Repeated calls to `TypeDescriptor::of::<T>()` for the same concrete type return the same immutable root. Direct field access for a known type needs no registry initialization.

## Check the PATCH result

The minimal example updates the email, reads it back as `"ada@corp.example"`, and confirms the read-only `id` cannot be replaced. The program prints no application output; passing assertions and a normal exit are the success signal for that exercise.

### What success looks like

Before generated code runs, `set` checks the target type, access policy, and the replacement's exact `TypeId`. Read-only fields and type mismatches fail during those pre-execution checks; `FieldSetFailure::into_parts()` can return the unused owned value. Recovery retains field identity but not a borrow of the target; you can borrow `customer` again after the failed call ends.

| Stage | What the application sees | What to do |
| --- | --- | --- |
| `field("email")` missing | `None`; the console may use a query name that differs from the source name after `rename`. | Check configuration and `rust_name()`; do not call field operations. |
| Pre-execution `set` rejection | `FieldSetFailure` recovery is `Some`; the replacement was not consumed. | Read the structured error, prompt for corrected input, recover the value if needed. |
| Error after the adapter took ownership | Recovery is `None`. | Inspect the error and business state; do not assume the original value is retryable. |

`get` needs a shared borrow; `get_mut` and `set` need an exclusive borrow. Reflection does not parse strings, coerce numbers, or infer `Into`. If persistence runs after field replacement, a failed `save` is the application's problem; field adapters do not manage database transactions.

## Construct a new value from input

When the console creates a record, it may have decoded field values but no `Customer` instance yet. For a named struct, supply every constructible field by query name:

```rust
use qubit_reflect::{NamedConstructionInput, Reflect, ReflectedOwned, TypeDescriptor};

#[derive(Reflect)]
struct Customer {
    id: u64,
    email: String,
    display_name: String,
    credit_limit_cents: u64,
}

fn main() {
    let customer = TypeDescriptor::of::<Customer>()
        .construct_struct(NamedConstructionInput::new([
            ("id", ReflectedOwned::new(1001_u64)),
            ("email", ReflectedOwned::new(String::from("ada@example.com"))),
            ("display_name", ReflectedOwned::new(String::from("Ada Lovelace"))),
            ("credit_limit_cents", ReflectedOwned::new(50_000_u64)),
        ]))
        .expect("complete, exactly typed input")
        .downcast::<Customer>()
        .unwrap_or_else(|_| unreachable!("the descriptor constructs Customer"));
    assert_eq!(customer.email, "ada@example.com");
}
```

Use `construct_tuple` or `construct_unit` for the corresponding struct shape;
an enum `VariantDescriptor` provides the same three construction methods.
Construction validates shape, names or indices, duplicates, missing inputs,
policy, and exact types before it consumes owned values. On failure,
`ConstructionRecovery` returns the supplied inputs in caller order. A struct
updater follows the same all-or-nothing validation rule, including for types
that implement `Drop`.

### Constructing empty structs

| Declaration | Descriptor shape | Construction entry |
| --- | --- | --- |
| `struct A;` | `StructKind::Unit` | `construct_unit()` |
| `struct B {}` | `StructKind::Named` | `construct_struct(NamedConstructionInput::new([]))` |
| `struct C();` | `StructKind::Tuple` | `construct_tuple(TupleConstructionInput::new([]))` |

The distinction also applies to empty const-generic structs. A wrong shape returns a construction
error, never a value of a different type or an internal assertion failure.

## Invoke methods

When the console must trigger a business operation, use `#[reflect_impl]` to
generate method metadata, then use the same registry for lookup and invocation.
The independent counter example below adds `2` to `1`, producing `3`. Passing
the string `"2"` fails, returns that string, and leaves the counter at `3`.
Converting UI input to `u64` remains the application's responsibility.

### Invoke a service and recover invalid input

Use the same registry for lookup and execution. `Err(InvocationUnavailable)` denotes an unavailable entry and preserves the input; `Ok(Err(InvocationFailure))` denotes a failed invocation; decode successful output by its exact type.

```rust
use qubit_reflect::{Invocation, InvocationOutput, Reflect, ReflectedMut, ReflectedOwned,
                    ReflectRegistry, TypeDescriptor, reflect_impl};
use qubit_reflect::descriptor::MethodLookup;
use qubit_reflect::invoke::{InvocationArg, InvocationErrorKind};

#[derive(Reflect)]
struct Counter { value: u64 }

#[reflect_impl]
impl Counter {
    fn add(&mut self, amount: u64) -> u64 {
        self.value += amount;
        self.value
    }
}

fn main() {
    let registry = ReflectRegistry::initialize().expect("valid declarations");
    let MethodLookup::Unique(method) = TypeDescriptor::of::<Counter>()
        .methods_named_in(&registry, "add") else { panic!("unique method") };
    let mut counter = Counter { value: 1 };
    {
        let invocation = Invocation::borrowed_mut(ReflectedMut::new(&mut counter),
            [InvocationArg::Owned(ReflectedOwned::new(2_u64))]);
        let output = method.invoke_local(&registry, invocation)
            .expect("statically supported signature")
            .expect("valid receiver and arguments");
        let InvocationOutput::Owned(value) = output else { panic!("owned output") };
        assert_eq!(value.downcast::<u64>().unwrap_or_else(|_| panic!("u64")), 3);
    }

    {
        let invalid = Invocation::borrowed_mut(ReflectedMut::new(&mut counter),
            [InvocationArg::Owned(ReflectedOwned::new(String::from("2")))]);
        let failure = method.invoke_local(&registry, invalid)
            .expect("static entry still exists").err().expect("exact types required");
        assert!(matches!(failure.error().kind(), InvocationErrorKind::ArgumentTypeMismatch { .. }));
        let (receiver, arguments) = failure.into_recovery().into_parts();
        drop(receiver);
        let InvocationArg::Owned(value) = arguments.into_vec().pop().unwrap() else {
            panic!("original owned input")
        };
        assert_eq!(value.downcast::<String>().unwrap_or_else(|_| panic!("String")), "2");
    }
    assert_eq!(counter.value, 3);
}
```

### Recover an input when the selected entry is unavailable

The support action does not request panic capture. This independent program first
tries `invoke_catching_local`, receives `CatchingNotRequested`, takes back the
complete `Invocation`, and explicitly retries through the ordinary local entry.
The customer is suspended and the original reason is stored. This fallback is an
application policy: ordinary invocation can propagate a method panic.

<!-- reflect-source: examples/support_action.rs -->
```rust
use qubit_reflect::Invocation;
use qubit_reflect::InvocationOutput;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectRegistry;
use qubit_reflect::ReflectedMut;
use qubit_reflect::ReflectedOwned;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::MethodLookup;
use qubit_reflect::invoke::InvocationArg;
use qubit_reflect::invoke::InvocationDispatchReason;
use qubit_reflect::reflect_impl;

#[derive(Reflect)]
#[reflect(crate = qubit_reflect)]
struct Customer {
    id: u64,
    suspended: bool,
    suspension_reason: Option<String>,
}

#[reflect_impl(crate = qubit_reflect)]
impl Customer {
    /// Suspends this customer with `reason`, returning false if already
    /// suspended.
    fn suspend(&mut self, reason: String) -> bool {
        if self.suspended {
            return false;
        }
        self.suspended = true;
        self.suspension_reason = Some(reason);
        true
    }
}

/// Runs the example and panics if a business assertion or reflection operation
/// fails.
fn main() {
    let registry = ReflectRegistry::initialize().expect("valid reflection declarations");

    let MethodLookup::Unique(suspend) = TypeDescriptor::of::<Customer>().methods_named_in(registry, "suspend") else {
        panic!("exactly one method named suspend")
    };

    let mut customer = Customer {
        id: 1001,
        suspended: false,
        suspension_reason: None,
    };
    {
        let invocation = Invocation::borrowed_mut(
            ReflectedMut::new(&mut customer),
            [InvocationArg::Owned(ReflectedOwned::new(String::from(
                "chargeback dispute",
            )))],
        );
        let Err(unavailable) = suspend.invoke_catching_local(registry, invocation) else {
            panic!("panic capture was not requested for this method")
        };
        assert!(matches!(
            unavailable.reason(),
            InvocationDispatchReason::CatchingNotRequested
        ));
        let invocation = unavailable.into_invocation();
        let output = suspend
            .invoke_local(registry, invocation)
            .expect("local invocation entry is available")
            .expect("valid receiver and arguments");
        let InvocationOutput::Owned(changed) = output else {
            panic!("owned output")
        };
        assert!(changed.downcast::<bool>().unwrap_or_else(|_| panic!("bool")));
    }
    assert!(customer.suspended);
    assert_eq!(customer.suspension_reason.as_deref(), Some("chargeback dispute"));
}
```

`InvocationDispatchResult<I, R>` is `Result<R, InvocationUnavailable<I>>`.
An outer `Err` occurs before input binding, validation, or user code;
`into_parts()` returns the triple `(mode, reason, invocation)`, while `into_invocation()`
returns just the input. `NoAdapter` retains all static descriptor reasons;
`MissingEntry` means this mode has no entry; catching also distinguishes
`CatchingNotRequested` and `PanicAbort`. Ordinary calls return `Ok(Ok(output))`
on success or `Ok(Err(InvocationFailure))` for validation errors. The counter
example above uses the existing `InvocationRecovery` to retrieve the wrong
string; that recovery contract is unchanged.

Catching adds a separate method-panic layer:
`Ok(Ok(Ok(output)))` succeeds, `Ok(Err(InvocationFailure))` reports validation,
`Ok(Ok(Err(InvocationPanic)))` reports a caught method-body panic, and outer `Err`
reports an unavailable catching entry. A caught panic does not restore inputs
already consumed by the method or undo side effects.

Pinned borrowed calls use `PinnedRefInvocation<T, Local>` or
`PinnedMutInvocation<T, Local>`. The supplied `T` must exactly match the receiver
required by the pinned adapter; mismatch returns outer
`PinnedReceiverTypeMismatch` before binding. Recovery preserves the original
`Pin` and arguments, including caller order and named bindings, without unpinning
or manufacturing a new pin proof. Ordinary validation uses the existing pinned
recovery types. Outputs and futures borrow inputs as their signatures require,
never the registry; recovering a mutable receiver still retains its exclusive
borrow until the recovered input is consumed or dropped.

Async invocation creates and returns a future; the application chooses its
executor and polls it. Panics while polling that future are **not caught** by the
invocation catching boundary. Async methods cannot request `catch_unwind`.
With `panic=abort`, catching is unavailable (`PanicAbort` when an ordinary entry
exists and catching was requested); aborting a method cannot be recovered by
`catch_unwind`.

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

Conditional compilation is applied before reflection validation. Put ordinary
`#[cfg(...)]` and `#[cfg_attr(...)]` attributes on the declaration or member;
inactive members are absent from both generated reflection metadata and the
compiled Rust item. A platform-specific method may refer to a type that exists
only on that platform, provided the same condition guards the method.
`#[reflect(no_invoke)]` has a different purpose: the method remains in the
descriptor, but reflection does not generate a dynamic call adapter.

Look up a `MethodInstanceDescriptor` through the registry or an effective type
view, then call `invoke_local(registry, invocation)` with the same explicit registry. Positional arguments are
the canonical form. The runtime validates receiver, argument count, passing
mode, and exact types in that order; a failure before user code returns the
complete `InvocationRecovery`.

Generic and blanket implementations register definition metadata. To make a
finite concrete generic case callable or effective, declare
`#[reflect(specialize(...))]`. `#[reflect(thread_safe)]` requests a
thread-safe adapter and is accepted only when the generated Rust bounds prove
the receiver, inputs, owned output, and future boundary. A thread-safe value
can be downgraded to local mode, never upgraded by a runtime flag.

### Explicit generic specialization

Register a finite concrete impl for `Service<u8>`, then look up and invoke it through that concrete type.

```rust
use qubit_reflect::{Invocation, InvocationOutput, Reflect, ReflectRegistry, TypeDescriptor, reflect_impl};
use qubit_reflect::descriptor::MethodLookup;

#[derive(Reflect)]
struct Service<T> { value: T }

#[reflect_impl(specialize(T = u8))]
impl<T> Service<T> {
    fn answer() -> u8 { 42 }
}

fn main() {
    let registry = ReflectRegistry::initialize().unwrap();
    let MethodLookup::Unique(method) = TypeDescriptor::of::<Service<u8>>()
        .methods_named_in(&registry, "answer") else { panic!("explicit specialization") };
    let output = method.invoke_local(&registry, Invocation::associated([])).unwrap().unwrap();
    let InvocationOutput::Owned(value) = output else { panic!("owned output") };
    assert_eq!(value.downcast::<u8>().unwrap_or_else(|_| panic!("u8")), 42);
}
```

## Discover types and extension capabilities

When you know the type, obtain its descriptor directly. Use a registry to
discover linked types, find methods, or resolve extension operations for a
type. A registry holds type and operation metadata, not application objects;
it does not load dynamic plugins.

Normally, `ReflectRegistry::initialize()` collects registrations already linked
into the program. Build an isolated snapshot only when a library or test needs
an explicit fact set. A capability is a type extension, such as a typed `Clone`
or `Default` operation. Resolving capabilities and registering type membership
are separate concerns.

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
    let _methods = descriptor.methods_in(&snapshot);
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
definition-level extensions. `TypeDefinitionDescriptor` describes a generic declaration; concrete instances
retain resolved type arguments. Definition fields contain `TypeExpression`
values and provide no value-access adapters. `TypeRef` resolves a target only
when generated Rust code can prove the concrete type; it never guesses from
a string name.

`Clone` and `Default` are typed capabilities. Register them only where their
Rust bounds hold, then query with `clone_key()` or `default_key()`. Other
safely generated special receiver forms require an exact `ReceiverAdapter` in
the selected registry, supplied by global registration or an explicit builder.
A missing capability leaves the entry point present: invocation returns
`Ok(Err(ReceiverAdapterUnavailable))` with the original inputs. Statically
unsupported signatures have no entry point.

In `#[reflect(capabilities(...))]`, the bare names `Clone`, `Default`, `Send`,
and `Sync` select built-in capabilities. A qualified path calls the named
custom provider, including when its final segment matches one of those names:
`#[reflect(capabilities(my_crate::Clone))]` invokes
`my_crate::Clone::<Self>()`.

### Decide whether a capability can execute

| `capability_lookup` state | Meaning | Application action |
| --- | --- | --- |
| `Missing` | No matching fact. | Choose the application's fallback. |
| `FactOnly` | Metadata exists without an adapter. | Inspect metadata; do not call it. |
| `AdapterTypeMismatch` | The ID matches but the adapter Rust type differs. | Check the registered adapter and typed key. |
| `Found` | A type-matching adapter exists. | Invoke under that capability's contract. |

`capability` maps `Found` to `Ok(Some(adapter))` and `Missing` to `Ok(None)`;
`FactOnly` and `AdapterTypeMismatch` are errors. `capability_by_id` reads facts by
text ID; use `capability_origin` and `capability_source` to trace intrinsic facts
or registration fragments. Querying a capability does not make its target a
snapshot member.

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
use std::any::TypeId;

fn source(kind: &str, line: u32) -> FragmentIdentity {
    FragmentIdentity::new("example", "fixture", line, 1, kind, u64::from(line))
}

fn main() -> Result<(), qubit_reflect::RegistryError> {
    let target = TypeDescriptor::of::<u32>();
    let key = CapabilityKey::<u32>::new(
        CapabilityId::new("example.limit").expect("valid capability ID"),
    );
    let mut builder = RegistrySnapshotBuilder::new();
    builder.add_type_with_capabilities(
        target,
        vec![CapabilityDescriptor::with_adapter(key, 7_u32)],
        source("type", 10),
        source("capability", 11),
    );
    builder.add_type_capabilities(
        TypeDescriptor::of::<u64>(),
        vec![CapabilityDescriptor::with_adapter(key, 8_u32)],
        source("capability", 12),
    );
    let snapshot = builder.build()?;

    assert!(snapshot.get(target.type_id()).is_some());
    assert_eq!(
        snapshot
            .capability(target, key)
            .expect("valid capability declarations"),
        Some(&7),
    );
    assert_eq!(snapshot.types().len(), 1);
    assert_eq!(
        snapshot.capability_only_type_targets("example.limit")[0].0,
        TypeId::of::<u64>(),
    );
    assert!(target.methods_in(&snapshot).is_empty());
    Ok(())
}
```

The combined `add_type_with_capabilities` method registers a root and its
capabilities while keeping their source identities separate. An empty builder yields a
snapshot with no registered roots. Calling only `add_type_capabilities` creates
a capability-only snapshot: `types()` remains empty, while `capability()` and
`capability_by_id()` can still resolve the target. The other typed inputs are
`add_definition`, `add_trait`, `add_impl_definition`, `add_impl`, and
`add_definition_capabilities`.

Only descriptors added with `add_type` or `add_type_with_capabilities` are
snapshot type members. `ModelRegistry::from_reflect_registry` projects those
members, so a capability-only target does not become a model implicitly. The
example registers `u64` only for a capability and confirms it remains outside
`types()`. Use
`capability_only_type_targets(capability_id)` to list registered capability
targets absent from `types()`. The query matches the stable capability ID even
when the adapter type differs, and orders results by their source fragment.
For an optional metadata registration audit, query
`snapshot.capability_only_type_targets("qubit.model.metadata.v1")`.
The audit is optional for reflection itself, but model projection is stricter:
`ModelRegistry::from_reflect_registry` returns `UnregisteredModelTarget` if a
model-metadata capability points to a type that is not a member of the
snapshot. When generic-model metadata is enabled, the same rule applies to
definition targets. Inspect the matching `capability_only_*_targets` result
and its source fragment, then either add the intended model type or definition
to the snapshot or remove that metadata registration from this view. Capability
queryability does not imply model membership.

For generic declarations, `definition_capabilities(id)` reports capability
facts independently from definition membership. It returns `None` when the
snapshot has neither the definition nor capability facts for that ID,
`Some(empty)` for a definition member without capabilities, and
`Some(nonempty)` when capability facts exist. The last case also includes a
capability-only target: `definition_capabilities(id)` can return facts while
`definition(id)` is `None`. Use `definition(id).is_some()` to check snapshot
membership. The typed `definition_capability` and textual
`definition_capability_by_id` queries can also return a matching adapter or
descriptor from a capability-only target; they return `None` when the target
has no matching facts or capability. Use `definition(id)` separately when
membership matters. A fact without an executable adapter produces
`CapabilityAccessError::FactOnly`, and a typed key with a different adapter
type produces `AdapterTypeMismatch`.

Pass the resulting snapshot explicitly to `impls_in`, `methods_in`, or
`methods_named_in` when a property or method query must use that exact set of
facts. `build()` validates all identities, links, and capability conflicts as
one transaction. It returns a `RegistryError` without publishing partial
state; provide stable `FragmentIdentity` values so duplicate or changed source
identities are diagnosable. For a conflict, inspect
`conflicting_fragments()`, `capability_details()`, `capability_target()`, and
`capability_id()`. Intrinsic provider failures remain available through
`intrinsic_conflict()` and `Error::source()`.

The global entry point is for the complete set of linked registrations; a
conflict makes its initialization fail as a whole. A caller that needs an
isolated model view can instead add only its selected descriptors to an
explicit snapshot and pass that same snapshot to
`qubit_model_metadata::registry::ModelRegistry::from_reflect_registry`:

```text
let mut builder = RegistrySnapshotBuilder::new();
builder.add_type(
    TypeDescriptor::of::<MyModel>(),
    FragmentIdentity::new("example", "models", 1, 1, "type", 1),
);
let snapshot = builder.build()?;
let models = qubit_model_metadata::registry::ModelRegistry::from_reflect_registry(&snapshot)?;
```

This is an integration fragment: the application supplies `MyModel`, its model
metadata dependency, and the error-return context. An explicit snapshot starts
empty and does not automatically include every registration linked into the process.

## Choose features and access boundaries

### Choose dependency features

| Feature | Provides |
| --- | --- |
| `derive` (default) | The `Reflect`, `reflect`, and `reflect_impl` macros. |
| `ecosystem-types` | Reflection implementations for `BigDecimal`, `DateTime<Utc>`, `NaiveDate`, `NaiveTime`, and `Uuid`. |
| `qubit-types` | Reflection implementations for `qubit_id::Id` and `qubit_datatype::DataType`; the `qubit-id` default generator feature is disabled. |

Choose one of the following alternatives for the `qubit-reflect` entry in
`[dependencies]`; do not paste all three into one manifest:

```toml
# Runtime descriptors, dynamic values, and handwritten registration only.
qubit-reflect = { version = "0.1", default-features = false }
```

```toml
# Macros plus BigDecimal, chrono, and UUID reflection implementations.
qubit-reflect = { version = "0.1", features = ["ecosystem-types"] }
```

```toml
# Macros plus Qubit DataType and Id reflection implementations.
qubit-reflect = { version = "0.1", features = ["qubit-types"] }
```

`ecosystem-types` and `qubit-types` are independent opt-ins. Neither belongs to
the default feature set, so a runtime-only consumer does not compile those
dependency families or silently acquire their trait implementations.
`qubit-types` only provides reflection for `qubit_id::Id`; if the application
also uses ID generators, add a direct `qubit-id` dependency with the needed
features instead of relying on features enabled transitively by this crate.
If a facade or metadata crate generates descriptors for one of these external
types, that crate must enable the matching feature on its own
`qubit-reflect` dependency; re-exporting the macros does not enable type-family
implementations by itself.

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
metadata through downstream-owned custom capabilities and providers.
`qubit-reflect` neither defines nor interprets those domain semantics. This preserves the dependency direction from model
crates to `qubit-reflect`.

The type-level `#[reflect(thread_safe)]` below generates thread-safe field
access support. Thread-safe method adapters must be requested on the relevant
methods; wrapping a value in `SendReflected*` alone is not sufficient:

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

## Errors, diagnostics, and troubleshooting

The API avoids implicit conversion: it does not coerce numeric values, parse strings, infer `Into`, or manufacture `Send`/`Sync` after type erasure. Match structured error categories, not `Display` text. Inspect recovery before retrying.

| Symptom | What to check |
| --- | --- |
| Cargo reports a missing `qubit-datatype` or `qubit-id` path manifest | In a source checkout, run `./.infra/tools/prepare-local-path-dependencies.sh`, then `cargo metadata --locked --format-version 1`. Success confirms both sibling manifests resolve; registry users do not need this script. |
| `field("...")` returns `None` | Use the query name; `rename` changes it while `rust_name()` retains the source spelling. |
| A field operation fails | Verify the wrapper (`ReflectedRef` versus `ReflectedMut`), the field policy, and the replacement's exact type; then inspect `FieldSetFailure` recovery. |
| Construction fails | Check shape, duplicate or missing fields, names or indices, and each value's type; recover inputs from `ConstructionRecovery`. |
| A method is visible but not callable | Distinguish outer `Err(InvocationUnavailable)` (no entry; recover with `into_invocation()`) from `Ok(Err(InvocationFailure))` (pre-execution failure); generic methods need an explicit specialization. |
| Registry initialization or snapshot build fails | Inspect `RegistryError` and conflicting fragments; global initialization errors are cached for the process. |
| Cross-thread entry points unavailable | Request `thread_safe` on the type or method and use `SendReflected*` only where Rust bounds hold. |
| An external type has no `Reflect` implementation | Enable `ecosystem-types` or `qubit-types` on the crate that owns the reflection boundary. |
| A facade-based derive cannot resolve helpers | Preserve `#[reflect(crate = ...)]`, export exactly `__private::codegen_v3`, and align facade and runtime versions. |

Field access returns `FieldAccessError`. Pre-execution `set` rejections preserve the owned replacement in `FieldSetFailure`; errors after the adapter takes ownership have no recovery payload. Construction failures return `ConstructionRecovery`. Pre-execution invocation failures retain receiver and arguments in `InvocationRecovery`. Ordinary invocation always propagates method panics. To capture them, the method must opt in with `#[reflect(catch_unwind)]` and the caller must select an available catching entry.

## Boundaries and a practice checklist

- Keep reflection attributes on the declaration that owns the contract. Generated code can reach private fields; reflection policies do not replace application authorization.
- `TypeId`, descriptor addresses, query names, and trait markers are process-local identity, not serialization or cross-process model IDs.
- Reflection does not parse request text, coerce numbers, infer `Into`, or upgrade local dynamic values to thread-safe mode at runtime.
- Unsafe functions, unsupported ABIs, variadics, unspecialized generics, and opaque `impl Trait` returns may be described but are not dynamically callable. Tuple and function-pointer descriptors support arities 0 through 32.
- Descriptors and per-type capability caches live for the whole process; initialization may allocate. Measure the paths your application uses if overhead matters.
- Tests should cover a successful field read and replace, unknown fields, read-only rejection, type mismatch recovery, method invocation failure, and registry initialization failure on conflicting declarations.

## Facade integration and migration

This section is for maintainers of dependency facades, procedural macros, or
older integrations. Applications using derives directly do not need to
configure the generated-code protocol.

### Integrate through a downstream facade or macro

A facade that directly hosts `qubit-reflect` derives exposes the versioned
generated-code protocol under the path expected by the derive. Public
application exports are an independent choice; this minimal example exports
the two types used by its callers. This is library code without a program entry point, so it is compiled without execution:

```rust,no_run
pub use qubit_reflect::Reflect;
pub use qubit_reflect::TypeDescriptor;

#[doc(hidden)]
pub mod __private {
    pub use qubit_reflect::__private::codegen_v3;
}
```

Declarations can then use `#[reflect(crate = my_facade)]`. Generated code needs
only the `codegen_v3` export; the facade does not need to re-export runtime
modules such as `descriptor`, `construct`, or `value`. Do not glob-re-export
`qubit_reflect` or its `__private` module: that turns unrelated implementation
details into the facade's API. A downstream procedural macro may give the same
module through exact item re-exports. `codegen_v3` is a
compiler-to-runtime protocol, not a supported handwritten construction API; a
future incompatible protocol receives a new versioned module.

Explicit snapshots do not change the generated-code protocol. Facades still
expose `__private::codegen_v3`; downstream `qubit-model-metadata` uses its
independent model metadata ABI `v7` in the current `qubit-model-metadata`
checkout. This is separate from the reflection `codegen_v3` protocol.

### Migrating effective capability queries

The existing query names now return `Result`; no error-swallowing compatibility entry remains.
`capabilities` returns `Result<&TypeCapabilities, CapabilityConflict>`. The typed `capability`
query returns `Result<Option<&A>, CapabilityAccessError>`, while `capability_by_id` returns
`Result<Option<&CapabilityDescriptor>, CapabilityConflict>`. Handle errors before testing for
absence. Typed access errors distinguish `FactOnly` and `AdapterTypeMismatch`; intrinsic
declaration conflicts are wrapped as `CapabilityAccessError::IntrinsicConflict`.
Conflicts retain their kind, capability ID, and both adapter TypeIds. Registration failures also expose
the original conflict through `RegistryError::intrinsic_conflict()` and `Error::source()`.

The convenience `capability` query maps `Found` to `Ok(Some(adapter))` and `Missing` to `Ok(None)`.
Fact-only descriptors return a `FactOnly` error, and typed keys with a different adapter type return an
`AdapterTypeMismatch` error. Use `capability_lookup` when you need to inspect all four states:
`Missing`, `FactOnly`, `AdapterTypeMismatch`, and `Found`. Invalid intrinsic capability sets return their
conflict error; these lookup states are distinct from an invalid set.
`capability_origin` reports `CapabilityOrigin::Intrinsic { type_id }` or `CapabilityOrigin::Registered { source }`, while
`capability_source` returns the contributing `FragmentIdentity` when one is registered. The corresponding
`definition_capability_origin` and `definition_capability_source` methods apply to generic declarations.
`type_capability_members` and `definition_capability_members` iterate only frozen registry members
and do not execute factories. Each result carries the lookup state, origin, and source; fact-only
and adapter-mismatch entries remain visible, while capability-only targets are audited separately
with `capability_only_type_targets` and `capability_only_definition_targets`. Intrinsic capability
sources use the member declaration when it exists, otherwise the earliest fragment that triggered
inspection. Effective queries for unregistered
concrete instances may execute an intrinsic factory, without inserting the instance into the snapshot.

Providers must depend only on static type facts, never on snapshots, time, or mutable external
configuration, and must not re-enter registry initialization. Generic intrinsic factories run outside
the cache-map lock; successes and conflicts are cached by concrete `TypeId` and shared by concurrent
queries. Provider panics still propagate; they do not become absence or `CapabilityConflict`.
Generated invocation adapters preserve receiver-capability resolution failures as structured
invocation errors and restore the receiver, values, names, and original caller ordering.
Invocation itself does not initialize a registry.

### Explicit invocation migration and troubleshooting

The twelve consuming methods on `descriptor::InvocationAdapter` and
`MethodInstanceDescriptor` now use outer `Result` instead of `Option`: migrate
`None` to outer `Err`, recover the input explicitly, and migrate `Some(result)`
to `Ok(result)`. This is a breaking change; the inner validation and catching
contracts and function-pointer ABI remain unchanged. See the
[stability note](2026-09-07-qubit-reflect-api-stability.md).

Every `invoke_*` entry now requires a registry. If lookup succeeds but
invocation returns `ReceiverAdapterUnavailable`, check the selected snapshot
for the exact receiver capability and invocation mode. A static entry does not
guarantee that capability exists. The same key may select different adapters in
two snapshots without cross-contamination; global failure does not affect
valid local calls. Outputs and futures do not borrow the registry, but remain
constrained by input lifetimes.

Old `codegen_v2` facades fail compilation: migrate the exact export to
`codegen_v3`. Model metadata ABI `v7` and `definition_provider_v2` remain
independent. Debug prints structural facts without running providers; providers
themselves must not re-enter initialization.

## Further reading

- [README](../README.md) · [Architecture and design](2026-09-03-qubit-reflect-design.md) · [API reference](https://docs.rs/qubit-reflect)
- [Chinese user guide](user_guide.zh_CN.md) · [中文 README](../README.zh_CN.md)
