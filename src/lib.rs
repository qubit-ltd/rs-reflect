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
//! opt-in through [`macro@Reflect`] and [`ReflectImpl`] attributes when the
//! `derive` feature is enabled.
//!
//! # Quick start
//!
//! ```
//! use qubit_reflect::TypeDescriptor;
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
//! let value = vec![String::from("Ada")];
//! assert_eq!(value[0], "Ada");
//! ```
//!
//! To reflect an application record and update a named field, enable the
//! default `derive` feature:
//!
//! ```rust
//! # #![allow(proc_macro_derive_resolution_fallback)]
//! # #[cfg(feature = "derive")]
//! # fn main() {
//! use qubit_reflect::{Reflect, TypeDescriptor};
//!
//! mod example {
//!     use qubit_reflect::Reflect;
//!     #[Reflect]
//!     pub struct Customer { pub email: String }
//! }
//!
//! let mut customer = example::Customer { email: String::from("old@example.com") };
//! let email = TypeDescriptor::of::<example::Customer>()
//!     .field("email")
//!     .expect("derived field");
//! email
//!     .set(&mut customer, String::from("new@example.com"))
//!     .expect("exactly typed replacement");
//! assert_eq!(customer.email, "new@example.com");
//! # }
//! # #[cfg(not(feature = "derive"))]
//! # fn main() {}
//! ```
//!
//! # Guides
//!
//! - [English user guide](https://github.com/qubit-ltd/rs-reflect/blob/main/doc/user_guide.md)
//! - [中文用户指南](https://github.com/qubit-ltd/rs-reflect/blob/main/doc/user_guide.zh_CN.md)
#![deny(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]

pub use crate as qubit_reflect;

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

/// A structured error raised while reading or writing a reflected field.
pub use access::FieldAccessError;
/// A failed field replacement together with its recoverable inputs.
pub use access::FieldSetFailure;
/// The untouched replacement value and field identity retained after a
/// failed field set.
pub use access::FieldSetRecovery;
/// A typed capability contract error.
pub use capability::CapabilityAccessError;
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
/// One source-level field in a reflected generic declaration.
pub use descriptor::FieldDefinitionDescriptor;
/// One reflected field declaration.
pub use descriptor::FieldDescriptor;
/// Error returned when a reflected optional value cannot be projected.
pub use descriptor::OptionalProjectionError;
/// Structural and projection operations for optional root descriptors.
pub use descriptor::OptionalTypeDescriptor;
/// The static reflection contract implemented by reflected Rust types.
pub use descriptor::Reflect;
/// The immutable source-level descriptor for a generic type declaration.
pub use descriptor::TypeDefinitionDescriptor;
/// The process-local identity of a generic type declaration.
pub use descriptor::TypeDefinitionId;
/// The immutable root descriptor for a reflected Rust type.
pub use descriptor::TypeDescriptor;
/// One source-level enum variant in a reflected generic declaration.
pub use descriptor::VariantDefinitionDescriptor;
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
/// The receiver and arguments retained after pre-execution validation
/// fails.
pub use invoke::InvocationRecovery;
/// The `Reflect` procedural attribute for reflected Rust type and trait
/// declarations.
#[cfg(feature = "derive")]
pub use qubit_reflect_derive::Reflect;
/// The `ReflectImpl` procedural attribute for reflected Rust impl blocks.
#[cfg(feature = "derive")]
pub use qubit_reflect_derive::ReflectImpl;
/// Build a compile-time method-instance binding for runtime reflection.
#[cfg(feature = "derive")]
pub use qubit_reflect_derive::bind_method;
// Registry facade.
/// The effective implementation and method view for one registered root.
pub use registry::EffectiveTypeView;
/// The immutable global registry of linked reflection fragments.
pub use registry::ReflectRegistry;
// Dynamic-value facade.
/// Internal dependencies needed by code generated by reflection macros.
#[doc(hidden)]
#[path = "private/mod.rs"]
pub mod __private;

#[cfg(test)]
mod tests;
