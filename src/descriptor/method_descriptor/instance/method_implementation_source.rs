// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Effective implementation source categories.

/// The effective source of a concrete method instance.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::MethodImplementationSource;
/// assert_eq!(MethodImplementationSource::Declared, MethodImplementationSource::Declared);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MethodImplementationSource {
    /// A method declared directly by an inherent impl.
    Declared,
    /// A required trait declaration has no implementation adapter.
    Required,
    /// The instance uses a trait default for the concrete target type.
    Defaulted,
    /// The implementation explicitly overrides the trait declaration.
    Overridden,
}
