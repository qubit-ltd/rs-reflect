# qubit-reflect

[![Rust CI](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml)
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
qubit-reflect = { version = "0.1", path = "../rs-reflect" }
```

The crate is currently consumed only from Qubit's internal workspace or an
approved internal Git revision. It is not published to crates.io. Keep the
runtime and derive crate on the same repository revision.

The default `derive` feature exports `#[derive(Reflect)]`, `#[reflect]`, and
`#[reflect_impl]`. Disabling default features keeps the runtime and handwritten
registration APIs, but does not re-export those macros.

External type implementations are opt-in:

| Feature | Adds `Reflect` implementations for |
| --- | --- |
| `derive` (default) | The three reflection macros; no external type dependency |
| `ecosystem-types` | `BigDecimal`, `DateTime<Utc>`, `NaiveDate`, `NaiveTime`, and `Uuid` |
| `qubit-types` | `qubit_id::Id` and `qubit_datatype::DataType` |

For a runtime-only dependency, use
`qubit-reflect = { version = "0.1", path = "../rs-reflect", default-features = false }`. Enable only
the external type families that cross your reflection boundary.

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
- A deterministic registry assembled from linked inventory fragments. It is
  the only public resolver for effective concrete and definition capabilities,
  including typed `Clone` and `Default` adapters. Every registration path
  enters this one validated fragment stream; callers may hold an explicit,
  immutable registry snapshot instead of consulting the process-global result.
- A versioned `__private::codegen_v2` protocol for derive/facade integration;
  downstream model code uses its own independent ABI v4.
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

## Learn More

- [English user guide](doc/2026-08-29-qubit-reflect-user-guide.md)
- [中文用户指南](doc/2026-08-29-qubit-reflect-user-guide.zh_CN.md)
- API documentation generated internally with `cargo doc --all-features`
- [English design](doc/2026-09-03-qubit-reflect-design.md)
- [中文详细设计](doc/2026-09-03-qubit-reflect-design.zh_CN.md)
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
