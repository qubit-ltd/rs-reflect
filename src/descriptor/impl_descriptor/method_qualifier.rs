// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! MethodQualifier metadata and behavior.

use crate::descriptor::TraitDescriptor;

/// A qualifier used to resolve methods across implementation namespaces.
///
/// # Type Parameters
///
/// - `'a`: Lifetime of the borrowed trait descriptor in [`Self::Trait`].
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::MethodQualifier;
/// let qualifier = MethodQualifier::Any;
/// assert!(matches!(qualifier, MethodQualifier::Any));
/// ```
#[derive(Clone, Copy, Debug)]
pub enum MethodQualifier<'a> {
    /// Search inherent and every trait namespace.
    Any,
    /// Search only inherent implementations.
    Inherent,
    /// Search one concrete applied trait namespace, using the contained trait.
    Trait(&'a TraitDescriptor),
}
