// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! method visibility definitions.

use crate::identity::Visibility;

/// Where a method declaration obtains its source visibility.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::MethodVisibility;
/// use qubit_reflect::identity::Visibility;
/// let visibility = MethodVisibility::Declared(Visibility::Public);
/// assert!(matches!(visibility, MethodVisibility::Declared(Visibility::Public)));
/// ```
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum MethodVisibility {
    /// Visibility declared by an inherent or implementation method.
    Declared(Visibility),
    /// Trait-item reachability inherited from the declaring trait.
    InheritedFromTrait,
}
