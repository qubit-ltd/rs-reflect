# Qubit Reflect (`rs-reflect`)

[![Rust CI](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-reflect/coverage-badge.json)](https://qubit-ltd.github.io/rs-reflect/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-reflect.svg?color=blue)](https://crates.io/crates/qubit-reflect)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

<!-- reflect-contract: runtime.direct=qubit_reflect -->
<!-- reflect-contract: examples.native=cargo-example -->

`qubit-reflect` brings opt-in, macro-based reflection to Rust. Add `#[derive(Reflect)]` to a struct or enum, `#[reflect]` to a trait, or `#[reflect_impl]` to an implementation. The macros generate structural metadata and checked runtime adapters, so application and framework code can discover types and members, access fields, construct values, and invoke supported methods without maintaining a separate handwritten description of each type. It works on stable Rust and keeps Rust's ownership and type checks at the dynamic boundary.

## Why use reflection?

Rust's ordinary type information cannot enumerate an application's fields or call methods selected by name at runtime. Frameworks that need this behavior often keep a parallel table of field names, types, and access functions. That table can drift as declarations change. Here the declaration and its reflection contract live together: the macros generate descriptors and adapters at compile time, while `TypeDescriptor` and the registry expose them at runtime.

For example, a generic record editor can look up `Customer.email` by name, inspect its type, and replace its value. The same editor can work with another reflected type without a new field-name `match`. The application still decides how to parse input, authorize edits, validate business rules, and persist records. The [user guide](doc/user_guide.md) follows this example through a complete load–patch–save workflow.

## Installation

```toml
[dependencies]
qubit-reflect = "0.2"
```

Reflection macros are enabled by default. Each crate that derives reflection declares its own `qubit-reflect` dependency. For runtime-only use or reflection of third-party types, see the [dependency profiles](doc/user_guide.md#choose-dependency-features) in the user guide.

## Quick start

This example derives reflection for a type, finds a field through its generated descriptor, and reads and replaces an exactly typed value. `read_only` removes the write adapter for `id` while retaining its field metadata.

<!-- reflect-source: examples/field_patch.rs -->
```rust
use qubit_reflect::FieldAccessError;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectedMut;
use qubit_reflect::ReflectedOwned;
use qubit_reflect::ReflectedRef;
use qubit_reflect::TypeDescriptor;

#[derive(Reflect)]
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
These are native examples of the runtime package. The macros resolve the
`qubit-reflect` runtime directly, including in same-package example targets;
no crate-path helper is required.
Cargo resolves the optional `qubit-datatype` and `qubit-id` dependencies from
crates.io, so a source checkout does not need adjacent repositories. Verify
dependency resolution from the repository root:

```bash
cargo metadata --locked --format-version 1
```

Successful metadata output confirms the locked dependency graph resolves. Now
run the examples from this source checkout:

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

### Reflect methods and implementations

For method reflection, annotate an `impl` block with `#[reflect_impl]`. Initialize `ReflectRegistry` once, then use the selected registry to look up and invoke a supported method. [`examples/support_action.rs`](examples/support_action.rs) shows a method selected by name; [the guide](doc/user_guide.md#invoke-methods) explains lookup results, invocation errors, and input recovery. Use `#[reflect]` to describe a trait declaration and `#[reflect_impl]` for its implementation.

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
- [API reference](https://docs.rs/qubit-reflect)
- [中文 README](README.zh_CN.md) · [中文用户手册](doc/user_guide.zh_CN.md)

## Testing

These commands run from the source checkout. Cargo resolves optional
`qubit-datatype` and `qubit-id` dependencies from crates.io; no sibling checkout
preparation is required. Run the metadata command shown above first if you want
to verify dependency resolution.

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
