// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Procedural macros for `qubit-reflect`.

#![forbid(unsafe_code)]

mod configure;
mod entry;
mod expand;
mod internal;
mod ir;
mod macros;
mod parse;
mod validate;

use proc_macro::TokenStream;

/// Derives structural reflection for a struct or enum.
///
/// # Helper attributes
///
/// Place `#[reflect(...)]` on the declaration or its fields and variants:
///
/// | Scope | Supported options |
/// | --- | --- |
/// | Type | `rename = "name"`, `opaque`, `capabilities(...)`, `thread_safe`, `crate = path`, `definition_provider_v2 = identifier` |
/// | Field | `rename = "name"` (named fields only), `opaque`, `skip`, `read_only`, `no_construct`, `default`, `default = path` |
/// | Enum variant | `rename = "name"`, `skip`, `no_construct` |
///
/// `rename` changes lookup names while preserving Rust names. `opaque` hides
/// structural access, `read_only` withholds writes, and `no_construct`
/// withholds construction. A skipped field remains structural metadata without
/// access adapters; a skipped variant remains metadata without construction.
/// `default` uses the field's `Default` implementation; `default = path` calls
/// the supplied zero-argument provider during construction. `thread_safe`
/// requests field adapters whose concrete Rust bounds must hold.
///
/// In `capabilities(...)`, only bare `Clone`, `Default`, `Send`, and `Sync`
/// select built-ins, requiring the corresponding Rust bounds. Every other
/// path calls a generic provider as `path::<Self>()` returning a
/// `qubit_reflect::capability::CapabilityDescriptor`. Qualified paths retain
/// this meaning even if their final segment is a built-in name:
/// `capabilities(my_crate::Clone)` calls `my_crate::Clone::<Self>()`.
///
/// `crate = path` selects the runtime facade used by generated code, for
/// example `crate = my_runtime`. Omit it to resolve the installed runtime
/// crate. The facade must expose the versioned code-generation support.
/// `definition_provider_v2 = identifier` is a facade integration hook for a
/// generic type: it emits a caller-named definition provider, not a path.
///
/// Unions, misplaced or unknown helpers, duplicate query names, and conflicting
/// policies produce source-oriented compiler diagnostics. Concrete bounds and
/// custom provider signatures are checked by rustc. See the [user guide](https://github.com/qubit-ltd/rs-reflect/blob/main/doc/user_guide.md)
/// for generic types and facade integration.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// use qubit_reflect_derive::Reflect;
/// #[derive(Reflect)]
/// #[reflect(crate = qubit_reflect, rename = "account")]
/// struct Account {
///     #[reflect(read_only)]
///     id: u32,
/// }
/// # fn main() {
/// assert_eq!(TypeDescriptor::of::<Account>().query_name(), "account");
/// # }
/// ```
///
/// # Parameters
///
/// - `input`: Rust item tokens supplied to the derive macro.
///
/// # Returns
///
/// Returns generated reflection items or compiler diagnostics.
#[proc_macro_derive(Reflect, attributes(reflect))]
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    macros::derive_reflect(input)
}

/// Reflects a trait declaration and its metadata contract.
///
/// # Attribute scopes
///
/// Trait arguments support `rename = "name"`, `crate = path`,
/// `supertrait(Path, ...)`, `external_trait(Path, id = "stable.id")`, and
/// `dyn_compatible` or `dyn_compatible(Parent::AssociatedType, ...)`.
/// `supertrait(...)` selects reflected supertrait relationships;
/// `external_trait(...)` supplies a stable identity for an external supertrait
/// and may be repeated. The `dyn_compatible` proof requests a real generated
/// `dyn Trait` type, including listed inherited associated types; rustc checks
/// the proof. Trait methods accept `#[reflect(rename = "name", skip,
/// no_invoke, catch_unwind, thread_safe, specialize(T = Concrete, ...))]`
/// with only the desired, mutually compatible options. Associated types and
/// constants carry metadata but accept no reflection helpers.
///
/// `skip` withholds a method's adapter; `no_invoke` retains active metadata
/// without invocation. Unsupported signatures retain structured unavailable
/// reasons. `catch_unwind` applies only to synchronous methods on supported
/// targets; it cannot be combined with `no_invoke` or `skip`. `thread_safe`
/// requests a boundary with concrete Rust bounds and cannot be combined with
/// `no_invoke` or `skip`. Method `specialize(...)` selects concrete generic
/// arguments and may be repeated. Misplaced helpers, duplicate query names,
/// and conflicting policies produce compiler diagnostics.
///
/// Rust evaluates `cfg` and `cfg_attr` before helper validation, descriptor
/// collection, adapter generation, and specialization analysis. An inactive
/// member is absent from the reflected definition.
///
/// `crate = path` selects a runtime facade; omission resolves the installed
/// runtime crate. A macro-forwarding facade must re-export the versioned
/// internal derive support or enable the runtime facade's `derive` feature.
/// See the [user guide](https://github.com/qubit-ltd/rs-reflect/blob/main/doc/user_guide.md)
/// for facade integration and supertrait proofs.
///
/// # Examples
///
/// ```standalone_crate
/// # use qubit_reflect_derive::ConfiguredReflection;
/// use qubit_reflect::TypeDescriptor;
/// use qubit_reflect_derive::{Reflect, reflect, reflect_impl};
/// #[derive(Reflect)]
/// #[reflect(crate = qubit_reflect)]
/// struct Service;
/// #[reflect(crate = qubit_reflect)]
/// trait Named {
///     fn name(&self) -> &'static str;
/// }
/// #[reflect_impl(crate = qubit_reflect)]
/// impl Named for Service {
///     fn name(&self) -> &'static str { "service" }
/// }
/// # fn main() {
/// let implementations = TypeDescriptor::of::<Service>().impls_global()
///     .expect("valid registrations");
/// assert!(implementations.iter().any(|item| item.implemented_trait().is_some()));
/// # }
/// ```
///
/// # Parameters
///
/// - `attribute`: Reflection macro arguments.
/// - `item`: Trait declaration tokens.
///
/// # Returns
///
/// Returns the augmented trait and generated support items or diagnostics.
#[proc_macro_attribute]
pub fn reflect(attribute: TokenStream, item: TokenStream) -> TokenStream {
    macros::reflect(attribute, item)
}

/// Reflects an inherent or trait implementation.
///
/// # Attribute scopes
///
/// Impl arguments support `crate = path`, repeated `specialize(T = Concrete,
/// N = 4, ...)`, `external_trait_id = "stable.id"` (trait impls only), and
/// `definition_provider_v2 = identifier` to resolve a facade-provided reflected
/// trait definition provider by its function name.
/// `specialize(...)` registers selected concrete instances of a generic impl;
/// unspecialized generic declarations retain metadata. `external_trait_id`
/// identifies an external trait namespace. `crate = path` selects the runtime
/// facade; omission resolves the installed runtime crate. The facade must
/// expose versioned code-generation and internal derive support, or enable
/// its runtime `derive` feature when forwarding attribute macros.
///
/// Direct methods accept `#[reflect(...)]` with `rename = "name"`, `skip`,
/// `no_invoke`, `catch_unwind`, `thread_safe`, and repeated `specialize(...)`.
/// These options have the method semantics and conflicts documented by
/// [`macro@reflect`]; `catch_unwind` and `thread_safe` are method options, not
/// impl-level options. Associated types and constants accept no helpers.
/// Invalid scope, conflicting policies, invalid specializations, and duplicate
/// query names produce compiler diagnostics; rustc checks generated call
/// bounds.
///
/// Rust evaluates `cfg` and `cfg_attr` before helper validation, metadata
/// collection, adapter generation, and specialization analysis. This keeps
/// generated calls aligned with the members compiled into the impl. See the
/// [user guide](https://github.com/qubit-ltd/rs-reflect/blob/main/doc/user_guide.md)
/// for facade and generic registration examples.
///
/// # Examples
///
/// ```standalone_crate
/// # use qubit_reflect_derive::ConfiguredReflection;
/// use qubit_reflect::TypeDescriptor;
/// use qubit_reflect_derive::{Reflect, reflect_impl};
/// #[derive(Reflect)]
/// #[reflect(crate = qubit_reflect)]
/// struct Service;
/// #[reflect_impl(crate = qubit_reflect)]
/// impl Service {
///     #[reflect(no_invoke)]
///     fn ping(&self) {}
/// }
/// # fn main() {
/// let implementations = TypeDescriptor::of::<Service>().impls_global()
///     .expect("valid registrations");
/// assert!(implementations.iter().any(|item| item.method("ping").is_some()));
/// # }
/// ```
///
/// # Parameters
///
/// - `attribute`: Reflection macro arguments.
/// - `item`: Impl declaration tokens.
///
/// # Returns
///
/// Returns generated registration and invocation items or diagnostics.
#[proc_macro_attribute]
pub fn reflect_impl(attribute: TokenStream, item: TokenStream) -> TokenStream {
    macros::reflect_impl(attribute, item)
}

/// Internal carrier that resolves conditional configuration for generated
/// traits or impls.
///
/// This macro is emitted by the public attribute macros, not a user-facing
/// configuration entry point.
///
/// # Parameters
///
/// - `input`: Compiler-filtered internal carrier tokens.
///
/// # Returns
///
/// Returns the reflected declaration and generated support items, or compiler
/// diagnostics when the carrier is invalid.
#[proc_macro_derive(
    ConfiguredReflection,
    attributes(
        reflect,
        reflect_configure_header,
        reflect_configure_args,
        reflect_configure_kind,
        reflect_configure_member
    )
)]
#[doc(hidden)]
pub fn derive_configured_reflection(input: TokenStream) -> TokenStream {
    configure::process_configured(input)
}
