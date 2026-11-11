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

For a reflected struct or enum with non-`Send + Sync` fields, use
`#[Reflect(local_only)]` to retain local field access and construction. Bare
`#[Reflect]` keeps its existing behavior, this option does not make values
thread-safe, and it does not change `#[ReflectImpl]` registrations. Use
`#[reflect(skip)]` only when the member should be omitted from reflection.

Explicit contexts select their target types and binding targets. Select a
target explicitly if code previously relied on unrelated linked registrations
appearing in a context. Context construction also reports invalid linked
capabilities, so handle the returned `ContextError`. Rc and Arc descriptors
now include their pointee type argument; this metadata addition requires no
downstream production change.

For a generic method with a pinned receiver, select the exact specialization
with `method.specialization(&MethodArguments)` and invoke it through
`MethodSpecializationDescriptor::invoke_pinned`. A rejected call returns the
pinned receiver and arguments for recovery. When the selected binding supplies
a concrete parameter type for `T`, pinned invocation checks the runtime
argument against that type before calling the adapter. The selection must match
the binding, and arguments that fail validation remain available for recovery.

The macros generate source metadata and adapters at compile time. Reflection is opt-in: a member that is not generated into metadata cannot be found through a descriptor. `skip` is a metadata choice, not an access control feature.

## Integrate a downstream derive

A downstream derive crate may use the versioned `qubit_reflect::__private::codegen_v4` integration protocol when it coordinates its version with the runtime. The supported expression symbols are listed in the [architecture protocol section](architecture.md#generated-code-integration-protocol); the list reflects built-in derive output and audited downstream generators, including `rs-model-metadata` and `rs-model-metadata-mm-implementation`. Check compatibility with `PROTOCOL_VERSION` and `protocol::assert_protocol_version`. This is not an application API or a promise that every `__private` item is supported. Keep runtime and generator versions aligned. An incompatible protocol change uses a new namespace and requires a coordinated runtime, derive, and downstream upgrade; the current v4 namespace remains available.

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

Composite descriptors display their complete diagnostic name. Context short-name lookup keeps its existing final-segment alias for compatibility; the display string is not a lookup key. Rust diagnostic names are unstable, so use an application-defined identifier when a name must be persisted.

## Read, update, and invoke

`TypeDescriptor::of` binds a descriptor to the process-global `ReflectionContext`. The context initializes lazily when an operation needs aggregate data. A field accessor is strict: it returns the one named field or a structured error. Typed access keeps the owner's and field's Rust types visible to the compiler.

```rust
use qubit_reflect::{Invocation, TypeDescriptor};
use qubit_reflect::value::Local;

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
        .invoke(Invocation::<Local>::on_mut(customer).arg(75_000_u64))
    {
        Ok(output) => output,
        Err(failure) => {
            let (error, _recovery) = failure.into_parts();
            return Err(format!("method invocation failed: {error:?}").into());
        }
    };
    let raised = output
        .into_owned::<bool>()
        .map_err(|_| std::io::Error::other("method returned a non-boolean output"))?;
    assert!(raised);
    Ok(())
}
```

The success signal is the returned `true` and the updated fields on the in-memory object. Reflection does not save the object. A service handler should load the customer through its repository, apply reflection operations, run application validation, then persist inside the application's transaction. If an update is rejected, retain or report the returned input and leave persistence to the handler's policy. Only commit the change after validation succeeds; if the operation fails after user code has run, use the repository transaction or an explicit compensation policy.

The runnable example deliberately stops before storage. An application can assemble the flow around it as: load by customer ID; invoke `update`; validate the resulting customer; save within a transaction on success; otherwise return the error without saving. Repository traits, database transactions, and authorization rules are application-specific.

## Handle missing and ambiguous results

Use a strict singular accessor when the application requires exactly one member. Use a collection's optional `get` for a field that may be absent, or `lookup_method` when same-name methods may have more than one candidate:

```rust
fn lookup_demo(ty: qubit_reflect::TypeDescriptor<'_>) {
    let email = ty.fields().get("email");
    assert!(email.is_some());
    match ty.lookup_method("reset").expect("lookup action") {
        qubit_reflect::lookup::Lookup::Missing => {}
        qubit_reflect::lookup::Lookup::Unique(_) => panic!("unknown action unexpectedly exists"),
        qubit_reflect::lookup::Lookup::Ambiguous(_) => panic!("unknown action is ambiguous"),
    }
    match ty.lookup_method("raise_credit_limit").expect("lookup method") {
        qubit_reflect::lookup::Lookup::Missing => panic!("method is registered"),
        qubit_reflect::lookup::Lookup::Unique(method) => {
            let mut customer = Customer { email: String::from("ada@corp.example"), credit_limit_cents: 10_000, internal_note: String::new() };
            let result = method.invoke(qubit_reflect::Invocation::<qubit_reflect::value::Local>::on_mut(&mut customer).arg(25_000_u64))
                .unwrap_or_else(|_| panic!("invoke method"));
            assert!(result.into_owned::<bool>().unwrap_or_else(|_| panic!("boolean result")));
            assert_eq!(customer.credit_limit_cents, 25_000);
        }
        qubit_reflect::lookup::Lookup::Ambiguous(_) => panic!("method has one candidate"),
    }
}
```

`method()` reports missing or ambiguous names as `LookupError`; `lookup_method()` returns the complete `Lookup` state and keeps context initialization errors separate. For type discovery, `ReflectionContext::type_by_name` is strict and `lookup_type_by_name` returns the complete state. Do not treat a diagnostic Rust type name or a `TypeId` as a persistent identifier.

## Construct a customer

Use a separate type with public reflected fields when callers are allowed to construct it. Checked construction requires every field and rejects values of the wrong Rust type; `into_input()` returns the rejected named input for correction or reporting. This example avoids skipped and private fields, and does not use unchecked construction.

```rust
#[derive(Debug, PartialEq)]
#[qubit_reflect::Reflect]
struct NewCustomer {
    pub email: String,
    pub credit_limit_cents: u64,
}

fn construct_demo() {
    use qubit_reflect::construct::{NamedFields, RecoveredConstructionInput};
    use qubit_reflect::value::Local;

    let ty = qubit_reflect::TypeDescriptor::of::<NewCustomer>();
    let customer = ty.construct::<NewCustomer, _>(
        NamedFields::<Local>::new()
            .field("email", String::from("ada@corp.example"))
            .field("credit_limit_cents", 10_000_u64),
    ).expect("construct customer");
    assert_eq!(customer.credit_limit_cents, 10_000);

    let missing = ty.construct::<NewCustomer, _>(
        NamedFields::<Local>::new().field("email", String::from("ada@corp.example")),
    ).expect_err("missing required field is rejected");
    assert!(matches!(missing.into_input(), RecoveredConstructionInput::Named(_)));

    let wrong_type = ty.construct::<NewCustomer, _>(
        NamedFields::<Local>::new()
            .field("email", String::from("ada@corp.example"))
            .field("credit_limit_cents", String::from("too many")),
    ).expect_err("wrong field type is rejected");
    assert!(matches!(wrong_type.into_input(), RecoveredConstructionInput::Named(_)));
}
```

The returned value is the success signal. Construction does not validate application rules such as authorization or credit policy; validate the value before saving it.

## Update a customer

`UpdateInput` groups several replacements with an owned base. A rejected update retains both; inspect the base before consuming the failure, then use `into_input()` to take ownership of the complete recovery input.

```rust
fn update_owned_demo() {
    use qubit_reflect::construct::UpdateInput;
    use qubit_reflect::value::Local;

    let ty = qubit_reflect::TypeDescriptor::of::<NewCustomer>();
    let customer = NewCustomer { email: String::from("ada@corp.example"), credit_limit_cents: 10_000 };
    let updated = ty.update::<NewCustomer, _>(
        UpdateInput::<Local>::new(customer)
            .field("email", String::from("new@corp.example"))
            .field("credit_limit_cents", 75_000_u64),
    ).expect("update customer");
    assert_eq!(updated.email, "new@corp.example");
    assert_eq!(updated.credit_limit_cents, 75_000);

    let customer = NewCustomer { email: String::from("before@corp.example"), credit_limit_cents: 20_000 };
    let failure = ty.update::<NewCustomer, _>(
        UpdateInput::<Local>::new(customer)
            .field("email", String::from("after@corp.example"))
            .field("credit_limit_cents", String::from("invalid")),
    ).expect_err("wrong field type is rejected");
    assert_eq!(failure.base().downcast_ref::<NewCustomer>().expect("recover base").credit_limit_cents, 20_000);
    let recovered = failure.into_input();
    let repeated_failure = ty.update::<NewCustomer, _>(recovered)
        .expect_err("recovered replacement remains available for correction");
    assert_eq!(repeated_failure.base().downcast_ref::<NewCustomer>().expect("recover base").credit_limit_cents, 20_000);
    drop(repeated_failure.into_input());
}
```

This proves checked input validation and ownership recovery before user code runs. It is not a transaction: it cannot undo arbitrary user side effects or a destructor panic. The application remains responsible for validation, persistence, and transaction or compensation policy.

## Select an explicit context

`include_type` selects a concrete type registration. `include_linked` gathers linked registrations. `include_linked_for` selects the linked closure for a target and also applies the library's global definitions and capability handling. It is a registration-selection tool, not a security boundary or a promise to exclude all unrelated metadata.

```rust
fn explicit_context_demo() {
    use qubit_reflect::{Invocation, ReflectionContext, TypeDescriptor};
    use qubit_reflect::value::Local;

    let context = ReflectionContext::builder()
        .include_linked_for::<Customer>()
        .build()
        .expect("build selected context");
    let ty = TypeDescriptor::of_in::<Customer>(&context);
    assert!(ty.method("raise_credit_limit").is_ok());
    let mut customer = Customer { email: String::from("ada@corp.example"), credit_limit_cents: 10_000, internal_note: String::new() };
    let output = ty.method("raise_credit_limit").expect("registered method")
        .invoke(Invocation::<Local>::on_mut(&mut customer).arg(25_000_u64))
        .unwrap_or_else(|_| panic!("invoke registered method"));
    assert!(output.into_owned::<bool>().unwrap_or_else(|_| panic!("boolean result")));
    assert_eq!(customer.credit_limit_cents, 25_000);
}
```

Handle `build()`'s `ContextError` and use its diagnostics to identify missing or invalid registrations. Choosing an explicit context does not replace authorization checks.

Linked registration factories run during `include_linked()` or
`include_linked_for()`, and may run again when included by another builder.
Keep them limited to producing registration data: do not query or initialize
`ReflectionContext::global()` recursively, or build a linked context from
inside a factory to inspect registrations. Same-thread recursive global
initialization panics, and a factory panic propagates from the include call;
`build()` does not catch it.

## Recover from rejected updates and calls

A typed field replacement returns `FieldSetFailure<V>` on failure. Its `into_parts()` method returns both the diagnostic and the replacement value, so a handler can preserve or report the rejected input. Before a method begins, invocation validation can fail because the receiver or an argument has the wrong type or mode, the parameter binding is invalid, or the method is unavailable. `InvocationFailure` retains the recovery state for inputs rejected before execution.

Once user code starts, reflection cannot undo its side effects. If the method partially updates the object and then returns an application error, the caller must apply its normal transaction or compensation policy. `invoke()` also does not catch a panic; choose [`MethodDescriptor::invoke_catching`](https://docs.rs/qubit-reflect/latest/qubit_reflect/struct.MethodDescriptor.html#method.invoke_catching) only when the application has a defined panic policy. This entry point catches panics only for synchronous calls when the selected value mode has a catching adapter and the build uses panic unwinding. Async methods and `panic=abort` builds are rejected before the method runs with `ValueModeUnavailable` when earlier checks pass; other preflight checks can report their own errors. A `CaughtInvocationFailure::Rejected` retains the invocation, which can be recovered through its `InvocationFailure`. A panic raised while polling a future is outside this synchronous boundary, and a caught panic does not roll back side effects or guarantee recovery of values consumed by the method.

## Use dynamic values at framework boundaries

Use typed field access when generic application code still knows its Rust types. Use `DynamicOwned<Local>`, `DynamicRef<Local>`, or `DynamicMut<Local>` when a framework receives a type-erased object or replacement. Dynamic operations still check the exact target and value types before access. A failed downcast returns the original value, and failed field replacement retains its replacement input.

`ThreadSafe` mode is available for values and adapters that satisfy the relevant `Send` and `Sync` bounds. Local values cannot be promoted to `ThreadSafe` by erasure. Choose the mode at the boundary where ownership and concurrency requirements are known.

Derive-generated invocation adapters for `async fn` currently support only `Local`. A stable procedural macro cannot infer a `ThreadSafe` adapter for an opaque future, so it does not automatically generate one even when a particular implementation's future would satisfy `Send`. To request a `ThreadSafe` adapter, explicitly bind the concrete method with `bind_method!(... as thread_safe async fn(...))`; the compiler checks that the generated future satisfies `Send`, without requiring you to name its opaque type. The binding signature describes the value produced after awaiting the method: its output must match exactly, and an omitted output means `()`. Invocation returns a reflected future without running the method; call `into_future()` and poll it to completion, including for unit-returning methods.

For source-level tuple types, `TypeExpression::tuple_elements()` borrows the retained elements without allocating a `Vec`. It returns `Some` for every tuple, including `()`, and `None` for other types:

```rust,ignore
if let Some(elements) = field.type_expression().tuple_elements() {
    for element in elements.iter() {
        println!("{}", element.display());
    }
}
```

`TypeExpression::kind()` remains compatible, but its `Tuple` variant owns an allocated element collection. Use `tuple_elements()` when callers only need to inspect or iterate tuple elements.

## Bind a generic method

Rust creates a generic method's concrete code at compile time. Register each supported specialization with `bind_method!` when building an explicit context; runtime reflection does not synthesize a new monomorphization from a `TypeId`.

For a generic parameter passed to a pinned method, the selected binding also
provides the concrete parameter type used during pinned-call validation.
Validation still checks the receiver and every argument, and a rejected call
returns its original pinned receiver and arguments.

The example below shows the full path from the generic method definition to a
selected specialization and its result. `Converter::convert` parses its text
input as the chosen output type; this context binds the `u8` specialization.

```rust
use qubit_reflect::descriptor::MethodArguments;
use qubit_reflect::value::Local;
use qubit_reflect::descriptor::Reflect as ReflectTrait;
use qubit_reflect::{bind_method, Invocation, ReflectImpl, ReflectionContext};

#[qubit_reflect::Reflect]
struct Converter;

#[ReflectImpl]
impl Converter {
    pub fn convert<T>(&self, value: String) -> T
    where
        T: ReflectTrait + std::str::FromStr,
        <T as std::str::FromStr>::Err: std::fmt::Debug,
    {
        value.parse().expect("the example input parses as the selected type")
    }
}

fn convert_demo() {
    let context = ReflectionContext::builder()
        .include_linked()
        .bind(bind_method!(Converter::convert::<u8> as fn(&Converter, String) -> u8))
        .build()
        .expect("the explicit context contains the converter binding");
    let method = context
        .type_of::<Converter>()
        .method("convert")
        .expect("the generic method is reflected");
    let specialization = method
        .specialization(&MethodArguments::new().type_of::<u8>())
        .expect("the u8 specialization is bound");
    let converter = Converter;
    let output = match specialization
        .invoke(Invocation::<Local>::on_ref(&converter).arg(String::from("42")))
    {
        Ok(output) => output,
        Err(failure) => panic!("the converter method runs: {:?}", failure.error().kind()),
    };
    let value = match output.into_owned::<u8>() {
        Ok(value) => value,
        Err(_) => panic!("the result is u8"),
    };
    assert_eq!(value, 42);
}
```

Trait method bindings should carry an explicit identity hint. For a local reflected trait, `reflected_trait` names its definition; the binder calls the hidden application hook through the impl's trait path, so rustc resolves imports and fills omitted type and const defaults:

```text
.bind(bind_method!(
    <Service as ImportedService>::refresh as fn(&Service) -> String,
    reflected_trait = api::ServiceContract,
))
```

For an external trait, create an identity handle with `reflect_external_trait!` and bind the explicit method with `external_trait = Handle`. The handle provides no external method declarations or default-argument information. Application identity contains only type and const arguments written in the impl path; the binding is an explicitly supplied method fact.

The method descriptor keeps the source declaration. Select a concrete specialization with `MethodArguments` before invoking it. Unbound or unsupported methods remain visible as metadata with an availability reason; they are not silently called through an unsafe fallback.

When comparing retained type-expression metadata, use `TypeExpression::structurally_eq` instead of comparing `display()` output. Display text is for diagnostics; structural comparison checks source facts and does not establish Rust type identity.

`display()` includes retained bounds and higher-ranked lifetime binders where available. Its diagnostic format can change as more source facts are shown; do not persist or parse it as a stable identifier.

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
- [Core API contract](core_api_contract.md) summarizes context ownership, lookup, recovery, and capability guarantees.
- [API reference](https://docs.rs/qubit-reflect) lists the public types and methods.
- [中文用户手册](user_guide.zh_CN.md) follows the same support-console workflow in Chinese.
