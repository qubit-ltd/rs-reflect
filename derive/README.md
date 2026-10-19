# `qubit-reflect-derive`

`qubit-reflect-derive` provides the `#[Reflect]`, `#[ReflectImpl]`, and `bind_method!` procedural macros. They generate metadata and adapters; runtime descriptors, contexts, and checked operations are provided by `qubit-reflect`.

## Installation

Most applications should depend on `qubit-reflect` with its default `derive` feature:

```toml
[dependencies]
qubit-reflect = "0.2"
```

If the runtime default features are disabled, add the matching derive crate explicitly:

```toml
[dependencies]
qubit-reflect = { version = "0.2", default-features = false }
qubit-reflect-derive = "0.2"
```

Keep the runtime and derive versions aligned. Generated code calls the versioned internal `__private::codegen_v4` protocol; it is an implementation boundary, not an application API.

## Quick start

```rust
use qubit_reflect::TypeDescriptor;
use qubit_reflect_derive::Reflect;

#[Reflect]
struct User {
    name: String,
}

fn read_name(user: &User) -> Result<&String, Box<dyn std::error::Error>> {
    let field = TypeDescriptor::of::<User>().field("name")?;
    Ok(field.get(user)?)
}
```

Use `#[ReflectImpl]` to describe inherent or trait implementations. `#[reflect(skip)]` excludes a field, variant, or method from reflection metadata. Other helper attributes are not part of the vNext macro surface.

Generic method declarations remain metadata until a concrete specialization is bound at compile time with `bind_method!`. The runtime cannot create Rust monomorphizations from a dynamic type ID. Unsupported method signatures remain inspectable with an availability reason.

## Examples and reference

The repository examples require the `derive` feature and can be run from the workspace root:

```bash
cargo run --example field_patch
cargo run --example customer_patch
cargo run --example support_action
```

- [English user guide](../doc/user_guide.md) · [中文用户手册](../doc/user_guide.zh_CN.md)
- [Runtime overview](../README.md) · [中文 README](../README.zh_CN.md)
- [Macro API docs](../src/lib.rs)
