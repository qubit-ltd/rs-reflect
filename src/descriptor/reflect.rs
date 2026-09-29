// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! The generic contract for types that provide static reflection descriptors.

use crate::descriptor::TypeDescriptor;

/// The sole public generic contract for types that provide a static reflection
/// descriptor.
///
/// Implementations must return the same immutable, process-lifetime root on
/// every call, with `descriptor.type_id() == std::any::TypeId::of::<Self>()`.
/// The implementation owns descriptor construction and concrete type identity;
/// registration and capability aggregation belong to the selected registry.
///
/// [`TypeDescriptor::of`] checks the returned `TypeId` and panics on a
/// mismatch. Calling [`Reflect::type_descriptor`] directly does not add that
/// check and does not repair an invalid handwritten implementation. Registry
/// aggregation reports registration conflicts as structured
/// [`crate::error::RegistryError`] values; invalid implementations are not
/// guaranteed to fail in one uniform way.
///
/// # Examples
///
/// ```
/// use qubit_reflect::{Reflect, TypeDescriptor};
///
/// let descriptor = <u32 as Reflect>::type_descriptor();
/// assert!(std::ptr::eq(descriptor, TypeDescriptor::of::<u32>()));
/// ```
pub trait Reflect: 'static {
    /// Returns the unique root descriptor for `Self`.
    ///
    /// # Returns
    ///
    /// The unique immutable root for this concrete type, carrying its exact
    /// `TypeId`. Repeated calls must return the same descriptor address.
    #[must_use]
    fn type_descriptor() -> &'static TypeDescriptor;
}
