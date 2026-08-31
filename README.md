# qubit-reflect

[![Rust CI](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-reflect/coverage-badge.json)](https://qubit-ltd.github.io/rs-reflect/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-reflect.svg?color=blue)](https://crates.io/crates/qubit-reflect)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

`qubit-reflect` provides macro-generated structural and executable reflection on stable Rust. It supplies immutable descriptors plus checked field access, method invocation, value construction, capability lookup, and a deterministic process-local registry.

## Intended Users

The crate is intended for framework and library authors who need checked runtime access to Rust structure without parsing source files, reading private layout, or depending on compiler-private APIs.

## Installation

```toml
[dependencies]
qubit-reflect = "0.1"
```

## Current Status

The documented reflection runtime is implemented. The public entry points are:

- `#[derive(Reflect)]` for structs and enums;
- `#[reflect]` for trait declarations;
- `#[reflect_impl]` for inherent and trait implementations;
- `TypeDescriptor` and related immutable descriptor views;
- checked dynamic values, field access, invocation, construction, capabilities, and registry discovery;
- local and explicitly requested thread-safe operation modes.

The crate is `#![forbid(unsafe_code)]`. Runtime validation is exact: it does not coerce numeric values, parse strings, infer `Into`, or manufacture `Send`/`Sync` after type erasure.

## Quick Start

```rust
use qubit_reflect::{Reflect, ReflectedMut, ReflectedOwned, ReflectedRef, TypeDescriptor};

#[derive(Reflect)]
struct User {
    id: u64,
    name: String,
}

let descriptor = TypeDescriptor::of::<User>();
let name = descriptor.field("name").expect("reflected field");
let mut user = User { id: 7, name: String::from("Ada") };

assert_eq!(
    name.get(ReflectedRef::new(&user))
        .expect("checked read")
        .downcast_ref::<String>()
        .map(String::as_str),
    Some("Ada"),
);

name.set(
    ReflectedMut::new(&mut user),
    ReflectedOwned::new(String::from("Grace")),
).expect("checked replacement");
assert_eq!(user.name, "Grace");
```

Reflection is opt-in. `rename` changes query names without changing Rust identity; `skip`, `read_only`, `no_construct`, and `no_invoke` retain structural facts while disabling the selected operation.

## Learn More

- [Simplified Chinese requirements](doc/2026-08-28-qubit-reflect-requirements.zh_CN.md)
- [English user guide](doc/2026-08-29-qubit-reflect-user-guide.md)
- [Chinese user guide](doc/2026-08-29-qubit-reflect-user-guide.zh_CN.md)
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
