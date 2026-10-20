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

## Reflect a trait implementation

Mark a local trait with `#[Reflect]`, then name its definition explicitly on each implementation that should use its reflected definition and default-method adapters:

```rust
use qubit_reflect::{Reflect, ReflectImpl};

#[Reflect]
trait HealthCheck {
    fn check(&self) -> bool;

    fn label(&self) -> &'static str {
        "ready"
    }
}

#[Reflect]
struct Worker;

use crate::HealthCheck as Check;

#[ReflectImpl(reflected_trait = crate::HealthCheck)]
impl Check for Worker {
    fn check(&self) -> bool {
        true
    }
}
```

`reflected_trait` points to the reflected trait definition. The trait path written after `impl` may use an import alias; Rust resolves that path as usual. A bare `#[ReflectImpl]` on any trait impl, including one for a local `#[Reflect]` trait, preserves the source path and records an unlinked application as `ExternalIncomplete`. It does not associate a reflected definition or provide default methods. Only `reflected_trait = ...` links a local reflected trait provider.

`reflected_trait` and `external_trait` are caller assertions about the trait identity of the impl. Stable procedural macros cannot resolve import aliases to prove path equivalence. Ensure the selected provider or handle belongs to the trait the impl actually implements; otherwise reflection metadata will be associated with the wrong trait.

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

### Assemble the support-console flow in an application

`field_patch` is an in-memory exercise: it constructs a `Customer`, changes it, checks the result, and exits. It does not load a record or write a database. In an application, keep those responsibilities in the request handler and its repository boundary:

1. Parse the request and authorize the operator before touching the object.
2. Load `Customer` through the application repository; run the field update and reflected action on that in-memory value.
3. Run application business validation after the action. A `true` action result and the expected field values are the success signals; a `false` result means the requested limit was not raised and should follow the application's rejection response.
4. Save only after validation succeeds, using the repository's transaction policy. If validation or persistence fails, discard the in-memory value or use the application's rollback/compensation path. Reflection itself does not persist or roll back.

The runnable example covers steps 2's reflection operations and their checks. Parsing, authorization, repository types, validation rules, and transaction behavior belong to the host application and are intentionally not represented as library APIs here.

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

Derive-generated invocation adapters for `async fn` currently support only `Local`. A stable procedural macro cannot infer a `ThreadSafe` adapter for an opaque future, so it does not automatically generate one even when a particular implementation's future would satisfy `Send`. To request a `ThreadSafe` adapter, explicitly bind the concrete method with `bind_method!(... as thread_safe async fn(...))`; the compiler checks that the generated future satisfies `Send`, without requiring you to name its opaque type.

## Bind a generic method

Rust creates a generic method's concrete code at compile time. Register each supported specialization with `bind_method!` when building an explicit context; runtime reflection does not synthesize a new monomorphization from a `TypeId`. The following is a local fragment, not a standalone file: it assumes `Converter` and its generic `convert` method are declared and reflected with `#[Reflect]` / `#[ReflectImpl]`, and that `bind_method!` and `ReflectionContext` are imported. The builder registers linked declarations plus this selected specialization.

```rust
let context = ReflectionContext::builder()
    .include_linked()
    .bind(bind_method!(Converter::convert::<u8> as fn(&Converter, String) -> u8))
    .build()?;
```

Trait method bindings should carry an explicit identity hint. For a local reflected trait, `reflected_trait` names its definition; the binder calls the hidden application hook through the impl's trait path, so rustc resolves imports and fills omitted type and const defaults. This is another local fragment appended to the preceding builder chain; it assumes `Service`, the reflected trait definition `api::ServiceContract`, its implementation for `Service`, and the `ImportedService` import alias are already declared. It is not independently compilable as shown.

```rust
.bind(bind_method!(
    <Service as ImportedService>::refresh as fn(&Service) -> String,
    reflected_trait = api::ServiceContract,
))
```

For an external trait, create an identity handle with `reflect_external_trait!` and bind the explicit method with `external_trait = Handle`. The handle provides no external method declarations or default-argument information. Application identity contains only type and const arguments written in the impl path; the binding is an explicitly supplied method fact.

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
- [Core API contract (English summary)](core_api_contract.md) · [中文核心 API 设计规范](2026-10-20-core-api-redesign.zh_CN.md)
- [API reference](https://docs.rs/qubit-reflect) lists the public types and methods.
- [中文用户手册](user_guide.zh_CN.md) follows the same support-console workflow in Chinese.
