# qubit-reflect

[![Rust CI](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-reflect/coverage-badge.json)](https://qubit-ltd.github.io/rs-reflect/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-reflect.svg?color=blue)](https://crates.io/crates/qubit-reflect)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

`qubit-reflect` lets Rust programs inspect type structure, access fields, and
invoke methods by name at runtime. It is for authors of configuration editors,
frameworks, and libraries that need to work with multiple types without keeping
a separate field table or method map. Reflection macros generate the required
code at the declaration site and work on stable Rust.

For example, an editor receiving the field name `"name"` can read a `User`'s
current name and replace it with a `String`. A type mismatch returns an error
and preserves inputs that have not been consumed. The application continues
to own the original `User` value.

If application code already knows the field, access `user.name` directly.
Reflection is useful when the target is chosen at runtime or a framework must
inspect different types uniformly. This crate does not convert form text into
Rust values, serialize objects, or supply business validation rules.

## Installation

```toml
[dependencies]
qubit-reflect = { version = "0.1", path = "../rs-reflect" }
```

Requires Rust 1.94 or later. Adjust `path` relative to your application's
`Cargo.toml`.

The crate is currently consumed only from Qubit's internal workspace or an
approved internal Git revision. It is not published to crates.io. Keep the
runtime and derive crate on the same repository revision.

The reflection macros are enabled by default; the example below needs no extra
features. For runtime-only use or third-party type implementations, see the
[dependency profiles](doc/2026-08-29-qubit-reflect-user-guide.md#choose-dependency-features) in the guide.

## Quick Start

A schema-driven editor needs to display and replace a field selected by name,
while ordinary application code continues to own the value. Derive the
descriptor at the declaration site, find the field, then pass the appropriate
borrow wrapper. The adapter validates the target, operation policy, and exact
Rust type before it changes anything.

```rust
use qubit_reflect::{Reflect, ReflectedMut, ReflectedOwned, ReflectedRef, TypeDescriptor};

#[derive(Reflect)]
struct User {
    id: u64,
    name: String,
}

fn main() {
    let descriptor = TypeDescriptor::of::<User>();
    let name = descriptor.field("name").expect("derived field");
    let mut user = User { id: 7, name: String::from("Ada") };

    let current = name.get(ReflectedRef::new(&user)).expect("checked read");
    assert_eq!(current.downcast_ref::<String>().map(String::as_str), Some("Ada"));

    name.set(
        ReflectedMut::new(&mut user),
        ReflectedOwned::new(String::from("Grace")),
    )
    .expect("exactly typed replacement");
    assert_eq!(user.name, "Grace");
}
```

Save the example as `src/main.rs` in a binary crate using the dependency above,
then run `cargo run`. It exits successfully after checking that the name changes
from `Ada` to `Grace`.

## Why This Project Exists

Rust deliberately does not offer unrestricted runtime reflection. Frameworks
that need a type graph, a property editor, plug-in discovery, or dynamic
dispatch often end up parsing source, maintaining a duplicate schema, or
erasing values without preserving their ownership and thread-safety boundary.
`qubit-reflect` keeps the contract in Rust declarations: generated code supplies
only operations that Rust can prove safe, and descriptors retain structural
facts even where an operation is unavailable.

## What It Provides

- Separate immutable descriptors for concrete runtime types and generic source
  definitions, plus traits, implementations, and supported built-in families.
- Checked field reads, mutable borrows, replacements, enum-branch checks, and
  dynamic construction. Pre-execution validation failures preserve
  caller-owned inputs in recovery objects.
- Generated invocation adapters for supported methods, with local and
  explicitly requested thread-safe modes.
- A deterministic registry assembled from linked inventory or explicit facts. It is
  the only public resolver for effective concrete and definition capabilities,
  including typed `Clone` and `Default` adapters. Every registration path
  uses the same transactional validator; callers may hold an explicit,
  immutable registry snapshot instead of consulting the process-global result.
- Strict typed capability lookup preserves four states: `Missing`, `FactOnly`,
  `AdapterTypeMismatch`, and `Found`. When provenance matters, the registry also
  reports whether an effective capability is `Intrinsic` or `Registered` and can
  return the contributing `FragmentIdentity`.
- Explicit `Local` and opt-in `ThreadSafe` dynamic boundaries. Thread-safe
  field access and construction exist only for types whose generated code
  proves the required `Send + Sync` bounds.

Reflection is deliberately bounded. It does not coerce numeric values, parse
strings, infer `Into`, or upgrade a local dynamic value to thread-safe mode.
`TypeId`, descriptor addresses, and trait markers are process-local identity,
not serialization or cross-process model identifiers. Unsupported or disabled
operations remain visible as descriptors with structured unavailable reasons.
Tuple and portable function-pointer descriptors support arities 0 through 32;
arity 33 and above are unsupported and intentionally have no `Reflect` impl.

Use `RegistrySnapshotBuilder` when a library or test needs an explicit set of
registrations. The guide covers [isolated snapshots](doc/2026-08-29-qubit-reflect-user-guide.md#build-an-isolated-registry-snapshot),
[capability conflicts](doc/2026-08-29-qubit-reflect-user-guide.md#migrating-effective-capability-queries),
and [empty struct construction](doc/2026-08-29-qubit-reflect-user-guide.md#constructing-empty-structs).

## Learn More

- [English user guide](doc/2026-08-29-qubit-reflect-user-guide.md)
- [中文用户指南](doc/2026-08-29-qubit-reflect-user-guide.zh_CN.md)
- [API overview in Rustdoc source](src/lib.rs); generate and open the full reference
  from the repository root with `cargo doc --all-features --no-deps --open`
- [English design](doc/2026-09-03-qubit-reflect-design.md)
- [中文详细设计](doc/2026-09-03-qubit-reflect-design.zh_CN.md)
- [Evolution history](doc/2026-09-07-qubit-reflect-evolution.md) · [中文演进历史](doc/2026-09-07-qubit-reflect-evolution.zh_CN.md)
- [Simplified Chinese requirements](doc/2026-08-28-qubit-reflect-requirements.zh_CN.md)
- [English requirements](doc/2026-09-03-qubit-reflect-requirements.md)
- [English traceability matrix](doc/2026-09-03-qubit-reflect-requirements-traceability.md)
- [中文需求追踪矩阵](doc/2026-08-29-qubit-reflect-requirements-traceability.zh_CN.md)
- [简体中文 README](README.zh_CN.md)

## Testing

```bash
# Run tests with the default feature set
cargo test

# Run tests with all declared features
cargo test --all-features

# Project CI checks
./ci-check.sh

# Check code coverage
./coverage.sh
```

## License

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for the
full license text.

## Contributing

Contributions are welcome. Please follow the Rust API guidelines, keep public
API documentation and tests current, and run `./align-ci.sh` to format code and
`./ci-check.sh` to satisfy CI requirements before submitting a pull request.

## Author

**Haixing Hu** - *Qubit Co. Ltd.*

Repository: [https://github.com/qubit-ltd/rs-reflect](https://github.com/qubit-ltd/rs-reflect)
