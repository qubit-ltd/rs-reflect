# Qubit Reflect (`rs-reflect`)

[![Rust CI](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/qubit-reflect.svg?color=blue)](https://crates.io/crates/qubit-reflect)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

`qubit-reflect` provides opt-in reflection for Rust declarations. Add `#[Reflect]` to a struct, enum, or trait and `#[ReflectImpl]` to an implementation. The macros generate metadata and checked adapters so application code can inspect fields and variants, construct values, and invoke supported methods by name.

## When reflection helps

A generic editor, serializer, or support tool can work with several reflected Rust types without maintaining a separate field-name dispatch table. The application still owns input parsing, authorization, business validation, and persistence. Descriptors retain their `ReflectionContext`, so method lookup and invocation use one consistent view.

## Installation

```toml
[dependencies]
qubit-reflect = "0.2"
```

The `derive` feature is enabled by default. For runtime-only consumers or third-party type integrations, see [dependency features](doc/user_guide.md#dependency-features).

## Quick start

This example updates a customer's email and invokes a reflected method. It uses the same runnable source as [`field_patch`](examples/field_patch.rs).

Run the complete example with `cargo run --example field_patch`.

```rust
use qubit_reflect::{Invocation, Reflect, ReflectImpl, TypeDescriptor};

#[Reflect]
struct Customer {
    pub email: String,
    pub credit_limit_cents: u64,
}

#[ReflectImpl]
impl Customer {
    pub fn raise_credit_limit(&mut self, amount: u64) -> bool {
        if amount <= self.credit_limit_cents {
            return false;
        }
        self.credit_limit_cents = amount;
        true
    }
}

fn update(customer: &mut Customer) -> Result<(), Box<dyn std::error::Error>> {
    let ty = TypeDescriptor::of::<Customer>();
    let email = ty.field("email")?;
    if let Err(failure) = email.set(customer, String::from("ada@corp.example")) {
        let (error, _rejected_value) = failure.into_parts();
        return Err(format!("email update failed: {error:?}").into());
    }
    let output = match ty
        .method("raise_credit_limit")?
        .invoke(Invocation::on_mut(customer).arg(75_000_u64))
    {
        Ok(output) => output,
        Err(failure) => {
            let (error, _recovery) = failure.into_parts();
            return Err(format!("method invocation failed: {error:?}").into());
        }
    };
    let raised = output.into_owned::<bool>()?;
    assert!(raised);
    Ok(())
}
```

`field()` and `method()` require one matching member and return structured errors when lookup fails. Dynamic operations check the exact Rust types. Generic methods need a concrete compile-time binding through `bind_method!`; a runtime `TypeId` cannot create a new Rust monomorphization.

## Core boundaries

- `ReflectionContext::global()` provides the process-wide view. Explicit contexts are available for isolation and custom registrations.
- Descriptors carry their context, so callers do not pass a registry again to accessors or invocation.
- `#[reflect(skip)]` removes a declaration from reflection metadata; it does not protect data from ordinary Rust code.
- Reflection does not perform authorization, parse external input, persist objects, or roll back application side effects.
- `Local` is the default dynamic value mode. `ThreadSafe` is available where the represented values satisfy the required Rust bounds.

## Learn more

- [Core API contract](doc/core_api_contract.md) · [中文核心 API 规范](doc/2026-10-20-core-api-redesign.zh_CN.md)
- [English user guide](doc/user_guide.md) · [中文用户手册](doc/user_guide.zh_CN.md)
- [Architecture](doc/architecture.md) · [中文架构](doc/architecture.zh_CN.md)
- [API reference](https://docs.rs/qubit-reflect)
- [中文 README](README.zh_CN.md)

## Development checks

From the repository root, run `./.infra/bin/align-ci.sh` to align formatting and CI configuration, then `./.infra/bin/ci-check.sh` for the project checks.

## License

Apache License 2.0; see [LICENSE](LICENSE).
