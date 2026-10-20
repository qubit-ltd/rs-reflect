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

Use `#[ReflectImpl]` to describe inherent or trait implementations. For a reflected trait implementation, pass its definition path explicitly so the macro can link the exact trait even when the impl uses an import alias:

```rust
#[ReflectImpl(reflected_trait = service_api::Service)]
impl ServiceAlias for MyService {}
```

A bare `#[ReflectImpl]` on any trait impl, including one for a local `#[Reflect]` trait, retains its source path and records an unlinked application as `ExternalIncomplete`. It does not associate the reflected trait definition or provide its default methods. Use `reflected_trait = ...` to link a local reflected trait definition; the macro does not guess whether a path names one. `#[reflect(skip)]` excludes a field, variant, or method from reflection metadata. Other helper attributes are not part of the vNext macro surface.

For an external trait, connect an explicit identity handle created with `reflect_external_trait!` by using `external_trait = Handle` on the impl:

Both parameters are caller assertions about trait identity. Stable procedural macros cannot resolve aliases and prove equivalence with the impl path, so ensure the handle or provider belongs to the trait actually implemented.

```rust
reflect_external_trait! { pub DisplayHandle = std::fmt::Display; }

#[ReflectImpl(external_trait = DisplayHandle)]
impl std::fmt::Display for LocalDisplay {
    // ...
}
```

This preserves the impl's written path for display while linking the application to the typed external identity. Without the handle, the application remains `ExternalIncomplete` even if its path text matches another registration.

Generic method declarations remain metadata until a concrete specialization is bound at compile time with `bind_method!`. The runtime cannot create Rust monomorphizations from a dynamic type ID. Unsupported method signatures remain inspectable with an availability reason.

For trait method bindings, append `, reflected_trait = DefinitionPath` after the function-pointer signature to select a local reflected definition and read compiler-resolved default generic arguments from the application hook. Use `, external_trait = Handle` for an external trait; its handle supplies identity only, while the binding records explicit method facts and only type/const arguments written in the impl path. `register_method_binding!` accepts the same hints.

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
