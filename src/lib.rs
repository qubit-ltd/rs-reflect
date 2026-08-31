// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Macro-based structural and executable reflection for Rust.
//!
//! The crate exposes immutable descriptors plus checked field access, method
//! invocation, construction, capability, and registry APIs. Reflection is
//! opt-in through [`Reflect`] and the `reflect` and `reflect_impl` attribute
//! macros when the `derive` feature is enabled.
//!
//! # Quick start
//!
//! ```
//! use qubit_reflect::{ReflectedOwned, TypeDescriptor};
//!
//! let descriptor = TypeDescriptor::of::<Vec<String>>();
//! let element = descriptor
//!     .as_sequence()
//!     .expect("Vec exposes a sequence view")
//!     .element_type()
//!     .as_resolved()
//!     .expect("String has a reflected root");
//! assert!(std::ptr::eq(element, TypeDescriptor::of::<String>()));
//!
//! let value = ReflectedOwned::new(vec![String::from("Ada")]);
//! let value = value
//!     .downcast::<Vec<String>>()
//!     .unwrap_or_else(|_| unreachable!("exact dynamic type"));
//! assert_eq!(value[0], "Ada");
//! ```
//!
//! # User guide
#![doc = include_str!("../doc/2026-08-29-qubit-reflect-user-guide.md")]
#![forbid(unsafe_code)]

/// APIs for reflected field and variant access.
pub mod access;
/// Built-in reflected type descriptors.
pub mod builtin;
/// Capability descriptors for reflected types.
pub mod capability;
/// Construction APIs for reflected values.
pub mod construct;
/// Type and member descriptor APIs.
pub mod descriptor;
/// Reflection error types.
pub mod error;
/// Type expression and generic definition APIs.
pub mod expression;
/// Type identity and naming APIs.
pub mod identity;
/// Invocation APIs for reflected methods.
pub mod invoke;
/// Reflection registry APIs.
pub mod registry;
/// Dynamic value APIs.
pub mod value;

// Field access facade.
pub use access::FieldAccessError;
/// A failed field replacement together with its recoverable inputs.
pub use access::FieldSetFailure;
/// The target borrow and replacement value retained after a failed field set.
pub use access::FieldSetRecovery;
// Construction facade.
/// A dynamic construction error.
pub use construct::ConstructionError;
/// Owned inputs retained after dynamic construction fails.
pub use construct::ConstructionRecovery;
/// Dynamic construction input for named struct and variant fields.
pub use construct::NamedConstructionInput;
/// Dynamic owned base plus named replacement fields for a struct update.
pub use construct::StructUpdateInput;
/// Dynamic construction input for tuple struct and variant fields.
pub use construct::TupleConstructionInput;
// Descriptor facade.
/// The concrete arguments applied to one generic definition.
pub use descriptor::ConcreteGenericDescriptor;
/// One reflected field declaration.
pub use descriptor::FieldDescriptor;
/// The static reflection contract implemented by reflected Rust types.
pub use descriptor::Reflect;
/// The immutable root descriptor for a reflected Rust type.
pub use descriptor::TypeDescriptor;
/// One reflected enum variant declaration.
pub use descriptor::VariantDescriptor;
// Error facade.
/// A deterministic registry initialization or lookup error.
pub use error::RegistryError;
/// The structured category of a registry error.
pub use error::RegistryErrorKind;
/// An exact dynamic type mismatch.
pub use error::TypeMismatch;
// Invocation facade.
/// A complete dynamic method invocation input.
pub use invoke::Invocation;
/// One owned, shared, or mutable dynamic invocation argument.
pub use invoke::InvocationArg;
/// A failed invocation together with its recoverable inputs.
pub use invoke::InvocationFailure;
/// A unit, owned, borrowed, mutable, or future invocation result.
pub use invoke::InvocationOutput;
/// The receiver and arguments retained after pre-execution validation fails.
pub use invoke::InvocationRecovery;
#[cfg(feature = "derive")]
pub use qubit_reflect_derive::Reflect;
#[cfg(feature = "derive")]
pub use qubit_reflect_derive::reflect;
#[cfg(feature = "derive")]
pub use qubit_reflect_derive::reflect_impl;
// Registry facade.
/// The effective implementation and method view for one registered root.
pub use registry::EffectiveTypeView;
/// The immutable global registry of linked reflection fragments.
pub use registry::ReflectRegistry;
// Dynamic-value facade.
/// Local-mode mutable dynamic borrow.
pub use value::ReflectedMut;
/// Local-mode owned dynamic value.
pub use value::ReflectedOwned;
/// Local-mode shared dynamic borrow.
pub use value::ReflectedRef;
/// Thread-safe mutable dynamic borrow.
pub use value::SendReflectedMut;
/// Thread-safe owned dynamic value.
pub use value::SendReflectedOwned;
/// Thread-safe shared dynamic borrow.
pub use value::SendReflectedRef;

/// Internal dependencies needed by code generated by reflection macros.
#[doc(hidden)]
#[path = "private/mod.rs"]
pub mod __private;

#[cfg(test)]
mod tests;
