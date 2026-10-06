# Qubit Reflect (`rs-reflect`)

[![Rust CI](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-reflect/coverage-badge.json)](https://qubit-ltd.github.io/rs-reflect/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-reflect.svg?color=blue)](https://crates.io/crates/qubit-reflect)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

<!-- reflect-contract: facade.explicit=qubit_reflect -->
<!-- reflect-contract: examples.native=cargo-example -->

`qubit-reflect` solves a common problem inside a back-office service: a support console sends `PATCH` requests that name the field to change, such as `email` or `credit_limit_cents`, and the same screen must work for customers, orders, and every record type added later. Hand-written `match field_name { ... }` blocks per type drift out of sync with the structs they describe. This crate lets each record type derive its own descriptor at the declaration site, so one generic handler can find a field by name, check the exact Rust type and the declared access policy, and then read or replace the value on a struct the application still owns. Reflection macros generate the code on stable Rust. The crate does not parse request text into Rust values, serialize objects, or enforce business rules and authorization.

## A customer service example

A customer service loads `Customer { id, email, display_name, credit_limit_cents }` from its repository. The API layer has already decoded the request into typed changes, for example `("email", String)`. A generic `apply_patch` looks each field up by name on `TypeDescriptor::of::<Customer>()` and calls `set`. The `id` field is declared `#[reflect(read_only)]`, so the handler rejects attempts to change the primary key before touching the struct; a wrong value type is rejected the same way and the untouched value is handed back to the caller. Only after every change is applied does the service save the customer. Adding an `Order` type to the same console means deriving `Reflect` on it; the handler does not change.

## Installation

```toml
[dependencies]
qubit-reflect = "0.2"
```

Reflection macros are enabled by default. For runtime-only use or reflection of third-party types, see the [dependency profiles](doc/user_guide.md#choose-dependency-features) in the user guide.

## Quick start

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


The complete, runnable load–patch–save flow lives in
[`examples/customer_patch.rs`](examples/customer_patch.rs). Method lookup and
invocation are shown in [`examples/support_action.rs`](examples/support_action.rs).
These are native examples of the runtime package. In same-package example targets,
automatic crate discovery can resolve the runtime to `crate`, which refers to the
example executable. The declarations explicitly select the runtime facade with
`#[reflect(crate = qubit_reflect)]` and `#[reflect_impl(crate = qubit_reflect)]`.
In a source checkout, Cargo resolves the optional `qubit-datatype` and
`qubit-id` path manifests even when `qubit-types` is disabled. Prepare those
sibling checkouts first, then confirm Cargo can read both manifests:

```bash
./.infra/bin/prepare-local-path-dependencies.sh
cargo metadata --locked --format-version 1
```

Successful metadata output confirms path resolution. Users of the published
crate do not need this repository script. Now run the examples from this
source checkout:

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

The complete load–patch–save service belongs in the [support-console guide
scenario](doc/user_guide.md#integrate-a-support-console); it explains where
`CustomerRepository` connects to application storage and how rejected changes
are handled. The shorter [`customer_patch` example](examples/customer_patch.rs)
shows the runnable path.

The API layer decodes request text into each field's Rust type and checks that the caller may edit the record; `apply_patch` only verifies field names, access policy, and exact types. When a change is rejected, earlier changes may already sit in the in-memory struct, but nothing has been saved because `update_customer` returns before `save`. See the [user guide](doc/user_guide.md#errors-diagnostics-and-troubleshooting) for the full error model.

### Register reflected actions at startup

The same console offers action buttons such as “Suspend customer”. Their configuration table stores a method name, and the backend invokes that method on the loaded record. Annotate the implementation with `#[reflect_impl]`, call `ReflectRegistry::initialize()` once during application startup, then resolve actions with `TypeDescriptor::methods_named_in` and `invoke_local`. The runnable flow is in [`examples/support_action.rs`](examples/support_action.rs). `methods_named_in` returns `Missing`, `Unique`, or `Ambiguous`; handle each case in application code. `invoke_local` returns an outer `Err(InvocationUnavailable)` when no entry is available, preserving the complete original input, and `Ok(Err(InvocationFailure))` when receiver or argument validation failed before the method body ran. Ordinary invocation propagates method-body panics; capture requires both `#[reflect(catch_unwind)]` and an available catching entry selected by the caller. A runnable version is in [`examples/support_action.rs`](examples/support_action.rs); registry initialization and method lookup are covered in the [user guide](doc/user_guide.md#invoke-methods).

## What it provides

- `#[derive(Reflect)]`, `#[reflect]`, and `#[reflect_impl]` generate immutable descriptors for structs, enums, traits, and implementations at the declaration site, on stable Rust.
- `TypeDescriptor` and `FieldDescriptor` provide checked field reads, mutable borrows, and replacements through `ReflectedRef`, `ReflectedMut`, and `ReflectedOwned`; `construct_struct`, `construct_tuple`, and `construct_unit` build values from named or positional inputs.
- Field attributes `rename`, `read_only`, `skip`, `no_construct`, `opaque`, and method attribute `no_invoke` limit a dynamic operation while keeping the structural fact visible; ordinary `#[cfg]` removes the member entirely.
- Validation failures before generated code runs return structured errors together with the caller's untouched inputs: `FieldSetFailure`, `ConstructionRecovery`, and `InvocationRecovery`.
- `ReflectRegistry::initialize()` assembles one deterministic, immutable registry from linked declarations; `RegistrySnapshotBuilder` builds an isolated snapshot from explicit facts for libraries and tests. Both resolve methods and typed capabilities such as `Clone` and `Default`, and report whether a capability is `Missing`, `FactOnly`, `AdapterTypeMismatch`, or `Found`.
- Generic declarations are described as definitions; `#[reflect(specialize(...))]` makes a finite concrete instance callable.
- `Local` dynamic values are the default; `SendReflected*` wrappers and `#[reflect(thread_safe)]` provide a thread-safe boundary only where generated code proves the required `Send + Sync` bounds.
- Built-in descriptors for primitives, text, tuples, arrays, `Option`, sequences, sets, maps, smart pointers, and function pointers; optional `ecosystem-types` and `qubit-types` features add `BigDecimal`, `chrono`, `Uuid`, and Qubit `Id`/`DataType` implementations.

`Option<T>` descriptors expose their element type and a checked borrowed
projection for built-in `Option<T>` values. The projection returns the inner
borrow for `Some`, `None` for an absent value, and a type mismatch for an
unrelated value. Descriptors assembled from structural facts expose the same
shape but report projection as unavailable because they have no runtime
adapter. See [optional values](doc/user_guide.md#optional-values).

Reflection is deliberately bounded. It does not coerce numeric values, parse strings, infer `Into`, or upgrade a local dynamic value to thread-safe mode. Arbitrary Rust types are not reflectable until they derive or implement `Reflect`. `TypeId`, descriptor addresses, and trait markers are process-local identity, not serialization or cross-process model identifiers. Generated access code can reach private fields, so reflection policies do not replace application authorization. Unsafe functions, unsupported ABIs, variadics, unspecialized generics, and opaque `impl Trait` returns are described but not callable. Tuple and function-pointer descriptors support arities 0 through 32. Descriptors and per-type capability caches stay alive for the whole process, and initialization may allocate; measure the paths your application uses if overhead matters. See the [user guide](doc/user_guide.md#boundaries-and-a-practice-checklist).

## Learn more

- [User guide](doc/user_guide.md)
- [Current design (0.2)](doc/2026-10-07-qubit-reflect-current-design.md) · [Historical design (0.1)](doc/2026-09-03-qubit-reflect-design.md) · [Derive contract matrix](doc/derive-contract-matrix.md)
- [API reference](https://docs.rs/qubit-reflect)
- [中文 README](README.zh_CN.md) · [中文用户手册](doc/user_guide.zh_CN.md)

## Testing

These commands run from the source checkout. Cargo resolves the optional
`qubit-datatype` and `qubit-id` path manifests during workspace loading, so
prepare local path dependencies first as shown above. Published-crate users do
not need that script.

```bash
# Run tests with the default feature set
cargo test

# Run tests with all declared features
cargo test --all-features

# Project CI checks
./.infra/bin/ci-check.sh

# Check code coverage
./.infra/bin/coverage.sh
```

## License

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for the
full license text.

## Contributing

Contributions are welcome. Please follow the Rust API guidelines, keep public
API documentation and tests current, and run `./.infra/bin/align-ci.sh` to format code and
`./.infra/bin/ci-check.sh` to satisfy CI requirements before submitting a pull request.

## Author

**Haixing Hu** - *Qubit Co. Ltd.*

Repository: [https://github.com/qubit-ltd/rs-reflect](https://github.com/qubit-ltd/rs-reflect)
