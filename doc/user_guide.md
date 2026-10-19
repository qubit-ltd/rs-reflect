# User guide

This guide is for Rust application and framework developers using the vNext API. It follows a support-console task: load a customer, apply a field update, invoke a supported action, then save the result. The runnable starting point is [`examples/field_patch.rs`](../examples/field_patch.rs); the README contains a smaller excerpt of the same path.

## Add reflection to a type

Add `qubit-reflect` with its default `derive` feature. Annotate declarations that should participate in reflection. `#[reflect(skip)]` omits a member from reflection metadata while leaving the Rust declaration intact.

```rust
use qubit_reflect::{Reflect, ReflectImpl};

#[Reflect]
struct Customer {
    pub email: String,
    pub credit_limit_cents: u64,
    #[reflect(skip)]
    internal_note: String,
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
```

The macros generate source metadata and adapters at compile time. Reflection is opt-in: a member that is not generated into metadata cannot be found through a descriptor. `skip` is a metadata choice, not an access control feature.

## Read, update, and invoke

`TypeDescriptor::of` binds a descriptor to the process-global `ReflectionContext`. The context initializes lazily when an operation needs aggregate data. A field accessor is strict: it returns the one named field or a structured error. Typed access keeps the owner's and field's Rust types visible to the compiler.

```rust
use qubit_reflect::{Invocation, TypeDescriptor};

fn update(customer: &mut Customer) -> Result<(), Box<dyn std::error::Error>> {
    let ty = TypeDescriptor::of::<Customer>();
    let email = ty.field("email")?;
    if let Err(failure) = email.set(customer, String::from("ada@corp.example")) {
        let (error, _rejected_email) = failure.into_parts();
        return Err(format!("email update failed: {error:?}").into());
    }
    assert_eq!(email.get::<_, String>(customer)?, "ada@corp.example");

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

The success signal is the returned `true` and the updated fields on the object. Reflection does not save the object. In a service, the handler should load through its repository, apply reflection operations, run application validation, and persist only according to its own transaction policy.

## Handle missing and ambiguous results

Use a strict singular accessor when the application requires exactly one member:

```rust
let method = ty.method("raise_credit_limit")?;
```

Use a collection's optional `get` for a field that may be absent, or `lookup_method` when same-name methods may have more than one candidate:

```rust
let email = ty.fields().get("email");
let matches = ty.lookup_method("reset")?;
match matches {
    qubit_reflect::Lookup::Missing => { /* report an unsupported action */ }
    qubit_reflect::Lookup::Unique(method) => { /* use the method */ }
    qubit_reflect::Lookup::Ambiguous(methods) => { /* ask for a qualified choice */ }
}
```

`method()` reports missing or ambiguous names as `LookupError`; `lookup_method()` returns the complete `Lookup` state and keeps context initialization errors separate. For type discovery, `ReflectionContext::type_by_name` is strict and `lookup_type_by_name` returns the complete state. Do not treat a diagnostic Rust type name or a `TypeId` as a persistent identifier.

## Recover from rejected updates and calls

A typed field replacement returns `FieldSetFailure<V>` on failure. Its `into_parts()` method returns both the diagnostic and the replacement value, so a handler can preserve or report the rejected input. Before a method begins, invocation validation can fail because the receiver or an argument has the wrong type or mode, the parameter binding is invalid, or the method is unavailable. `InvocationFailure` retains the recovery state for inputs rejected before execution.

Once user code starts, reflection cannot undo its side effects. If the method partially updates the object and then returns an application error, the caller must apply its normal transaction or compensation policy. `invoke()` also does not catch a panic; choose the explicit catching entry point only when the application has a defined panic policy.

## Use dynamic values at framework boundaries

Use typed field access when generic application code still knows its Rust types. Use `DynamicOwned<Local>`, `DynamicRef<Local>`, or `DynamicMut<Local>` when a framework receives a type-erased object or replacement. Dynamic operations still check the exact target and value types before access. A failed downcast returns the original value, and failed field replacement retains its replacement input.

`ThreadSafe` mode is available for values and adapters that satisfy the relevant `Send` and `Sync` bounds. Local values cannot be promoted to `ThreadSafe` by erasure. Choose the mode at the boundary where ownership and concurrency requirements are known.

## Bind a generic method

Rust creates a generic method's concrete code at compile time. Register each supported specialization with `bind_method!` when building an explicit context; runtime reflection does not synthesize a new monomorphization from a `TypeId`.

```rust
let context = ReflectionContext::builder()
    .include_linked()
    .bind(bind_method!(Converter::convert::<u8> as fn(&Converter, String) -> u8))
    .build()?;
```

The method descriptor keeps the source declaration. Select a concrete specialization with `MethodArguments` before invoking it. Unbound or unsupported methods remain visible as metadata with an availability reason; they are not silently called through an unsafe fallback.

## Dependency features

The default feature set includes `derive`. A runtime-only crate can disable defaults. `ecosystem-types` and `qubit-types` add metadata for optional external types; enable only the integrations used by the application.

```toml
[dependencies]
qubit-reflect = { version = "0.2", default-features = false }
qubit-reflect-derive = "0.2"
```

When using the derive crate directly, keep it paired with the matching runtime version. Most applications should use the default `derive` feature instead.

## Further reading

- [Architecture](architecture.md) explains context construction and generated metadata boundaries.
- [API reference](https://docs.rs/qubit-reflect) lists the public types and methods.
- [中文用户手册](user_guide.zh_CN.md) follows the same support-console workflow in Chinese.
