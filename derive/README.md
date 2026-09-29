# qubit-reflect-derive

[![Rust CI](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-reflect/coverage-badge.json)](https://qubit-ltd.github.io/rs-reflect/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-reflect-derive.svg?color=blue)](https://crates.io/crates/qubit-reflect-derive)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

<!-- reflect-contract: facade.explicit=qubit_reflect -->
<!-- reflect-contract: provider.qualified=custom-provider -->

`qubit-reflect-derive` provides the `Reflect`, `reflect`, and `reflect_impl` procedural macros used to generate reflection metadata and checked operation adapters. Most applications should depend on `qubit-reflect` with its default `derive` feature, which re-exports these macros together with the matching runtime API. This crate is useful when a facade or macro integration needs to depend on the procedural macros directly.

## Installation

Direct use requires the matching runtime crate because generated code calls its versioned `__private::codegen_v3` protocol. Create the example application in a directory beside the `rs-reflect` checkout and use both crates from that same checkout:

```toml
[dependencies]
qubit-reflect = { version = "0.1", path = "../rs-reflect", default-features = false }
qubit-reflect-derive = { version = "0.1", path = "../rs-reflect/derive" }
```

This source-checkout recipe does not verify registry publication of the current
checkout. Keep the local checkout layout intact because `qubit-reflect` also uses
the sibling `rust-common/rs-id` and `rust-common/rs-datatype` crates. Both crates require Rust 1.94 or later. For normal application code, depend on `qubit-reflect` with its default `derive` feature instead of adding the derive crate directly.

For registry dependencies, select published versions from the same release line:

```toml
[dependencies]
qubit-reflect = { version = "0.1", default-features = false }
qubit-reflect-derive = "0.1"
```

## Quick Start

An editor can read a field selected at runtime while the application keeps ownership of the original value. The derive macro generates the type metadata; the runtime descriptor performs the checked lookup.

```rust
use qubit_reflect::TypeDescriptor;
use qubit_reflect_derive::Reflect;

#[derive(Reflect)]
#[reflect(crate = qubit_reflect)]
struct User {
    name: String,
}

fn main() {
    let field = TypeDescriptor::of::<User>()
        .field("name")
        .expect("the derived field exists");
    assert_eq!(field.query_name(), Some("name"));
}
```

The derive supports structs and enums. The `reflect` attribute describes traits, and `reflect_impl` describes inherent or trait implementations. Unsupported operations remain unavailable with structured metadata instead of bypassing Rust's type and ownership checks. The macros do not provide reflection storage or runtime operations; those belong to `qubit-reflect`.

In same-package runtime examples, automatic discovery may select `crate`, the
example executable. Use `#[reflect(crate = qubit_reflect)]` for derives and
`#[reflect_impl(crate = qubit_reflect)]` for implementations to choose the runtime
facade explicitly. Native runtime examples declare `required-features = ["derive"]`
and are shipped in the runtime package; from the checkout root run
`cargo run --example field_patch`, `cargo run --example customer_patch`, or
`cargo run --example support_action`.

In `#[reflect(capabilities(...))]`, bare `Clone`, `Default`, `Send`, and `Sync`
select built-in capabilities. Qualified paths always select custom providers:
`my_crate::Clone` calls `my_crate::Clone::<Self>()`, even though
its final segment has the same spelling as a built-in.

## Learn More

- [English user guide](https://github.com/qubit-ltd/rs-reflect/blob/main/doc/user_guide.md)
- [中文用户指南](https://github.com/qubit-ltd/rs-reflect/blob/main/doc/user_guide.zh_CN.md)
- [Runtime crate overview](https://github.com/qubit-ltd/rs-reflect/blob/main/README.md) · [简体中文 README](https://github.com/qubit-ltd/rs-reflect/blob/main/README.zh_CN.md)
- [Procedural macro API documentation](https://github.com/qubit-ltd/rs-reflect/blob/main/derive/src/lib.rs)

## Testing

Run these commands from the `rs-reflect` repository root:

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
API documentation and tests current, and from the `rs-reflect` repository root
run `./align-ci.sh` to format code and `./ci-check.sh` to satisfy CI requirements
before submitting a pull request.

## Author

**Haixing Hu** - *Qubit Co. Ltd.*

Repository: [https://github.com/qubit-ltd/rs-reflect](https://github.com/qubit-ltd/rs-reflect)
