# `qubit-reflect` Final Requirements Specification

- Date: 2026-09-03
- Status: normative internal specification; publishing is intentionally deferred
- Translation source: [简体中文需求规范](2026-08-28-qubit-reflect-requirements.zh_CN.md)
- Scope: declaration macros, descriptors, checked access, invocation, construction, and registry integration

The key words **MUST**, **SHOULD**, **MUST NOT**, and **MAY** are normative.
This English edition preserves exactly the same 284 requirement identifiers as
the Chinese edition. The [traceability matrix](2026-09-03-qubit-reflect-requirements-traceability.md)
maps every identifier to current implementation and executable tests.

## System

- **REQ-SYS-001**: `qubit-reflect` MUST remain an independent infrastructure layer and MUST NOT depend on `qubit-model-derive` or `qubit-model-metadata`.
- **REQ-SYS-002**: Every descriptor MUST be read-only, immutable, statically shareable, and safe for concurrent queries.
- **REQ-SYS-003**: Reflection MUST be opt-in; the system MUST NOT claim to discover types, traits, or impls that did not use a reflection macro.
- **REQ-SYS-004**: Descriptors MUST expose strongly typed structure; an arbitrary string map MUST NOT be the core public API.
- **REQ-SYS-005**: Querying a descriptor MUST NOT require constructing an instance of the described type.
- **REQ-SYS-006**: Dynamic operations MUST preserve Rust type, ownership, borrowing, and thread-safety boundaries; mismatches MUST return errors and MUST NOT cause undefined behavior.
- **REQ-SYS-007**: Errors decidable from one macro input MUST be diagnosed at compile time; only cross-declaration aggregation, runtime values, and dynamic arguments MAY fail at runtime.
- **REQ-SYS-008**: Public descriptor APIs, errors, and macro behavior MUST have Rustdoc, and final user-guide examples MUST be acceptance inputs.
- **REQ-SYS-009**: Descriptor graphs MUST support recursive type relationships without unbounded traversal or formatting recursion.
- **REQ-SYS-010**: Identical fragment sets MUST produce deterministic member order across compilation, linking, and querying; fields and variants use source order, while distributed impls MUST NOT use linker enumeration or first-query thread order.
- **REQ-SYS-011**: The target API requires `std` and the manifest `rust-version`; `no_std`/`alloc` are out of scope. CI defines supported platforms, and platforms unable to support distributed static registration MUST fail explicitly at compile time.
- **REQ-SYS-012**: The reflection layer guarantees structural correctness and safe dynamic operations, not validation, uniqueness, referential integrity, cross-field conditions, or other domain validity.
- **REQ-SYS-013**: First-party runtime and generated code MUST be compatible with `#![forbid(unsafe_code)]`; third-party unsafe code MAY be used only behind safe APIs without transferring unchecked invariants to callers.

## Declaration Macros

- **REQ-MAC-001**: Structs and enums MUST opt in with `#[derive(Reflect)]`.
- **REQ-MAC-002**: Except for an explicit type-level `#[reflect(opaque)]`, derive MUST record every direct field, including private, tuple, and enum-variant fields; type-level opaque MUST hide all members.
- **REQ-MAC-003**: A non-opaque derive MUST record field source order, Rust name, declared type, and visibility; tuple fields have an index but no name.
- **REQ-MAC-004**: Derive MUST implement the public `Reflect` query trait so generic code can require `T: Reflect`.
- **REQ-MAC-005**: Unions MUST be rejected with a compile-time diagnostic at the declaration.
- **REQ-MAC-006**: Traits MUST opt in with `#[reflect]`.
- **REQ-MAC-007**: The trait macro MUST describe identity, visibility, supertraits, method signatures, receivers, parameters, and return types.
- **REQ-MAC-008**: The trait macro MUST describe associated types, associated constants, generic methods, and default methods; only items with an explicit concrete invocation adapter MAY execute dynamically.
- **REQ-MAC-009**: `#[reflect]` MUST NOT change trait semantics, object safety, or dispatch.
- **REQ-MAC-010**: Every impl block intended to appear in reflection for a reflected struct or enum MUST be marked `#[reflect_impl]`; unrelated impls MAY remain unmarked.
- **REQ-MAC-011**: `#[reflect_impl]` MUST support inherent and trait impls.
- **REQ-MAC-012**: Unmarked impl blocks and methods MUST NOT appear in reflection; omission is the caller's responsibility and is not a framework error.
- **REQ-MAC-013**: Marked impls for one type MAY be distributed across modules and files and MUST be aggregated by final queries.
- **REQ-MAC-014**: A complete reflected-trait impl requires the trait itself to use `#[reflect]`; an unreflected external trait MAY produce an explicitly incomplete descriptor containing only facts present in the impl.
- **REQ-MAC-015**: Macros MUST NOT change original method resolution, visibility, generic constraints, or trait-impl semantics.
- **REQ-MAC-016**: Procedural macros live in the separate `qubit-reflect-derive` crate and runtime APIs in `qubit-reflect`; the runtime crate re-exports the three macros by default so users need one direct dependency. Both crates are currently internal and unpublished.
- **REQ-MAC-017**: Generated code MUST resolve renamed dependencies and MUST NOT hard-code only `qubit_reflect`; the derive crate MUST NOT create a Cargo dependency cycle back to the runtime.
- **REQ-MAC-018**: `#[reflect_impl]` for an unreflected trait MUST provide `external_trait_id = "..."`; reflected traits MUST reject it. Missing, malformed, or reserved-namespace IDs MUST fail compilation.
- **REQ-MAC-019**: Helpers use `#[reflect(...)]` with the documented target-specific keys for types, fields, variants, methods, and traits; misplaced, duplicate, or mutually exclusive keys MUST fail compilation.
- **REQ-MAC-020**: Concrete generic impls use `#[reflect_impl(specialize(...))]` and generic methods use `#[reflect(specialize(...))]`; complete named type/const arguments are required, and duplicates, unknowns, omissions, or unsatisfied predicates MUST fail compilation.
- **REQ-MAC-021**: External supertraits or bounds in a reflected trait MUST map explicitly through `external_trait(path, id = "...")`; path/ID conflicts, missing mappings, and reserved namespaces MUST fail, and the mapping MUST NOT invent unexposed associated items.

## Descriptor Identity and Structure

- **REQ-DESC-001**: Every descriptor returned by a public query MUST provide a complete public navigable read-only interface.
- **REQ-DESC-002**: References between descriptors MUST be static or live as long as their root and MUST never dangle.
- **REQ-DESC-003**: Types, traits, fields, variants, impls, methods, and parameters MUST have explicit identities rather than Debug-text identity.
- **REQ-DESC-004**: Rust names and query names MUST be distinct concepts and equal only when no rename applies.
- **REQ-DESC-005**: Descriptors MUST distinguish members declared on a type from members contributed by an impl.
- **REQ-DESC-006**: Descriptor identity MUST support structured errors, deduplication, and qualified queries. Types use process-local `TypeId`, reflected traits use marker `TraitId`, and members use a composite owner/category/declaration identity; identity implements process-local `Eq`/`Hash` without cross-build persistence guarantees.
- **REQ-DESC-007**: Domain attributes such as `#[text]` MUST NOT become built-in reflection semantics. Reflection exposes neither arbitrary attributes nor `syn`/`TokenStream`; domain macros generate their own metadata.
- **REQ-DESC-008**: Impl-fragment identity MUST include declaring crate, module path, source location, and generated content fingerprint. Duplicate or conflicting identity MUST fail deterministically; addresses and linker order MUST NOT participate.
- **REQ-DESC-009**: Visibility MUST normalize public, crate, super, restricted path, and private forms; inherited trait-item and enum-field visibility MUST be represented without inventing an explicit `pub`.
- **REQ-DESC-010**: `TypeExpression` MUST be a `syn`-independent structural tree covering concrete types, parameters, `Self`, associated types, references, raw pointers, slices, arrays, tuples, function pointers, trait objects, opaque `impl Trait`, never, generic arguments, bounds, and lifetimes. Source text is diagnostic only.
- **REQ-DESC-011**: Type, trait, field, variant, and method descriptors MUST expose immutable Rust name/path and query name separately; name lookup uses query names, diagnostics retain both, and rename MUST NOT alter Rust identity or `type_name()`.

## Types and Capabilities

- **REQ-TYPE-001**: A Rust type implementing `Reflect` MUST have one static root through `TypeDescriptor::of::<T>()`; types without that impl cannot use the entry point.
- **REQ-TYPE-002**: `Reflect::type_descriptor()` and `TypeDescriptor::of::<T>()` MUST return the same object.
- **REQ-TYPE-003**: `type_id()` is the process-local Rust `TypeId`, not a stable cross-build, cross-process, or persistent ID.
- **REQ-TYPE-004**: `type_name()` supplies a diagnostic Rust name and MUST NOT be used as a stable protocol ID.
- **REQ-TYPE-005**: Struct `fields()` returns direct fields; an enum root has no fields, and variant fields are queried through `VariantDescriptor`.
- **REQ-TYPE-006**: Navigation inapplicable to a kind MUST return an empty collection or `None`, never panic.
- **REQ-TYPE-007**: Built-ins MUST cover primitives, `String`, `()`, `Option<T>`, `Vec<T>`, arrays, tuples, `str`, slices, `Box<T>`, `Rc<T>`, `Arc<T>`, `HashMap`, `BTreeMap`, `HashSet`, and `BTreeSet`; descriptor support does not imply generic dynamic construction.
- **REQ-TYPE-008**: Lookups MUST distinguish missing from ambiguous results and MUST NOT silently choose one same-named method.
- **REQ-TYPE-009**: `Reflect` is the sole public generic constraint for static descriptors; model crates MUST NOT create an equivalent `HasTypeDescriptor` hierarchy.
- **REQ-TYPE-010**: `TypeDescriptor::of` MUST cover the built-ins as well as derived declarations. Non-opaque concrete fields require `Reflect`; no query or registry state may auto-downgrade them to opaque. Only explicit field `#[reflect(opaque)]` permits an unresolved `'static` field type.
- **REQ-TYPE-011**: Struct shape MUST distinguish named, tuple/newtype, and unit forms through `StructKind`, not only `TypeKind::Struct`.
- **REQ-TYPE-012**: `TypeDescriptor::of` is the static primary query; `ReflectRegistry` supports `TypeId`, full enumeration, and zero-to-many diagnostic-name candidates without treating `type_name` as a model ID.
- **REQ-TYPE-013**: `TypeKind` MUST be hierarchical across primitive, text, struct, enum, tuple, array, optional, sequence, set, map, smart pointer, reference, slice, raw pointer, function pointer, trait object, and opaque. `()` is a zero-arity tuple; closures and unnameable function items remain declaration-level opaque expressions.
- **REQ-TYPE-014**: Extensible `TypeCapabilities` MUST include built-in `Send`, `Sync`, `Clone`, and `Default`, preserve and forward non-conflicting extensions, and never bypass operation type checks.
- **REQ-TYPE-015**: An opaque member MUST permit exact-`TypeId` whole-value read, write, argument passing, and enclosing construction, but MUST NOT expose internals or independently construct that type's root.
- **REQ-TYPE-016**: Capabilities use stable dotted `CapabilityId` namespaces; `qubit.reflect.*` is reserved, third parties use their own namespace, and identity MUST NOT depend on allocated bits or link order.
- **REQ-TYPE-017**: The library registers core capabilities for built-ins; user capabilities require derive metadata or explicit registration. Rust-trait-backed declarations MUST be compile-time bound checked and false claims MUST fail compilation.
- **REQ-TYPE-018**: A capability MAY carry a safe dynamic adapter. Core `Clone` and `Default` MUST provide dynamic operations; extensions are obtained through their `CapabilityId` and public contract without unchecked conversion.
- **REQ-TYPE-019**: `TypeCapabilities` MUST be immutable and deterministically iterable; duplicate IDs, incompatible contracts, or reserved namespace conflicts fail at compile time when local and during aggregation only when necessarily cross-unit.
- **REQ-TYPE-020**: `kind()` returns a stable hierarchy, while kind-specific navigation lives on typed views such as `as_struct`, `as_sequence`, `as_map`, and `as_function`, not unrelated flattened root methods.
- **REQ-TYPE-021**: The registry contains linked reflected and built-in roots only. `TypeDescriptor::of` is independent of global name lookup; distributed-impl queries return `RegistryError`, while pure field and variant queries MUST NOT panic because of registry conflicts.
- **REQ-TYPE-022**: `Reflect` and `TypeDescriptor::of` support `?Sized + 'static`, allowing descriptors for `str`, slices, and dyn-compatible traits. Except the safe built-in `str` borrow, unsized values are descriptive only and MUST NOT cross `Any`-based values, construction, or invocation through raw-pointer workarounds.
- **REQ-TYPE-023**: `PrimitiveKind` covers all Rust primitive scalars. `String` and `str` use distinct owned/borrowed `TextKind` values; `str` is not a character slice.
- **REQ-TYPE-024**: Container and smart-pointer views retain the exact family and all arguments; references/raw pointers retain mutability, and function pointers retain ABI, safety, variadic status, parameters, and return expression.
- **REQ-TYPE-025**: `ReflectRegistry` provides separate zero-to-many `find_by_type_name` and `find_by_query_name`; cross-crate query-name conflicts require caller disambiguation, while member conflicts within one type fail at compile time.
- **REQ-TYPE-026**: Capability sets describe registered adapters, not negative reflection over arbitrary Rust trait impls. Missing means `NotRegistered`; dynamic-wrapper constructors still rely on call-site bounds and MUST accept values already proven safe.
- **REQ-TYPE-027**: The internal `DescriptorInterner` and public `ReflectRegistry` remain separate. On-demand generic/composite interning MUST NOT mutate the frozen enumeration; only static definitions and explicit concrete registrations appear there.
- **REQ-TYPE-028**: Field types use `TypeRef`: resolvable reflected concrete fields are `Resolved`, explicit opaque fields are `Opaque`, and unspecialized generic-definition fields are `Symbolic`. `OpaqueTypeDescriptor` is a member view, not a second root, and explicit opacity persists even if the concrete type implements `Reflect` elsewhere.
- **REQ-TYPE-029**: Type-level `#[reflect(opaque)]` creates that type's unique `TypeKind::Opaque` root with no fields, variants, or internal navigation, but MAY retain compile-time-validated capabilities. Type and field opacity MUST be explicit, never inferred or downgraded automatically.

## Fields and Variants

- **REQ-FLD-001**: Every field records declaring type, source index, optional name, field type, and source visibility.
- **REQ-FLD-002**: Named fields support query-name and index lookup; tuple/newtype fields support index only and return `None` for both names.
- **REQ-FLD-003**: Private fields remain discoverable through descriptors without changing ordinary Rust access rules.
- **REQ-FLD-004**: A returned field borrow MUST NOT outlive the target borrow, and mutable access MUST NOT create aliasing mutable borrows.
- **REQ-FLD-005**: Field writes validate target and value types before mutation.
- **REQ-FLD-006**: Validation failure MUST NOT partially modify the target.
- **REQ-FLD-007**: Read/write errors include field identity and expected/actual types when available.
- **REQ-FLD-008**: Reflection MUST NOT merge getter/setter methods into a field; a downstream layer MAY derive a property.
- **REQ-FLD-009**: Enum-variant fields inherit their containing access boundary and MUST NOT be represented as explicitly `pub`.
- **REQ-FLD-010**: Non-opaque concrete fields navigate through `TypeRef::Resolved`; missing `Reflect` MUST produce a field-local compile error.
- **REQ-FLD-011**: Explicit opaque fields always remain `TypeRef::Opaque`; whole-value access, arguments, and enclosing construction still perform exact `TypeId` checks.
- **REQ-VAR-001**: A non-opaque enum describes all variants in source order; a type-level opaque enum exposes no variants.
- **REQ-VAR-002**: Variants distinguish unit, tuple, and struct shapes.
- **REQ-VAR-003**: Variant fields follow struct-field rules for index, name, type, visibility, and safe access.
- **REQ-VAR-004**: The active variant of an enum value MUST be checkable safely.
- **REQ-VAR-005**: Reading a field of an inactive variant returns an explicit error without panic or invalid memory access.
- **REQ-VAR-006**: A variant records whether its discriminant is explicit or implicit, and the enum records valid layout/discriminant `repr`; diagnostic text is not a numeric protocol.
- **REQ-VAR-007**: Variant-field identity includes enum root, variant, and position; same-named or same-indexed fields in different variants are distinct.
- **REQ-VAR-008**: Only fieldless integer-`repr` enums expose exact numeric discriminants and reverse lookup, including Rust-computed implicit values; other enums MUST NOT invent numeric mappings.
- **REQ-VAR-009**: Missing reverse lookup returns `None` or a structured missing result; duplicate or overflowing declarations remain compiler errors rather than runtime arbitrary selection.

## Methods and Traits

- **REQ-MTH-001**: A method records name, owning impl, visibility, receiver, non-receiver parameters, return type, and qualifiers.
- **REQ-MTH-002**: Receivers distinguish no receiver, `self`, `&self`, and `&mut self` at minimum.
- **REQ-MTH-003**: Parameter indexes exclude the receiver; names are diagnostic/binding aids, not stable source-independent ABI.
- **REQ-MTH-004**: Methods distinguish inherent and trait provenance, and trait methods navigate to their trait descriptor.
- **REQ-MTH-005**: Same-named inherent methods and methods from different traits MUST NOT overwrite one another.
- **REQ-MTH-006**: Descriptors retain `async`, `unsafe`, `const`, `extern`, generic, and other callability facts even when invocation is unavailable.
- **REQ-MTH-007**: Associated functions are receiverless methods and remain separate from dynamic type construction.
- **REQ-MTH-008**: `#[reflect_impl]` describes all methods in the block, including restricted/private inherent methods, without changing Rust visibility; invocation still requires safe signature and policy.
- **REQ-MTH-009**: Every described method separately reports dynamic callability and all structured blocking reasons, including receiver, pattern, generic, ABI, unsafe, or return-borrow restrictions.
- **REQ-MTH-010**: Parameter names are optional: identifiers are named, wildcard/destructure patterns are unnamed and retain a `syn`-independent `ParameterPatternDescriptor`. Positional invocation supports every safe pattern; named binding requires unique simple identifiers.
- **REQ-MTH-011**: Parameters retain structural `TypeExpression`, optional concrete navigation, and owned/shared/mutable passing mode. `ReturnDescriptor` distinguishes unit, never, concrete, reference, and opaque `impl Trait` without fake ordinary descriptors.
- **REQ-MTH-012**: Generic method declarations retain type/lifetime/const parameters, bounds, and predicates; only finite explicit `specialize` instances enter dynamic invocation, never runtime monomorphization search.
- **REQ-MTH-013**: Unsized borrowed signatures are fully described but callable only through a safe dedicated adapter; `&str`/`&mut str` use built-in text adapters, and a sized owner MUST NOT impersonate the exact unsized parameter type.
- **REQ-TRT-001**: `TraitDescriptor` describes a trait independently of any concrete implementation.
- **REQ-TRT-002**: `ImplDescriptor` targets one type and distinguishes inherent from trait impls.
- **REQ-TRT-003**: A trait impl links both its trait and target type descriptors.
- **REQ-TRT-004**: Impl methods link their concrete implementation rather than only the abstract trait signature.
- **REQ-TRT-005**: The effective method view includes unoverridden defaults marked `Defaulted`, with generated target adapters when safe; explicit overrides are `Overridden` and navigate to the concrete impl method.
- **REQ-TRT-006**: Trait descriptors expose direct supertraits and a deterministic deduplicated cycle-safe transitive closure; reflected supertraits are complete and external ones incomplete.
- **REQ-TRT-007**: Blanket, generic, and constrained impls have definition descriptors with complete parameters and predicates. Runtime never enumerates infinite instances; only explicit concrete impl registrations enter effective views.
- **REQ-TRT-008**: Each reflected trait generates a hidden marker whose `TypeId` forms process-local `TraitId`; diagnostic path is not identity, and dyn compatibility is not required.
- **REQ-TRT-009**: An unreflected trait impl links an `ExternalIncomplete` descriptor containing only provable path, target, and impl methods; supported methods MAY still invoke safely, but defaults and unobserved trait facts MUST NOT be invented.
- **REQ-TRT-010**: `TraitDescriptor` always models the declaration. A dyn-compatible trait additionally has a separate `dyn Trait` `TypeDescriptor`; non-dyn-compatible traits do not. Cross-file uncertainty requires explicit `dyn_compatible` proof, including inherited associated-type bindings, which rustc validates through a real generated dyn type.
- **REQ-TRT-011**: Every unreflected external trait requires a stable namespaced `ExternalTraitId`; different source aliases aggregate by ID. Incompatible duplicate impls for one target/ID fail deterministically.
- **REQ-TRT-012**: Associated-type descriptors retain name, bounds, default expression, and order; impls retain concrete or symbolic bindings, and only concretely resolved bindings participate in dynamic calls.
- **REQ-TRT-013**: Associated-constant descriptors retain name, declared type, order, and default status; impls identify default versus override and provide a safe owned reader when the value can cross the dynamic boundary.
- **REQ-TRT-014**: `TraitId` distinguishes reflected marker `TypeId` from stable `ExternalTraitId` with process-local `Eq`/`Hash`; equal diagnostic names do not equate them, and external identity is not automatically a model domain ID.
- **REQ-TRT-015**: Generic traits distinguish definition and applied descriptors. Reflected and external applied identities include concrete arguments, and applied views substitute supertraits, associated items, and method signatures; unapplied definitions have no fake callable runtime identity.

## Dynamic Values and Field Access

- **REQ-VAL-001**: Dynamic values carry enough identity for exact type checking.
- **REQ-VAL-002**: Borrowed values retain the original lifetime in the type system and MUST NOT erase it to `'static`.
- **REQ-VAL-003**: Failed owned downcasts MUST NOT lose the original value; the caller can recover it or the consumption point is explicit in the contract.
- **REQ-VAL-004**: JSON, strings, and Debug text MUST NOT be universal dynamic intermediates.
- **REQ-VAL-005**: Actual value types determine `Send`/`Sync`; type erasure MUST NOT add them unconditionally.
- **REQ-VAL-006**: Public safe APIs MUST NOT require callers to maintain unchecked raw-pointer invariants.
- **REQ-VAL-007**: Dynamic wrappers follow the explicit `Any` contracts below and MUST NOT be replaced by bare lifetime/thread-unconstrained `dyn Any`.
- **REQ-VAL-008**: `Reflect` is the only user-facing reflection trait; wrappers construct directly from suitable Rust values without a second public value trait.
- **REQ-VAL-009**: Default wrappers are `Local` and do not unconditionally implement `Send`/`Sync`; a separately bounded `ThreadSafe` mode accepts only qualifying values, and conversion MUST NOT add auto traits from runtime metadata.
- **REQ-VAL-010**: Local owned construction accepts `'static`; thread-safe owned construction requires `'static + Send + Sync`. Successful downcast returns `T`, failed owned downcast returns the intact wrapper, and borrowed downcast failure leaves the borrow unchanged.
- **REQ-VAL-011**: Local and thread-safe wrappers share mode-parameterized underlying types with ergonomic aliases. Modes determine erasure and auto traits at compile time, not through a runtime enum.
- **REQ-VAL-012**: Wrappers primarily expose `is`/`downcast`, plus safe `as_any`, `as_any_mut`, and `into_any` for Any-compatible storage. Dedicated unsized variants return `None`/the wrapper and use typed accessors; conversions preserve lifetime and thread guarantees.
- **REQ-VAL-013**: Local `DynamicRef/Mut::new` requires sized `'static`; thread-safe shared borrow additionally requires `Sync`, and mutable borrow requires `Send + Sync`, all enforced by bounds rather than capabilities or raw pointers.
- **REQ-VAL-014**: `DynamicRef/Mut` includes a safe `str` variant with typed constructors/accessors, preserving lifetime and mode for exact borrowed arguments/returns without entering owned or `dyn Any` storage.
- **REQ-VAL-015**: Every `ThreadSafe` wrapper provides infallible consuming `into_local` that preserves value, identity, and lifetime. No runtime-capability-based Local-to-ThreadSafe upgrade exists.
- **REQ-ACC-001**: `get` validates that the target's actual type equals the declaring type.
- **REQ-ACC-002**: `get_mut` and `set` require an exclusive mutable target borrow.
- **REQ-ACC-003**: `set` requires the exact declared field type without widening, parsing, or inferred `Into`.
- **REQ-ACC-004**: Private fields MAY be accessed by adapters generated inside an opted-in type, but equivalent access MUST NOT be given to unreflected types.
- **REQ-ACC-005**: Reflection access is not blocked by source `pub` because derive is explicit authorization; visibility remains available to policy layers.
- **REQ-ACC-006**: `read_only` preserves description/shared read but disables mutable read/set. `skip` preserves indexed structure while disabling relevant dynamic access, invocation, and construction; attributes MUST NOT erase structural facts or renumber fields.
- **REQ-ACC-007**: `rename` changes only query name; `rust_name` stays the source name. Empty or colliding query names fail compilation, and errors retain Rust names.
- **REQ-ACC-008**: Method `no_invoke` preserves description but disables invocation; field/variant `no_construct` disables construction. Misapplied attributes fail compilation, and no visibility-bypassing `expose` exists.

## Invocation and Construction

- **REQ-INV-001**: Receiverless functions, `&self`, `&mut self`, and `self` methods use distinct invocation input constraints.
- **REQ-INV-002**: Argument count or type mismatch fails before user code executes.
- **REQ-INV-003**: `&mut self` requires an exclusive mutable borrow; `self` consumes an owned value.
- **REQ-INV-004**: Outputs MAY be owned or borrow from receiver/arguments, and the API preserves that lifetime relationship in types.
- **REQ-INV-005**: Ordinary invoke propagates user panics unchanged. Catching exists only for explicit `catch_unwind`, compile-time-validates unwind safety, returns separate `InvocationPanic` under unwind, reports unavailable under abort, and MUST NOT infer unwind safety through specialization.
- **REQ-INV-006**: Unsafe functions are descriptive only and receive no safe or unsafe dynamic adapter, preserving `forbid(unsafe_code)` compatibility.
- **REQ-INV-007**: A `const fn` MAY run as an ordinary runtime function; descriptors record constness without dynamic const evaluation.
- **REQ-INV-008**: Async invocation returns `ReflectedFuture<'call, Mode>` and never selects or runs an executor. Local accepts non-`Send`; ThreadSafe requires an explicitly requested, compile-time-validated `Send` future, with ordinary poll panic semantics.
- **REQ-INV-009**: Only finite explicit generic method specializations invoke dynamically; each records concrete type/const arguments and owns an adapter. Unspecialized declarations remain descriptive with a structured reason.
- **REQ-INV-010**: Safe adapters cover `Box<Self>`, `Rc<Self>`, `Arc<Self>`, `Pin<&Self>`, `Pin<&mut Self>`, and `Pin<Box<Self>>`; other arbitrary-self forms require an exact registered `ReceiverAdapter`, and no implicit pinning is allowed.
- **REQ-INV-011**: Method lookup supports impl/trait qualification rather than resolving ambiguity by bare name alone.
- **REQ-INV-012**: All borrows in a call reborrow to one conservative `'call`; borrowed output records receiver or parameter origins, and HRTB receives an adapter only when safely instantiable at `'call`.
- **REQ-INV-013**: Every pre-user-code failure returns `InvocationRecovery`, preserving owned inputs in order and borrowed relationships. Post-entry behavior follows the original signature's consumption semantics.
- **REQ-INV-014**: `InvocationArg` distinguishes owned/shared/mutable input with exact mode matching; only mutable-to-shared reborrow is implicit. Positional arguments are canonical, while named binding requires uniquely named simple identifiers.
- **REQ-INV-015**: `InvocationOutput` distinguishes unit, owned, shared borrow, mutable borrow, and future. Normal return from never is unreachable; non-async opaque `impl Trait` is descriptive-only with stable `OpaqueReturnType` reason unless changed to a nameable type.
- **REQ-INV-016**: Invocation adapters default to Local. Only explicit `thread_safe` emits ThreadSafe after compile-time checking receiver, inputs, owned outputs, futures, and borrow rules; absence means unregistered, not proof of inherent non-thread-safety.
- **REQ-CON-001**: Named structs construct by field name, tuple/newtype structs by position, and unit structs without input.
- **REQ-CON-002**: By default every field appears exactly once; missing, duplicate, and unknown inputs have distinct errors.
- **REQ-CON-003**: All field values validate before construction begins, so failure exposes no partial object.
- **REQ-CON-004**: Constructing private fields is an internal operation explicitly authorized by derive and independent of caller visibility.
- **REQ-CON-005**: `Option<T>` is not implicitly absent. Only explicit `default` or `default = path` permits omission, with compile-time `Default` or exact provider signature validation.
- **REQ-CON-006**: Unit, tuple, and struct variants expose construction matching their shape.
- **REQ-CON-007**: Calling the wrong construction shape returns a `WrongShape` category.
- **REQ-CON-008**: Variant construction returns the owned enum root, not an unwrapped payload.
- **REQ-CON-009**: Variant fields follow struct construction completeness, duplicate, and exact-type rules.
- **REQ-CON-010**: Failed validation MUST NOT leak, double-drop, or lose inputs and occurs before observable value construction; recovery follows invocation's ownership rule.
- **REQ-CON-011**: A skipped/non-constructible field needs an explicit default provider or whole struct/variant zero-construction is unavailable with structured reasons; uninitialized memory MUST NOT bypass it.
- **REQ-CON-012**: Struct update consumes one exact owned base plus overrides, validates all names, duplicates, and types before rebuilding, preserves non-overridden fields, and returns base plus overrides on failure.
- **REQ-CON-013**: A verified root `Default` capability MAY construct dynamically but remains independent of per-field omission and MUST NOT silently fill ordinary construct inputs.
- **REQ-CON-014**: Descriptors expose default providers, field source, and construct/update availability so tools can decide before attempting execution.

## Aggregation and Generics

- **REQ-AGG-001**: All marked impl fragments for one type aggregate into one query entry point.
- **REQ-AGG-002**: Aggregation supports impls and declarations in different files/modules.
- **REQ-AGG-003**: Impl discovery MUST NOT scan source trees, parse other files, or use rustc-private interfaces.
- **REQ-AGG-004**: Unmarked impls do not exist in reflection, which MUST NOT claim completeness relative to rustc.
- **REQ-AGG-005**: Duplicate fragment registration produces a deterministic diagnostic or initialization error and never duplicate enumeration.
- **REQ-AGG-006**: The aggregated view is immutable and safely concurrently readable.
- **REQ-AGG-007**: Every type, trait, and impl macro submits an immutable static fragment. A safe once primitive lazily builds one `ReflectRegistry`, and `initialize()` exposes all errors; mutable hot registration is not required.
- **REQ-AGG-008**: Dynamic-library unloading and runtime hot registration are out of scope.
- **REQ-AGG-009**: Determinism applies to equal linked fragment sets; different binaries MAY contain different sets, but linker accidents and first-query thread order MUST NOT affect each result.
- **REQ-AGG-010**: Distributed registration covers same-crate modules, dependency crates, and valid downstream trait impls in the final binary. Unsupported platforms fail compilation or explicitly disable aggregation APIs, never return incomplete success.
- **REQ-AGG-011**: Aggregation MAY use an unsafe linker-section dependency only through safe wrappers; CI verifies completeness, duplicates, and concurrent initialization on every guaranteed platform.
- **REQ-AGG-012**: Impl sorting uses category, full reflected path or external ID, declaring crate, module, and source location; inherent precedes trait and methods preserve source order.
- **REQ-AGG-013**: `ReflectRegistry::initialize()` returns cached `Result<&'static ReflectRegistry, RegistryError>` safely under concurrency. Pure structure is independent; aggregate impl/method queries propagate cached errors.
- **REQ-AGG-014**: Generic/blanket definitions MAY register globally but never expand infinite concrete instances; only explicit specializations attach to a concrete root.
- **REQ-AGG-015**: Interning is a cache, not hot registration, and MUST NOT change frozen membership, ordering, or conflicts. Generic concrete roots discoverable by registry `TypeId` require static specialization/registration.
- **REQ-GEN-001**: Generic definitions and concrete monomorphizations are distinct; for example `Page<User>` and `Page<Order>` have distinct `TypeId`s.
- **REQ-GEN-002**: Concrete owned values, `TypeId` identities, and construction results are `'static`; borrowed method inputs/outputs use call lifetimes without requiring a `'static` borrow.
- **REQ-GEN-003**: Generated bounds are minimal. Resolved generic fields add the `Reflect` needed to form descriptors; parameters used only in opaque fields do not, but still satisfy accurate dynamic identity requirements.
- **REQ-GEN-004**: `GenericDefinitionDescriptor` retains lifetime/type/const parameters, bounds, defaults, and predicates; each concrete descriptor navigates to it and ordered concrete arguments.
- **REQ-GEN-005**: Definition metadata fully records lifetimes without fake runtime identity. `'static` monomorphizations containing references MAY operate dynamically; non-`'static` roots remain expression-only and cannot enter owned `Any` storage.
- **REQ-GEN-006**: Arrays, tuples, standard containers, slices, references, raw pointers, function pointers, and trait objects expose kind-specific child navigation. Safe APIs never dereference raw pointers, while unsafe signatures remain describable.
- **REQ-GEN-007**: Concrete type arguments navigate to descriptors; const arguments retain declared type, normalized diagnostic expression, and an owned reflected value where representable. Source const text is not cross-build identity.
- **REQ-GEN-008**: Generic method/impl/blanket definition descriptors are separate from concrete instances; instances retain all arguments and compile-time predicate results without runtime trait solving.
- **REQ-GEN-009**: HRTB and named lifetime relationships use structural `LifetimeExpression`; adapters exist only when safely instantiable at one `'call`, otherwise the full signature remains descriptive with a reason.
- **REQ-GEN-010**: Generic concrete and composite descriptors are concurrently interned by `TypeId`; identical concrete types return one static pointer. Bounded process-lifetime leaking is allowed, but generic-function statics MUST NOT be assumed per-monomorphization; recursive links resolve lazily without deadlock or partial publication.
- **REQ-GEN-011**: Source attributes, not later monomorphization traits, determine opaque/resolved field shape consistently across all instances of a definition.
- **REQ-GEN-012**: Lifetime-parameterized derived types implement `Reflect` only for `Self: 'static`; full generic declarations remain navigable from valid instances, with no fake non-static `TypeId` or duplicate public descriptor trait. Every derived generic struct or enum MUST register one first-class `TypeDefinitionDescriptor` without requiring a concrete monomorphization. Definition fields and variants MUST remain descriptive and MUST NOT expose runtime field, construction, invocation, or active-variant adapters. Concrete generic descriptors MUST link to the shared definition identity while retaining their own `TypeId` and resolved arguments. `ReflectRegistry` MUST be the only public effective-capability resolver for concrete types and generic definitions; textual lookup MUST accept `&str` without allocating a `CapabilityId`.

## Model Integration and Errors

- **REQ-INT-001**: Model-layer type and field structure comes from `qubit-reflect`, not a second independent implementation.
- **REQ-INT-002**: `FieldMetadata` MAY wrap/link `FieldDescriptor` and add identifier, unique, reference, constraint, validator, codec, and redaction facts.
- **REQ-INT-003**: Property is not core reflection; model layers MAY derive field-backed, computed, or virtual properties from field and method descriptors.
- **REQ-INT-004**: Entity, Projection, Model, Enum, and Value role macros generate equivalent `Reflect` capability and static fragments directly, without requiring users to stack another derive.
- **REQ-INT-005**: Reflection MUST NOT depend on or understand `ModelId`, identifiers, relations, validation, codecs, or redaction.
- **REQ-INT-006**: Model field/method name queries reuse reflection identity and ambiguity rules.
- **REQ-INT-007**: Model metadata/derive depend on and reuse `qubit-reflect`; the reverse dependency and a duplicate `HasTypeDescriptor` system are prohibited.
- **REQ-INT-008**: A generator using only `TypeDescriptor` guarantees Rust type, complete fields, and shape. Static domain constraints require `TypeMetadata`; contextual uniqueness, references, and external validation require higher layers.
- **REQ-INT-009**: Domain attributes MUST NOT become built-in reflection semantics. A role macro MAY parse once and generate both reflection and model metadata without duplicate user annotations.
- **REQ-INT-010**: `TypeDescriptor` cannot directly return model metadata; reverse lookup belongs in a model wrapper, extension trait, or registry to avoid a dependency cycle.
- **REQ-INT-011**: Model crates implement structure through the public reflection contract. Role macros delegate to the facade's compatible `Reflect` derive, avoid duplicate structure generators, expose only hidden `codegen_v2` plus required derive paths, do not require full runtime module re-exports or terminal direct dependencies, and reject stacking explicit `Reflect` derive.
- **REQ-INT-012**: This specification is authoritative across the three repositories for structure, dynamic operations, and dependency direction; conflicts are fixed downstream, never by adding domain concepts or compatibility forks to reflection.
- **REQ-ERR-001**: Compile diagnostics point to the relevant declaration, field, variant, impl, or method rather than only the macro name.
- **REQ-ERR-002**: Error messages state the violated rule and provide supported syntax when practical.
- **REQ-ERR-003**: Ordinary invalid macro input yields `compile_error!`/`syn::Error`, not a procedural-macro panic.
- **REQ-ERR-004**: Public dynamic operations return structured errors rather than strings alone.
- **REQ-ERR-005**: Errors retain available operation kind, descriptor identity, argument/field path, and expected/actual types.
- **REQ-ERR-006**: Error types implement `Debug`, `Display`, and `std::error::Error`.
- **REQ-ERR-007**: Display text is not a stable machine protocol; consumers match public categories.
- **REQ-ERR-008**: Invalid dynamic input MUST NOT panic; only documented internal-invariant violations are framework bugs.
- **REQ-ERR-009**: Pre-execution field set, invocation, construction, and update errors carry structured recovery for still-owned inputs and provide non-panicking retrieval by original name/index.
- **REQ-ERR-010**: `InvocationPanic` remains separate from `InvocationError`, retains method identity and safely retainable payload, and is never misreported as input error; payload Display is not protocol.
- **REQ-ERR-011**: `RegistryError` distinguishes duplicate fragment, identity/content conflict, external-trait ID conflict, capability conflict, and unsupported platform; cached errors are repeatable and query-order independent.
- **REQ-ERR-012**: Unavailable invocation/construction reports a stable category and all blocking reasons, not only one string, so tools can explain unsafe, unspecialized, borrow, and policy restrictions before execution.

## Explicit Non-goals

- **REQ-OUT-001**: The system does not provide general runtime reflection for unannotated Rust types.
- **REQ-OUT-002**: It does not parse the workspace to discover impls or use rustc-private APIs.
- **REQ-OUT-003**: It provides no general backdoor around type safety, borrowing, or privacy of non-opted-in types; safe private-member adapters generated at the owner's annotated declaration are explicit authorization.
- **REQ-OUT-004**: Neither `TypeId` nor `type_name` is a persistent cross-version schema ID.
- **REQ-OUT-005**: Reflection macros, compiler plug-ins, and dynamic libraries need not hot-load at runtime.
- **REQ-OUT-006**: Arbitrary signatures not expressible through safe Rust need not invoke dynamically; they remain describable with an unavailable reason.
- **REQ-OUT-007**: Core reflection defines no serialization format, database mapping, validation, domain roles, or access-control policy.
- **REQ-OUT-008**: Core reflection defines no BeanRandom/fixture distribution and does not guarantee model-valid or context-valid constructed values; structural, constraint-aware, and contextual generation are separate downstream levels.
- **REQ-OUT-009**: `no_std`/`alloc` are out of scope, and any external port MUST preserve identity, structured errors, and safe operation contracts.
- **REQ-OUT-010**: Core construction targets derived structs/enums/variants and does not infer generic construction from container kind; a concrete composite MAY register a verified extension adapter such as `ConstructFromElements`.

## Acceptance Requirements

- **REQ-ACCPT-001**: Compile-pass/fail tests cover legal and illegal targets plus key signature restrictions for all three macros.
- **REQ-ACCPT-002**: Tests cover description and construction for named, tuple, newtype, unit structs and all three variant shapes.
- **REQ-ACCPT-003**: Tests cover public/private field read, mutable read, write, errors, and borrow boundaries.
- **REQ-ACCPT-004**: Tests cover description and supported invocation for receiverless, shared, mutable, owned, and trait-impl methods.
- **REQ-ACCPT-005**: Tests aggregate marked impls from multiple files and exclude unmarked impls.
- **REQ-ACCPT-006**: Tests prove same-named inherent and multi-trait methods do not overwrite and qualified lookup disambiguates.
- **REQ-ACCPT-007**: Compile-fail tests prove dynamic APIs cannot extend lifetimes, alias mutable borrows, or add `Send`/`Sync` incorrectly.
- **REQ-ACCPT-008**: Tests prove all invalid dynamic input fails before user code or target mutation.
- **REQ-ACCPT-009**: Tests prove recursive descriptors terminate initialization, traversal, and Debug formatting.
- **REQ-ACCPT-010**: At least one model-layer integration test proves model Field/Property reuses reflection descriptors.
- **REQ-ACCPT-011**: Integration tests prove `String`, `Option`, `Vec`, and confirmed composites are queryable and navigable as fields, parameters, or returns.
- **REQ-ACCPT-012**: Tests prove descriptive-only methods report stable structured reasons across at least three of pattern, generic, unsafe, and receiver boundaries.
- **REQ-ACCPT-013**: Tests cover input consumption/recovery, drop counts, and no-user-code execution on invocation/construction validation failure.
- **REQ-ACCPT-014**: Model boundary tests prove roles reuse the same `Reflect` root while domain constraints stay out of core descriptors.
- **REQ-ACCPT-015**: Tests cover map/set kinds and children and prove descriptor presence does not imply generic construction.
- **REQ-ACCPT-016**: Tests cover exact whole-value opaque-field read/write/argument/enclosing construction while rejecting internal navigation and independent root construction.
- **REQ-ACCPT-017**: Compile-fail tests prevent Local wrappers crossing threads and prove ThreadSafe constructors require the exact `Send`/`Sync` bounds.
- **REQ-ACCPT-018**: Tests cover unit as a zero-arity tuple and reference, slice, raw-pointer, function-pointer, and trait-object navigation, with no safe raw dereference.
- **REQ-ACCPT-019**: Tests prove owned downcast recovery, unchanged borrowed state on failure, and rejection of non-`Send`/`Sync` thread-safe owned construction.
- **REQ-ACCPT-020**: Tests cover explicit/implicit discriminants and reverse lookup for fieldless integer-repr enums and no fake numeric API elsewhere.
- **REQ-ACCPT-021**: Compile-pass/fail tests validate real capability trait bounds, reserved namespaces, duplicate IDs, and deterministic preservation of extensions.
- **REQ-ACCPT-022**: Tests verify mode generics/aliases have expected compile-time auto traits and safe adapters for core Clone, Default, and one extension.
- **REQ-ACCPT-023**: Each `TypeKind` typed view is tested for correct matches, wrong-kind `None`, and absence of unrelated flattened navigation.
- **REQ-ACCPT-024**: Tests cover marker identity for dyn-compatible and non-dyn-compatible traits, and dyn root existence/navigation only when valid.
- **REQ-ACCPT-025**: Tests prove unreflected external impls are incomplete, retain only provable facts, allow safe methods, and invent no defaults or associated items.
- **REQ-ACCPT-026**: Tests cover direct/transitive supertraits, associated type declarations/bindings, associated constant default/override reads, and defaulted/overridden method provenance and invocation.
- **REQ-ACCPT-027**: Tests cover definitions for generic type/method/impl/blanket impl, const, lifetime, and HRTB and prove only explicit concrete specialization enters effective invocation.
- **REQ-ACCPT-028**: Tests cover private methods by default and legal targets, conflicts, and diagnostics for rename, skip, read-only, no-invoke, no-construct, default, capability, and specialization attributes.
- **REQ-ACCPT-029**: Cross-crate tests spanning at least two dependencies cover distributed registration, deterministic ordering, conflict errors, concurrent first query, explicit initialization, and link-order independence.
- **REQ-ACCPT-030**: Tests cover complete-field construction, explicit defaults/providers, non-defaulted Option, unavailable fields, struct update, and recovery of base plus inputs on every failure path.
- **REQ-ACCPT-031**: Tests separately cover ordinary panic propagation, explicit unwind catching, compile failure for non-unwind-safe annotations, and catching unavailable under `panic=abort`.
- **REQ-ACCPT-032**: Tests cover Local async borrowing, explicitly thread-safe `Send` futures, compile failure for non-`Send` futures, and no implicit polling.
- **REQ-ACCPT-033**: Tests cover `Box`, `Rc`, `Arc`, all required `Pin` receivers, one extension receiver adapter, and descriptive-only behavior without a safe adapter.
- **REQ-ACCPT-034**: Compile-pass/fail tests cover common `'call` reborrowing, receiver/argument origins, mutable-to-shared reborrow, no owned implicit borrow, non-static root restrictions, and safe HRTB instantiation.
- **REQ-ACCPT-035**: Tests cover identifier/wildcard/destructure patterns, unit/never/reference/opaque returns, positional/named binding, and stable descriptive-only opaque-return reasons.
- **REQ-ACCPT-036**: Tests aggregate aliased external traits by one `ExternalTraitId` and deterministically reject duplicate impl identity or unmergeable facts.
- **REQ-ACCPT-037**: Tests cover default macro re-exports, direct derive-crate advanced use, and generated paths under renamed Cargo dependencies.
- **REQ-ACCPT-038**: Tests cover `TypeId`, duplicate diagnostic-name candidates, enumeration, and absence of arbitrary attributes or public `syn` dependency.
- **REQ-ACCPT-039**: Tests cover unsized `str`, slice, and dyn descriptors; built-in `str` borrowing/invocation; compile rejection of unsupported slice/dyn wrappers/construction; and exact thread-safe borrow bounds.
- **REQ-ACCPT-040**: Tests exhaust primitive/text, container/pointer families, reference/raw mutability, function ABI/safety/variadic navigation, and `syn` independence of structural expressions.
- **REQ-ACCPT-041**: Concurrent tests prove one descriptor pointer per `TypeId`, isolation of concrete arguments, and deadlock-free complete direct/indirect recursion.
- **REQ-ACCPT-042**: Tests cover reflected/external generic trait definitions and multiple applied descriptors with correct identity, supertrait, and associated-item substitution and no erroneous merging.
- **REQ-ACCPT-043**: Tests prove on-demand generic/composite interning after initialization does not alter enumeration/name indexes; explicit registration remains discoverable and concurrent interner growth preserves order.
- **REQ-ACCPT-044**: Compile-pass/fail tests cover missing `Reflect` versus explicit opaque fields, minimal resolved/opaque generic bounds, no opacity auto-upgrade, no second root through `TypeRef`, and unique member-free type-level opaque roots.
- **REQ-ACCPT-045**: Tests cover valid targets, compile-time bounds, and unmarked absence for catch/thread-safe attributes, plus external supertrait mapping, missing mappings, and conflicts.
- **REQ-ACCPT-046**: Tests prove lifetime-generic derives produce roots only for `'static` concrete instances, retain full lifetime/symbolic definition navigation, and reject non-static dynamic roots.
- **REQ-ACCPT-047**: Cross-crate integration proves model role macros delegate through the facade to the same derive, terminal users need no direct reflection dependency, minimal facade supports struct/enum/generic/trait/impl paths, model/reflection root identity matches, and duplicate explicit derive is diagnosed.
- **REQ-ACCPT-048**: Tests cover lossless `into_local` for all three ThreadSafe wrappers, reject runtime/capability Local upgrades, and use downgraded values for field access and dynamic construction.
