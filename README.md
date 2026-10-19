# Qubit Reflect (`rs-reflect`)

[![Rust CI](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-reflect/coverage-badge.json)](https://qubit-ltd.github.io/rs-reflect/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-reflect.svg?color=blue)](https://crates.io/crates/qubit-reflect)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

`qubit-reflect` brings opt-in, macro-based reflection to Rust. Add `#[Reflect]` to a struct, enum, or trait and `#[ReflectImpl]` to an inherent or trait implementation. The macros generate structural metadata and checked runtime adapters, so application and framework code can discover types and members, access fields, construct values, and invoke supported methods without maintaining a separate handwritten description of each type. It works on stable Rust and keeps Rust's ownership and type checks at the dynamic boundary.

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

This example uses `#[Reflect]` on a type and `#[ReflectImpl]` on its methods. It
finds fields through the type descriptor, reads and replaces exactly typed
values, then looks up and invokes `raise_credit_limit` by name. Safe field
operations reject the private `secret` field; the example uses its documented
`_unsafe` accessors with an explicit safety rationale. The skipped
`internal_note` remains part of the Rust struct but is absent from reflection
metadata.

<!-- reflect-source: examples/field_patch.rs -->
```rust
use std::error::Error;
use std::io;

use qubit_reflect::Invocation;
use qubit_reflect::InvocationOutput;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectRegistry;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::access::FieldAccessError;
use qubit_reflect::descriptor::MethodLookup;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;

#[Reflect]
struct Customer {
    pub id: u64,
    pub email: String,
    pub credit_limit_cents: u64,
    secret: String,
    #[reflect(skip)]
    internal_note: String,
}

#[ReflectImpl]
impl Customer {
    /// Raises the credit limit, returning false when the requested limit is not
    /// greater than the current one.
    pub fn raise_credit_limit(&mut self, credit_limit_cents: u64) -> bool {
        if credit_limit_cents <= self.credit_limit_cents {
            return false;
        }
        self.credit_limit_cents = credit_limit_cents;
        true
    }
}

/// Runs the example, returning reflection setup and operation failures to the
/// caller.
fn main() -> Result<(), Box<dyn Error>> {
    let registry = ReflectRegistry::initialize()?;
    let descriptor = TypeDescriptor::of::<Customer>();
    let mut customer = Customer {
        id: 1001,
        email: String::from("ada@example.com"),
        credit_limit_cents: 50_000,
        secret: String::from("private"),
        internal_note: String::from("not reflected"),
    };

    let email_field = descriptor
        .field("email")
        .ok_or(io::Error::other("missing reflected email field"))?;
    email_field.set(&mut customer, String::from("ada@corp.example"))?;
    assert_eq!(customer.email, "ada@corp.example");

    let current: &String = email_field.get(&customer)?;
    assert_eq!(current, "ada@corp.example");

    let secret_field = descriptor
        .field("secret")
        .ok_or(io::Error::other("missing reflected secret field"))?;
    assert!(matches!(
        secret_field.get::<_, String>(&customer),
        Err(FieldAccessError::NotPublic { .. })
    ));
    // SAFETY: this example owns Customer and intentionally demonstrates the
    // visibility bypass.
    assert_eq!(unsafe { secret_field.get_unsafe::<_, String>(&customer) }?, "private");
    // SAFETY: this example owns Customer and maintains the String field's type
    // invariant.
    unsafe { secret_field.set_unsafe(&mut customer, String::from("updated")) }?;
    assert_eq!(customer.secret, "updated");
    assert!(descriptor.field("internal_note").is_none());
    assert_eq!(customer.internal_note, "not reflected");

    let MethodLookup::Unique(raise_credit_limit) = descriptor.methods_named_in(registry, "raise_credit_limit") else {
        return Err(io::Error::other("expected exactly one method named raise_credit_limit").into());
    };
    {
        let invocation = Invocation::receiver_mut(&mut customer).arg(75_000_u64);
        let output = raise_credit_limit
            .invoke(registry, invocation)
            .map_err(|failure| io::Error::other(failure.to_string()))?;
        let InvocationOutput::Owned(changed) = output else {
            return Err(io::Error::other("raise_credit_limit did not return an owned value").into());
        };
        let changed = DynamicOwned::<Local>::downcast::<bool>(changed)
            .map_err(|_| io::Error::other("raise_credit_limit did not return bool"))?;
        assert!(changed);
    }
    assert_eq!(customer.credit_limit_cents, 75_000);
    Ok(())
}
```

This code completes one reflection flow in order: obtain the `Customer` type
descriptor, find a field by name, write and read the email, confirm that safe
access rejects a private field and that a skipped field is absent from metadata,
then find and invoke a method by name. Passing assertions and a normal exit are
the observable result.

Reflection is responsible here for member discovery and for checking visibility
and exact Rust types before dynamic operations. Request parsing, caller
authorization, business validation, and persistence remain application
responsibilities. For a load–patch–save workflow built on field access, continue
with the runnable [`customer_patch` example](examples/customer_patch.rs) and the
[support-console integration guide](doc/user_guide.md#integrate-a-support-console).

## Core capabilities and boundaries

`qubit-reflect` provides these core capabilities:

- `#[Reflect]` generates structural descriptions for structs, enums, and traits; `#[ReflectImpl]` generates implementation and method descriptions for inherent and trait implementations.
- `TypeDescriptor` and `FieldDescriptor` provide checked field reads, mutable borrows, and replacements. Construction APIs create or update objects from ordinary Rust values.
- `ReflectRegistry` collects linked types, implementations, and methods so applications can discover members, invoke supported methods, and query typed capabilities such as `Clone` and `Default` at runtime.
- Built-in descriptors cover primitives, text, tuples, arrays, `Option`, sequences, sets, maps, smart pointers, and function pointers. The `ecosystem-types` and `qubit-types` features add common ecosystem and Qubit types.

Keep these boundaries in mind:

- Dynamic operations require exact types; the library does not parse strings, coerce numbers, or infer `Into` conversions.
- Reflection does not replace authorization, business validation, database transactions, or synchronization. The application retains those policies.
- `#[reflect(skip)]` only removes a member from reflection metadata; it is not a secrecy mechanism. Safe APIs reject non-public members, and `_unsafe` APIs may bypass visibility only when their documented safety requirements hold.
- Field setting, construction, and invocation return structured errors with unconsumed inputs when rejected before execution. Once an adapter begins executing, the library does not promise to roll back application state.
- Runtime `TypeId` values cannot create new generic monomorphizations. Dynamically invoked generic methods must be bound at a compile-time call site with `bind_method!`; unsafe functions, unsupported ABIs, variadics, and opaque `impl Trait` returns can be described but not dynamically invoked.
- `TypeId`, descriptor addresses, and trait markers are meaningful only within the current process, not as serialization or cross-process model identifiers. Descriptors and per-type capability caches remain alive for the process lifetime.

See the [user guide](doc/user_guide.md) for construction, method invocation,
`Option` projection, error recovery, and registry details.

## Learn more

- [Architecture](doc/architecture.md) · [中文架构](doc/architecture.zh_CN.md)
- [User guide](doc/user_guide.md)
- [API reference](https://docs.rs/qubit-reflect)
- [中文 README](README.zh_CN.md) · [中文用户手册](doc/user_guide.zh_CN.md)

## Testing

Run these commands from the source checkout root:

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
