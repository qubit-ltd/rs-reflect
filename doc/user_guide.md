# Qubit Reflect user guide

<!-- reflect-contract: runtime.direct=qubit_reflect -->
<!-- reflect-contract: provider.qualified=custom-provider -->
<!-- reflect-contract: panic.async_poll=not-caught -->
<!-- reflect-contract: panic.abort=catching-unavailable -->
<!-- reflect-contract: dispatch.input_recovery=original-input -->
<!-- reflect-contract: examples.native=cargo-example -->
<!-- reflect-contract: typeref.constructor=owned-resolved -->
<!-- reflect-contract: lazy.retry=panic-retry -->
<!-- reflect-contract: field.lookup=linear -->

[Chinese user guide](user_guide.zh_CN.md) · [README](../README.md) · [API reference](https://docs.rs/qubit-reflect)

This guide covers `qubit-reflect` 0.2.0 on Rust 1.94 or later. It is for Rust application and framework developers who want to add reflection to their own types with macros and use the generated descriptors at runtime. Start with the macro and runtime model below, then run the field example. The support-console integration shows one practical use of that mechanism. Later sections cover construction, method invocation, registries, capabilities, and access boundaries; existing integrations can jump to [Migration notes](#migration-notes).

## Contents

- [The problem it solves](#the-problem-it-solves)
- [How macro-based reflection works](#how-macro-based-reflection-works)
- [Where to start](#where-to-start)
- [Use generated field reflection](#use-generated-field-reflection)
- [Integrate a support console](#integrate-a-support-console)
- [Check the PATCH result](#check-the-patch-result)
  - [What success looks like](#what-success-looks-like)
- [Construct a new value from input](#construct-a-new-value-from-input)
- [Invoke methods](#invoke-methods)
- [Discover types and extension capabilities](#discover-types-and-extension-capabilities)
  - [Capabilities and registry discovery](#capabilities-and-registry-discovery)
  - [Build an isolated registry snapshot](#build-an-isolated-registry-snapshot)
- [Choose features and access boundaries](#choose-features-and-access-boundaries)
  - [Choose dependency features](#choose-dependency-features)
  - [Choose transparent or opaque boundaries](#choose-transparent-or-opaque-boundaries)
- [Errors, diagnostics, and troubleshooting](#errors-diagnostics-and-troubleshooting)
- [Boundaries and a practice checklist](#boundaries-and-a-practice-checklist)
- [Migration notes](#migration-notes)
- [Further reading](#further-reading)

## The problem it solves

Rust does not provide general runtime inspection of an application's struct fields, enum variants, traits, or implementations. If a framework needs to discover members by name or perform an operation selected at runtime, handwritten metadata and dispatch tables repeat facts already present in the Rust declaration. Those copies can drift.

`qubit-reflect` generates the description and supported operation adapters from annotated Rust declarations. Consumers use immutable descriptors and checked dynamic values to inspect or operate on a reflected type. Reflection is opt-in: annotate custom types and traits with `#[Reflect]`, and implementations with `#[ReflectImpl]`; manual implementations of the runtime `Reflect` trait are also supported. The registry collects linked type and implementation facts, but does not store application objects or load plugins at runtime.

## How macro-based reflection works

| Declaration | Runtime facts |
| --- | --- |
| `#[Reflect]` on a struct, enum, or trait | Structural type, field, variant, and trait metadata plus supported adapters. |
| `#[ReflectImpl]` on an inherent or trait `impl` | Implementation and method metadata plus supported invocation adapters. |
| `#[reflect(skip)]` on a field, variant, or method | Completely omits that member from the reflection contract. |

The macros run at compile time. `TypeDescriptor::of::<T>()` gives the descriptor for a concrete type. Field operations accept ordinary references and values; invocation uses an `Invocation` builder and a selected `ReflectRegistry`. The runtime checks exact Rust types and preserves inputs rejected before an operation begins.

`skip` does not remove the Rust member and is not a secrecy mechanism. Non-public members can be described, but safe operations return a visibility error. The documented `_unsafe` operations bypass visibility only; callers must still satisfy the member's type and other invariants. Structs and enum variants with skipped fields remain describable but do not receive dynamic construction adapters, because the macro cannot invent a valid value for a hidden field without imposing extra bounds.

Generic method declarations remain definitions. To call one dynamically, bind a concrete instance at a compile-time call site with `bind_method!`, supplying its explicit function signature. Runtime `TypeId` values cannot generate new monomorphized code; an unbound generic method has no callable adapter. See the [method invocation section](#invoke-methods).

`Box<dyn Any>` inputs to dynamic construction and update APIs are checked using the box's enclosed concrete type. A field declared as `Box<dyn Any>` still has that exact field type. Applications provide their own locks and synchronization; there are no separate thread-safe dynamic wrapper modes.

A manual `Reflect` implementation must report the same Rust `TypeId` as its type. `TypeRef::of::<T>()` resolves to `TypeDescriptor::of::<T>()`; it is not a globally interned reference. Descriptor initialization can allocate. `TypeDescriptor::field` scans fields by name, so measure repeated lookups if they sit on a hot path.

## Where to start

1. Read [How macro-based reflection works](#how-macro-based-reflection-works), then run [Use generated field reflection](#use-generated-field-reflection). This is enough to see derive, descriptor lookup, and checked field access together.
2. Continue through the generic patch and [Check the PATCH result](#check-the-patch-result) when building a field-driven application workflow.
3. Read on as needed: [Construct a new value from input](#construct-a-new-value-from-input), [Invoke methods](#invoke-methods), [Discover types and extension capabilities](#discover-types-and-extension-capabilities), or [Choose features and access boundaries](#choose-features-and-access-boundaries).

Registry snapshots and migration notes come after the basic path.

## Use generated field reflection

The following runnable example focuses on the reflection boundary. `Customer` remains an ordinary Rust value owned by the caller; `#[Reflect]` supplies its descriptor and checked field adapters. The business workflow appears after the example.

<!-- reflect-source: examples/field_patch.rs -->
```rust
use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::access::FieldAccessError;

#[Reflect]
struct Customer {
    pub id: u64,
    pub email: String,
    pub credit_limit_cents: u64,
    secret: String,
    #[reflect(skip)]
    internal_note: String,
}

/// Runs the example and panics if a business assertion or reflection operation
/// fails.
fn main() {
    let descriptor = TypeDescriptor::of::<Customer>();
    let mut customer = Customer {
        id: 1001,
        email: String::from("ada@example.com"),
        credit_limit_cents: 50_000,
        secret: String::from("private"),
        internal_note: String::from("not reflected"),
    };

    let email = descriptor.field("email").expect("derived field");
    email
        .set(&mut customer, String::from("ada@corp.example"))
        .expect("exactly typed replacement");
    assert_eq!(customer.email, "ada@corp.example");

    let current: &String = email.get(&customer).expect("shared read");
    assert_eq!(current, "ada@corp.example");

    let secret = descriptor.field("secret").expect("private field is described");
    assert!(matches!(
        secret.get::<_, String>(&customer),
        Err(FieldAccessError::NotPublic { .. })
    ));
    // SAFETY: this example owns Customer and intentionally demonstrates the
    // visibility bypass.
    assert_eq!(
        unsafe { secret.get_unsafe::<_, String>(&customer) }.expect("exact field type"),
        "private"
    );
    // SAFETY: this example owns Customer and maintains the String field's type
    // invariant.
    unsafe { secret.set_unsafe(&mut customer, String::from("updated")) }.expect("exact replacement type");
    assert_eq!(customer.secret, "updated");
    assert!(descriptor.field("internal_note").is_none());
    assert_eq!(customer.internal_note, "not reflected");
}
```


Add the dependency:

```toml
[dependencies]
qubit-reflect = "0.2"
```

The macros resolve the runtime crate directly, including in same-package
examples. Optional `qubit-datatype`
and `qubit-id` dependencies resolve from crates.io, so a source checkout does
not require adjacent repositories. Verify dependency resolution from the
repository root:

```bash
cargo metadata --locked --format-version 1
```

A successful metadata command confirms the locked dependency graph resolves.
Now run the native examples:

```bash
cargo run --example field_patch
cargo run --example customer_patch
cargo run --example support_action
```

All three targets declare `required-features = ["derive"]`; a runtime-only build
skips them. Their source files are included in the packaged runtime crate, so the
same commands work in an extracted package when `derive` is enabled. The version
dependency above selects a published release; source-checkout execution does not
prove that this checkout has been published or installed from a registry.

## Integrate a support console

The runnable [`customer_patch` example](../examples/customer_patch.rs) shows a load–patch–save flow. An API layer parses request text into Rust values and authorizes the caller; the patch layer resolves field names and calls `field.set(&mut target, value)`. If the operation fails during validation, `FieldSetFailure<V>` retains the original value for inspection or retry. Earlier successful changes may exist in the in-memory object, so persist only after the full patch succeeds.

A private field remains describable, but its safe getter or setter returns a visibility error. Use an `_unsafe` operation only when the caller can uphold the documented `# Safety` contract. A skipped field does not appear in queries at all; it is not an access-control mechanism.

## Check the PATCH result

The minimal example updates the email, reads it back as `"ada@corp.example"`, and confirms the skipped `internal_note` is absent from the descriptor. The program prints no application output; passing assertions and a normal exit are the success signal for that exercise.

### What success looks like

Before generated code runs, `set` checks the target type, visibility, and the replacement's exact `TypeId`. Non-public fields and type mismatches fail during those pre-execution checks; `FieldSetFailure::into_parts()` can return the unused value. Recovery retains field identity but not a borrow of the target; you can borrow `customer` again after the failed call ends.

| Stage | What the application sees | What to do |
| --- | --- | --- |
| `field("email")` missing | `None`; the member may be skipped or the name may not match the Rust declaration. | Check the declaration and whether `#[reflect(skip)]` is present; skipped members are not queryable. |
| Pre-execution `set` rejection | `FieldSetFailure` recovery is `Some`; the replacement was not consumed. | Read the structured error, prompt for corrected input, recover the value if needed. |
| Error after the adapter took ownership | Recovery is `None`. | Inspect the error and business state; do not assume the original value is retryable. |

`get` needs a shared borrow; `get_mut` and `set` need an exclusive borrow. Reflection does not parse strings, coerce numbers, or infer `Into`. If persistence runs after field replacement, a failed `save` is the application's problem; field adapters do not manage database transactions.

## Construct a new value from input

Construction builders take ordinary owned values. A typed construction returns the concrete Rust value; dynamic construction returns `Box<dyn Any>`. Validation checks shape, member names or indices, duplicates, visibility, missing values, and exact types before it consumes the inputs.

```rust
use qubit_reflect::{NamedConstructionInput, Reflect, TypeDescriptor};

#[Reflect]
struct Customer {
    pub id: u64,
    pub email: String,
}

fn main() {
    let customer: Customer = TypeDescriptor::of::<Customer>()
        .construct_struct(
            NamedConstructionInput::new()
                .field("id", 1001_u64)
                .field("email", String::from("ada@example.com")),
        )
        .expect("all public fields have values");
    assert_eq!(customer.email, "ada@example.com");
}
```

`ConstructionRecovery` retains inputs in caller order when validation fails. Struct updates accept a base value and ordinary replacement values. Dynamic `Box<dyn Any>` inputs are checked by their enclosed type; use the explicitly named `field_any`/`value_any` builder methods when supplying such dynamic boxes. If any field is marked `#[reflect(skip)]`, reflection still describes the type but dynamic construction is unavailable for that struct or enum variant: the omitted Rust field cannot safely be initialized by generated code.

## Invoke methods

Annotate method implementations with `#[ReflectImpl]`. A registry provides the method view, and `Invocation` accepts an ordinary receiver borrow and ordinary owned or borrowed arguments. The result is a single `Result`; validation failures preserve the receiver and unconsumed inputs in `InvocationFailure` recovery.

<!-- reflect-source: examples/support_action.rs -->
```rust
use qubit_reflect::Invocation;
use qubit_reflect::InvocationOutput;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectRegistry;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::MethodLookup;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;

#[Reflect]
struct Customer {
    pub id: u64,
    pub suspended: bool,
    pub suspension_reason: Option<String>,
}

#[ReflectImpl]
impl Customer {
    /// Suspends this customer with `reason`, returning false if already
    /// suspended.
    pub fn suspend(&mut self, reason: String) -> bool {
        if self.suspended {
            return false;
        }
        self.suspended = true;
        self.suspension_reason = Some(reason);
        true
    }
}

/// Runs the example and panics if a business assertion or reflection operation
/// fails.
fn main() {
    let registry = ReflectRegistry::initialize().expect("valid reflection declarations");

    let MethodLookup::Unique(suspend) = TypeDescriptor::of::<Customer>().methods_named_in(registry, "suspend") else {
        panic!("exactly one method named suspend")
    };

    let mut customer = Customer {
        id: 1001,
        suspended: false,
        suspension_reason: None,
    };
    {
        let invocation = Invocation::receiver_mut(&mut customer).arg(String::from("chargeback dispute"));
        let output = suspend
            .invoke(registry, invocation)
            .expect("valid receiver and arguments");
        let InvocationOutput::Owned(changed) = output else {
            panic!("owned output")
        };
        assert!(DynamicOwned::<Local>::downcast::<bool>(changed).unwrap_or_else(|_| panic!("bool")));
    }
    assert!(customer.suspended);
    assert_eq!(customer.suspension_reason.as_deref(), Some("chargeback dispute"));
}
```

Use the same registry for method lookup and invocation. Safe calls reject non-public methods. An `unsafe` invocation entry bypasses visibility only and still checks receiver type, argument types, borrowing, Pin, and enum state where applicable; read its `# Safety` section before calling it. Owned, borrowed, pinned, and future outputs retain the lifetimes imposed by their Rust signatures.

Generic methods need an explicit compile-time instance binding before name-based dynamic invocation. `bind_method!(method_path as fn(signature))` records the concrete target and generic arguments together with a generated adapter. The function signature is checked by Rust at the binding call site. Generic definitions remain queryable without a binding, but do not have a callable adapter. No runtime `TypeId` lookup generates code.

The binding is an ordinary value; add it to the snapshot builder before `build()` so the registry can match it with the linked method declaration:

```rust,no_run
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::bind_method;
use qubit_reflect::identity::FragmentIdentity;
use qubit_reflect::registry::RegistrySnapshotBuilder;

#[Reflect]
struct GenericService<T>(T);

#[ReflectImpl]
impl<T> GenericService<T> {
    fn read(&self) -> &T {
        &self.0
    }
}

fn main() {
    let binding = bind_method!(GenericService::<u8>::read as fn(&GenericService<u8>) -> &u8);
    let mut builder = RegistrySnapshotBuilder::new();
    builder.include_linked_fragments();
    builder.add_method_binding(
        binding,
        FragmentIdentity::new("example", module_path!(), line!(), 1, "binding", 1),
    );
    let registry = builder.build().expect("binding matches the linked method");
    let _registry = registry;
}
```

Method lookup can return no match, one method, or an ambiguous set. Handle each case explicitly. An invocation failure means the adapter exists but rejected the provided input; an unbound generic method instead reports an unavailable entry. Ordinary method panics follow the method's Rust panic behavior. Async calls return a future for the caller's executor to poll.

## Discover types and extension capabilities

When you know the type, obtain its descriptor directly. Use a registry to
discover linked types, find methods, or resolve extension operations for a
type. A registry holds type and operation metadata, not application objects;
it does not load dynamic plugins.

Normally, `ReflectRegistry::initialize()` collects registrations already linked
into the program. Build an isolated snapshot only when a library or test needs
an explicit fact set. A capability is a type extension, such as a typed `Clone`
or `Default` operation. Resolving capabilities and registering type membership
are separate concerns.

### Optional values

An optional descriptor separates structural knowledge from runtime access.
`element_type()` describes `T`; `has_ref_projection()` tells whether this
descriptor also carries an adapter for inspecting a borrowed `Option<T>`.
Built-in `Option<T>` descriptors provide that adapter:

```rust
use qubit_reflect::{OptionalProjectionError, TypeDescriptor};

fn main() -> Result<(), OptionalProjectionError> {
let value = Some(12_u32);
let optional = TypeDescriptor::of::<Option<u32>>()
    .as_optional()
    .expect("Option descriptor");
let inner = optional.project_ref(&value)?;
assert_eq!(inner, Some(&12));

let absent: Option<u32> = None;
assert!(optional.project_ref(&absent)?.is_none());
assert!(matches!(
    optional.project_ref(&Some(12_u8)),
    Err(OptionalProjectionError::TypeMismatch(_)),
));
Ok(())
}
```

Descriptors built from structural facts can describe an optional element
without holding the concrete `Option<T>` adapter. Their `project_ref` call
returns `OptionalProjectionError::Unavailable`; callers should check
`has_ref_projection()` when adapter availability determines whether an
operation can be bound. Projection uses the local shared borrow boundary; a
Projection accepts an ordinary shared reference. The caller is responsible for
any synchronization needed to access the value from multiple threads.

### Capabilities and registry discovery

Call `ReflectRegistry::initialize()` after the relevant crates are linked. The
registry transactionally aggregates fragments: conflicts yield `RegistryError`
and do not publish a partial result. Once frozen, its type, name, trait, impl,
capability, and effective-method indexes do not change. Static built-ins are
available before a lookup; on-demand composite descriptors use a separate
interner and do not mutate the public frozen registry.

```rust
use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::registry::ReflectRegistry;

#[Reflect]
struct Service;

fn main() {
    let snapshot = ReflectRegistry::initialize().expect("all fragments validate");
    let descriptor = TypeDescriptor::of::<Service>();
    let _methods = descriptor.methods_in(&snapshot);
    assert!(snapshot.get(descriptor.type_id()).is_some());
    let clone = snapshot.capability_by_id(descriptor, "qubit.reflect.clone")
        .expect("valid capability declarations");
    assert!(clone.is_none());
}
```

`impls_global()`, `methods_global()`, and `methods_named_global(name)` initialize
and query the process-global registry. Their `_in` counterparts
(`impls_in(&snapshot)`, `methods_in(&snapshot)`, and
`methods_named_in(&snapshot, name)`) query the explicitly supplied snapshot.
Use the same registry for method lookup and invocation: initialize once and pass
that registry to both operations, or pass the same explicit snapshot to both.
The snapshot is immutable; a failed global initialization never exposes a
partially built registry.

`snapshot.definitions()` enumerates generic declarations even when no concrete
instance is registered. Query them by `TypeDefinitionId`, Rust path, or query
name, and use `definition_capability` or `definition_capability_by_id` for
definition-level extensions. `TypeDefinitionDescriptor` describes a generic declaration; concrete instances
retain resolved type arguments. Definition fields contain `TypeExpression`
values and provide no value-access adapters. `TypeRef` resolves a target only
when generated Rust code can prove the concrete type; it never guesses from
a string name.

`Clone` and `Default` are typed capabilities. Register them only where their
Rust bounds hold, then query with `clone_key()` or `default_key()`. Other
safely generated special receiver forms require an exact `ReceiverAdapter` in
the selected registry, supplied by global registration or an explicit builder.
A missing capability leaves the entry point present: invocation returns
`Ok(Err(ReceiverAdapterUnavailable))` with the original inputs. Statically
unsupported signatures have no entry point.

In `#[reflect(capabilities(...))]`, the bare names `Clone`, `Default`, `Send`,
and `Sync` select built-in capabilities. A qualified path calls the named
custom provider, including when its final segment matches one of those names:
`#[reflect(capabilities(my_crate::Clone))]` invokes
`my_crate::Clone::<Self>()`.

### Decide whether a capability can execute

| `capability_lookup` state | Meaning | Application action |
| --- | --- | --- |
| `Missing` | No matching fact. | Choose the application's fallback. |
| `FactOnly` | Metadata exists without an adapter. | Inspect metadata; do not call it. |
| `AdapterTypeMismatch` | The ID matches but the adapter Rust type differs. | Check the registered adapter and typed key. |
| `Found` | A type-matching adapter exists. | Invoke under that capability's contract. |

`capability` maps `Found` to `Ok(Some(adapter))` and `Missing` to `Ok(None)`;
`FactOnly` and `AdapterTypeMismatch` are errors. `capability_by_id` reads facts by
text ID; use `capability_origin` and `capability_source` to trace intrinsic facts
or registration fragments. Querying a capability does not make its target a
snapshot member.

### Build an isolated registry snapshot

`ReflectRegistry::initialize()` is the process-global inventory entry point.
Use `RegistrySnapshotBuilder` when a library, fixture, or test owns an explicit
set of fragments and must keep it independent from linked inventory and global
initialization state. The builder starts empty, does not execute providers while
facts are being added, and freezes one immutable `ReflectRegistry` only when
`build()` succeeds.

```rust
use qubit_reflect::capability::{CapabilityDescriptor, CapabilityKey};
use qubit_reflect::identity::{CapabilityId, FragmentIdentity};
use qubit_reflect::registry::RegistrySnapshotBuilder;
use qubit_reflect::TypeDescriptor;
use std::any::TypeId;

fn source(kind: &str, line: u32) -> FragmentIdentity {
    FragmentIdentity::new("example", "fixture", line, 1, kind, u64::from(line))
}

fn main() -> Result<(), qubit_reflect::RegistryError> {
    let target = TypeDescriptor::of::<u32>();
    let key = CapabilityKey::<u32>::new(
        CapabilityId::new("example.limit").expect("valid capability ID"),
    );
    let mut builder = RegistrySnapshotBuilder::new();
    builder.add_type_with_capabilities(
        target,
        vec![CapabilityDescriptor::with_adapter(key, 7_u32)],
        source("type", 10),
        source("capability", 11),
    );
    builder.add_type_capabilities(
        TypeDescriptor::of::<u64>(),
        vec![CapabilityDescriptor::with_adapter(key, 8_u32)],
        source("capability", 12),
    );
    let snapshot = builder.build()?;

    assert!(snapshot.get(target.type_id()).is_some());
    assert_eq!(
        snapshot
            .capability(target, key)
            .expect("valid capability declarations"),
        Some(&7),
    );
    assert_eq!(snapshot.types().len(), 1);
    assert_eq!(
        snapshot.capability_only_type_targets("example.limit")[0].0,
        TypeId::of::<u64>(),
    );
    assert!(target.methods_in(&snapshot).is_empty());
    Ok(())
}
```

The combined `add_type_with_capabilities` method registers a root and its
capabilities while keeping their source identities separate. An empty builder yields a
snapshot with no registered roots. Calling only `add_type_capabilities` creates
a capability-only snapshot: `types()` remains empty, while `capability()` and
`capability_by_id()` can still resolve the target. The other typed inputs are
`add_definition`, `add_trait`, `add_impl_definition`, `add_impl`, and
`add_definition_capabilities`.

Only descriptors added with `add_type` or `add_type_with_capabilities` are
snapshot type members. `ModelRegistry::from_reflect_registry` projects those
members, so a capability-only target does not become a model implicitly. The
example registers `u64` only for a capability and confirms it remains outside
`types()`. Use
`capability_only_type_targets(capability_id)` to list registered capability
targets absent from `types()`. The query matches the stable capability ID even
when the adapter type differs, and orders results by their source fragment.
Use `member_capabilities(type_id)` when a query must require snapshot
membership: it returns `None` for the capability-only `u64`, while
`member_capabilities(TypeId::of::<u32>())` returns `Some(...)` for the
registered `u32` above. A member without effective facts returns
`Some(empty)`. This lookup reads the frozen snapshot and never runs a
provider. The existing `capabilities(descriptor)` and `capability(...)`
queries continue to resolve capability-only targets.
For an optional metadata registration audit, query
`snapshot.capability_only_type_targets("qubit.model.metadata.v1")`.
The audit is optional for reflection itself, but model projection is stricter:
`ModelRegistry::from_reflect_registry` returns `UnregisteredModelTarget` if a
model-metadata capability points to a type that is not a member of the
snapshot. When generic-model metadata is enabled, the same rule applies to
definition targets. Inspect the matching `capability_only_*_targets` result
and its source fragment, then either add the intended model type or definition
to the snapshot or remove that metadata registration from this view. Capability
queryability does not imply model membership.

For generic declarations, `definition_capabilities(id)` reports capability
facts independently from definition membership. It returns `None` when the
snapshot has neither the definition nor capability facts for that ID,
`Some(empty)` for a definition member without capabilities, and
`Some(nonempty)` when capability facts exist. The last case also includes a
capability-only target: `definition_capabilities(id)` can return facts while
`definition(id)` is `None`. Use `definition(id).is_some()` to check snapshot
membership. The typed `definition_capability` and textual
`definition_capability_by_id` queries can also return a matching adapter or
descriptor from a capability-only target; they return `None` when the target
has no matching facts or capability. Use `definition(id)` separately when
membership matters. A fact without an executable adapter produces
`CapabilityAccessError::FactOnly`, and a typed key with a different adapter
type produces `AdapterTypeMismatch`.

Use `member_definition_capabilities(id)` to combine the membership check and
capability-set lookup. It returns `None` for a definition registered only as a
capability target, `Some(empty)` for a member without capabilities, and the
member's facts otherwise. Like the type-member query, it reads only frozen
snapshot data and never executes a provider.

Pass the resulting snapshot explicitly to `impls_in`, `methods_in`, or
`methods_named_in` when a property or method query must use that exact set of
facts. Global queries use the process-global registry instead. A method lookup
and its invocation must use the same registry snapshot. `build()` validates all identities, links, and capability conflicts as
one transaction. It returns a `RegistryError` without publishing partial
state; provide stable `FragmentIdentity` values so duplicate or changed source
identities are diagnosable. For a conflict, inspect
`conflicting_fragments()`, `capability_details()`, `capability_target()`, and
`capability_id()`. Intrinsic provider failures remain available through
`intrinsic_conflict()` and `Error::source()`.

The global entry point is for the complete set of linked registrations; a
conflict makes its initialization fail as a whole. A caller that needs an
isolated model view can instead add only its selected descriptors to an
explicit snapshot and pass that same snapshot to
`qubit_model_metadata::registry::ModelRegistry::from_reflect_registry`:

```text
let mut builder = RegistrySnapshotBuilder::new();
builder.add_type(
    TypeDescriptor::of::<MyModel>(),
    FragmentIdentity::new("example", "models", 1, 1, "type", 1),
);
let snapshot = builder.build()?;
let models = qubit_model_metadata::registry::ModelRegistry::from_reflect_registry(&snapshot)?;
```

This is an integration fragment: the application supplies `MyModel`, its model
metadata dependency, and the error-return context. An explicit snapshot starts
empty and does not automatically include every registration linked into the process.

## Choose features and access boundaries

### Choose dependency features

| Feature | Provides |
| --- | --- |
| `derive` (default) | The `Reflect`, `ReflectImpl`, and `bind_method!` macros. |
| `ecosystem-types` | Reflection implementations for `BigDecimal`, `DateTime<Utc>`, `NaiveDate`, `NaiveTime`, and `Uuid`. |
| `qubit-types` | Reflection implementations for `qubit_id::Id` and `qubit_datatype::DataType`; the `qubit-id` default generator feature is disabled. |

Choose one of the following alternatives for the `qubit-reflect` entry in
`[dependencies]`; do not paste all three into one manifest:

```toml
# Runtime descriptors, dynamic values, and handwritten registration only.
qubit-reflect = { version = "0.2", default-features = false }
```

```toml
# Macros plus BigDecimal, chrono, and UUID reflection implementations.
qubit-reflect = { version = "0.2", features = ["ecosystem-types"] }
```

```toml
# Macros plus Qubit DataType and Id reflection implementations.
qubit-reflect = { version = "0.2", features = ["qubit-types"] }
```

`ecosystem-types` and `qubit-types` are independent opt-ins. Neither belongs to
the default feature set, so a runtime-only consumer does not compile those
dependency families or silently acquire their trait implementations.
`qubit-types` only provides reflection for `qubit_id::Id`; if the application
also uses ID generators, add a direct `qubit-id` dependency with the needed
features instead of relying on features enabled transitively by this crate.
Each crate generating descriptors for these external types must enable the
matching feature on its own direct `qubit-reflect` dependency.

### Choose transparent or opaque boundaries

A transparent reflected type exposes its member structure and supported operations. Use the existing opaque type presentation when a value should be treated as a whole; it does not make a private member safe to access and does not replace application authorization. `#[reflect(skip)]` is stronger: the member is absent from the reflected descriptor and all queries.

The runtime does not define a separate cross-thread reflection mode. Keep synchronization in the application, for example by holding a mutex guard while calling `get`, `set`, or `invoke`. Rust's static `Send` and `Sync` checks continue to govern moving or sharing the underlying values.

## Errors, diagnostics, and troubleshooting

The API avoids implicit conversion: it does not coerce numeric values, parse strings, infer `Into`, or manufacture `Send`/`Sync` after type erasure. Match structured error categories, not `Display` text. Inspect recovery before retrying.

| Symptom | What to check |
| --- | --- |
| Cargo cannot resolve `qubit-datatype` or `qubit-id` | Check registry access and the resolved dependency graph with `cargo metadata --locked --format-version 1`; adjacent repository checkouts are not required. |
| `field("...")` returns `None` | Check the Rust member name and whether the member is marked `#[reflect(skip)]`. Skipped members are absent from reflection. |
| A field operation fails | Check public visibility and the exact target and value types; inspect `FieldSetFailure` to recover the original replacement. |
| Construction fails | Check shape, duplicate or missing fields, names or indices, and each value's type; recover inputs from `ConstructionRecovery`. |
| A method is visible but not callable | Check whether the method has a supported adapter and, for a generic method, a compile-time `bind_method!` instance binding. Inspect invocation recovery for rejected inputs. |
| Registry initialization or snapshot build fails | Inspect `RegistryError` and conflicting fragments; global initialization errors are cached for the process. |
| Cross-thread use is unavailable | Apply synchronization in the caller and satisfy Rust's `Send`/`Sync` constraints for values moved between threads. |
| An external type has no `Reflect` implementation | Enable `ecosystem-types` or `qubit-types` on the crate that owns the reflection boundary. |
| A derive cannot resolve the runtime | Declare `qubit-reflect` as a direct dependency and enable its default `derive` feature. |

Field access returns `FieldAccessError`. Pre-execution `set` rejections preserve the owned replacement in `FieldSetFailure`; errors after the adapter takes ownership have no recovery payload. Construction failures return `ConstructionRecovery`. Pre-execution invocation failures retain receiver and arguments in `InvocationRecovery`. Ordinary invocation always propagates method panics. Do not assume method panics are caught by reflection; follow the method's Rust panic behavior.

## Boundaries and a practice checklist

- Keep reflection attributes on the declaration that owns the contract. Generated code can reach private fields; reflection policies do not replace application authorization.
- `TypeId`, descriptor addresses, query names, and trait markers are process-local identity, not serialization or cross-process model IDs.
- Reflection does not parse request text, coerce numbers, or infer `Into`. Callers own synchronization and thread-safety policy.
- Unsafe functions, unsupported ABIs, variadics, unbound generic methods, and opaque `impl Trait` returns may be described but are not dynamically callable. Tuple and function-pointer descriptors support arities 0 through 32.
- Descriptors and per-type capability caches live for the whole process; initialization may allocate. Measure the paths your application uses if overhead matters.
- Tests should cover a successful field read and replace, unknown fields, type mismatch recovery, method invocation failure, and registry initialization failure on conflicting declarations.

## Migration notes

Each downstream crate that derives reflection must directly depend on
`qubit-reflect`; generated code resolves that dependency automatically.

### Migrating effective capability queries

The existing query names now return `Result`; no error-swallowing compatibility entry remains.
`capabilities` returns `Result<&TypeCapabilities, CapabilityConflict>`. The typed `capability`
query returns `Result<Option<&A>, CapabilityAccessError>`, while `capability_by_id` returns
`Result<Option<&CapabilityDescriptor>, CapabilityConflict>`. Handle errors before testing for
absence. Typed access errors distinguish `FactOnly` and `AdapterTypeMismatch`; intrinsic
declaration conflicts are wrapped as `CapabilityAccessError::IntrinsicConflict`.
Conflicts retain their kind, capability ID, and both adapter TypeIds. Registration failures also expose
the original conflict through `RegistryError::intrinsic_conflict()` and `Error::source()`.

The convenience `capability` query maps `Found` to `Ok(Some(adapter))` and `Missing` to `Ok(None)`.
Fact-only descriptors return a `FactOnly` error, and typed keys with a different adapter type return an
`AdapterTypeMismatch` error. Use `capability_lookup` when you need to inspect all four states:
`Missing`, `FactOnly`, `AdapterTypeMismatch`, and `Found`. Invalid intrinsic capability sets return their
conflict error; these lookup states are distinct from an invalid set.
`capability_origin` reports `CapabilityOrigin::Intrinsic { type_id }` or `CapabilityOrigin::Registered { source }`, while
`capability_source` returns the contributing `FragmentIdentity` when one is registered. The corresponding
`definition_capability_origin` and `definition_capability_source` methods apply to generic declarations.
`type_capability_members` and `definition_capability_members` iterate only frozen registry members
and do not execute factories. Each result carries the lookup state, origin, and source; fact-only
and adapter-mismatch entries remain visible, while capability-only targets are audited separately
with `capability_only_type_targets` and `capability_only_definition_targets`. Intrinsic capability
sources use the member declaration when it exists, otherwise the earliest fragment that triggered
inspection. Effective queries for unregistered
concrete instances may execute an intrinsic factory, without inserting the instance into the snapshot.

Providers must depend only on static type facts, never on snapshots, time, or mutable external
configuration, and must not re-enter registry initialization. Generic intrinsic factories run outside
the cache-map lock; successes and conflicts are cached by concrete `TypeId` and shared by concurrent
queries. Provider panics still propagate; they do not become absence or `CapabilityConflict`.
Generated invocation adapters preserve receiver-capability resolution failures as structured
invocation errors and restore the receiver, values, names, and original caller ordering.
Invocation itself does not initialize a registry.

### Explicit invocation migration and troubleshooting

The twelve consuming methods on `descriptor::InvocationAdapter` and
`MethodInstanceDescriptor` now use outer `Result` instead of `Option`: migrate
`None` to outer `Err`, recover the input explicitly, and migrate `Some(result)`
to `Ok(result)`. This is a breaking change; the inner validation and catching
contracts and function-pointer ABI remain unchanged. Outer `Err(InvocationUnavailable<I>)`
preserves the original input before method execution; use `reason()` to inspect
the cause and `into_invocation()` or `into_parts()` to recover it. Once user code
has started, consumed inputs and side effects cannot be restored.

Every `invoke_*` entry now requires a registry. If lookup succeeds but
invocation returns `ReceiverAdapterUnavailable`, check the selected snapshot
for the exact receiver capability and invocation mode. A static entry does not
guarantee that capability exists. The same key may select different adapters in
two snapshots without cross-contamination; global failure does not affect
valid local calls. Outputs and futures do not borrow the registry, but remain
constrained by input lifetimes.

Model metadata ABI `v7` and `definition_provider_v2` remain independent.
Debug prints structural facts without running providers; providers
themselves must not re-enter initialization.

## Further reading

- [README](../README.md) · [API reference](https://docs.rs/qubit-reflect)
- [Chinese user guide](user_guide.zh_CN.md) · [中文 README](../README.zh_CN.md)
