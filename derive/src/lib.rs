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

/// Reflects a struct, enum, or trait declaration.
///
/// Put `#[reflect(skip)]` on a field, enum variant, or method to exclude it
/// completely from reflection metadata and generated adapters. The Rust item
/// remains unchanged. Add `local_only` to a struct or enum when its fields are
/// not `Send + Sync` and should be available through local reflection only.
/// This option does not change the default behavior of a bare `#[Reflect]`,
/// and it does not alter `#[ReflectImpl]` method registrations.
///
/// `#[Reflect]` on a trait describes the trait declaration. It does not
/// describe an implementation; use `#[ReflectImpl]` on the implementation.
/// A bare Rust implementation without `#[ReflectImpl]` remains outside the
/// reflected application catalog and may be reported as `ExternalIncomplete`
/// when a reflected trait application is required.
///
/// # Example
///
/// ```text
/// #[qubit_reflect::Reflect(local_only)]
/// struct LocalRecord {
///     value: std::rc::Rc<u8>,
/// }
/// ```
///
/// The generated type remains usable with local descriptors and values; the
/// macro does not make `Rc` or the containing type thread-safe.
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Reflect(attribute: TokenStream, item: TokenStream) -> TokenStream {
    macros::reflect(attribute, item)
}

/// Reflects an inherent or trait implementation.
///
/// Methods marked with `#[reflect(skip)]` are omitted from reflected method
/// metadata and invocation adapters. The attribute applies to inherent and
/// trait implementations. For a trait implementation, it records that
/// application separately from the trait declaration; an unannotated Rust
/// implementation is not inferred from the compiler's global type system.
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn ReflectImpl(attribute: TokenStream, item: TokenStream) -> TokenStream {
    macros::reflect_impl(attribute, item)
}

/// Binds one explicitly monomorphized method to a dynamic invocation adapter.
///
/// The input is a method path followed by `as` and its concrete function
/// pointer type. The pointer type must include the receiver and concrete
/// argument and return types. The binding is available to the registration
/// context that selects its target type and implementation.
///
/// ```text
/// qubit_reflect::bind_method!(
///     Counter::add::<u32> as fn(&mut Counter, u32) -> u32
/// );
/// ```
#[proc_macro]
pub fn bind_method(input: TokenStream) -> TokenStream {
    macros::bind_method(input)
}

/// Registers one explicitly monomorphized method binding in the linked catalog.
///
/// Use the same concrete function-pointer form as [`bind_method!`]. The linked
/// catalog makes the binding discoverable; an explicit context still needs to
/// select the target type before that binding participates in its method
/// lookup.
///
/// ```text
/// qubit_reflect::register_method_binding!(
///     Counter::add::<u32> as fn(&mut Counter, u32) -> u32
/// );
/// ```
#[proc_macro]
pub fn register_method_binding(input: TokenStream) -> TokenStream {
    macros::register_method_binding(input)
}

/// Defines an opaque identity handle for an external trait declaration.
///
/// The handle identifies a trait that cannot be annotated at its source. It
/// does not prove that a particular type implements the trait. The caller is
/// responsible for supplying a truthful association, and context validation
/// reports missing reflected application metadata.
///
/// ```text
/// qubit_reflect::reflect_external_trait!(pub DisplayHandle = std::fmt::Display);
/// ```
#[proc_macro]
pub fn reflect_external_trait(input: TokenStream) -> TokenStream {
    macros::reflect_external_trait(input)
}

/// Registers a compiler-checked Rust trait-object type in linked contexts.
///
/// The trait-object type must be valid Rust syntax and satisfy the compiler's
/// object-safety rules. Registration makes the object type discoverable; it
/// does not provide implementations or bypass context selection.
///
/// ```text
/// qubit_reflect::register_trait_object!(dyn std::fmt::Display);
/// ```
#[proc_macro]
pub fn register_trait_object(input: TokenStream) -> TokenStream {
    macros::register_trait_object(input)
}

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
