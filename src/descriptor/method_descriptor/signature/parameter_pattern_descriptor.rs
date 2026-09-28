// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! parameter pattern descriptor definitions.

/// The source pattern category of a non-receiver parameter.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::ParameterPatternDescriptor;
/// let pattern = ParameterPatternDescriptor::Identifier;
/// assert!(matches!(pattern, ParameterPatternDescriptor::Identifier));
/// ```
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ParameterPatternDescriptor {
    /// A simple identifier that can participate in named binding.
    Identifier,
    /// A wildcard pattern without a bindable name.
    Wildcard,
    /// A destructuring pattern retained for positional binding and diagnostics.
    Destructure(Box<str>),
}
