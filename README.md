# qubit-reflect

[![Rust CI](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-reflect/coverage-badge.json)](https://qubit-ltd.github.io/rs-reflect/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-reflect.svg?color=blue)](https://crates.io/crates/qubit-reflect)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

`qubit-reflect` gives framework and library authors opt-in, macro-generated
reflection on stable Rust. It turns declared Rust types into immutable
descriptors and exposes checked dynamic field access, construction, method
invocation, capabilities, and process-local registry discovery—without source
parsing, private-layout inspection, compiler-private APIs, or `unsafe`.

## Installation

```toml
[dependencies]
qubit-reflect = "0.1"
```

The default `derive` feature exports `#[derive(Reflect)]`, `#[reflect]`, and
`#[reflect_impl]`. Disabling default features keeps the runtime and handwritten
registration APIs, but does not re-export those macros.

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
```

## Why This Project Exists

Rust deliberately does not offer unrestricted runtime reflection. Frameworks
that need a type graph, a property editor, plug-in discovery, or dynamic
dispatch often end up parsing source, maintaining a duplicate schema, or
erasing values without preserving their ownership and thread-safety boundary.
`qubit-reflect` keeps the contract in Rust declarations: generated code supplies
only operations that Rust can prove safe, and descriptors retain structural
facts even where an operation is unavailable.

## What It Provides

- Immutable descriptors for reflected structs, enums, traits, implementations,
  generic facts, and supported built-in type families.
- Checked field reads, mutable borrows, replacements, enum-branch checks, and
  dynamic construction. Failed owned-input operations return recovery objects.
- Generated invocation adapters for supported methods, with local and
  explicitly requested thread-safe modes.
- A deterministic registry assembled from linked inventory fragments, plus
  typed `Clone` and `Default` capability adapters.

Reflection is deliberately bounded. It does not coerce numeric values, parse
strings, infer `Into`, or upgrade a local dynamic value to thread-safe mode.
`TypeId`, descriptor addresses, and trait markers are process-local identity,
not serialization or cross-process model identifiers. Unsupported or disabled
operations remain visible as descriptors with structured unavailable reasons.

## Learn More

- [English user guide](doc/2026-08-29-qubit-reflect-user-guide.md)
- [中文用户指南](doc/2026-08-29-qubit-reflect-user-guide.zh_CN.md)
- [API documentation](https://docs.rs/qubit-reflect)
- [Simplified Chinese requirements](doc/2026-08-28-qubit-reflect-requirements.zh_CN.md)
- [Requirements traceability matrix](doc/2026-08-29-qubit-reflect-requirements-traceability.zh_CN.md)
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
