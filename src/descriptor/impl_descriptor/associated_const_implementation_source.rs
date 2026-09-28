// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! AssociatedConstImplementationSource metadata and behavior.

/// The effective source of an associated constant value.
///
/// This value distinguishes a trait-provided default from an explicit impl
/// override.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::AssociatedConstImplementationSource;
/// assert_eq!(AssociatedConstImplementationSource::Defaulted, AssociatedConstImplementationSource::Defaulted);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AssociatedConstImplementationSource {
    /// The implementation uses the trait declaration's default value.
    Defaulted,
    /// The implementation explicitly overrides the constant.
    Overridden,
}
