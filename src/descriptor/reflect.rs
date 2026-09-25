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
    fn type_descriptor() -> &'static TypeDescriptor;
}
