// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! SmartPointerKind category metadata.

/// A standard smart-pointer family.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::SmartPointerKind;
/// let pointer = SmartPointerKind::Box;
/// assert_eq!(pointer, SmartPointerKind::Box);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SmartPointerKind {
    /// [`Box<T>`](Box).
    Box,
    /// [`Rc<T>`](std::rc::Rc).
    Rc,
    /// [`Arc<T>`](std::sync::Arc).
    Arc,
}
