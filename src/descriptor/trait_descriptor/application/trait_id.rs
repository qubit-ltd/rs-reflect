// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Process-local identities for reflected and external traits.

use std::any::TypeId;

use crate::identity::ExternalTraitId;

/// The process-local identity source of a reflected or external trait.
///
/// # Examples
///
/// ```
/// use std::any::TypeId;
/// use qubit_reflect::descriptor::TraitId;
/// struct Marker;
/// let identity = TraitId::Reflected(TypeId::of::<Marker>());
/// assert!(matches!(identity, TraitId::Reflected(_)));
/// ```
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TraitId {
    /// A marker type generated for a reflected trait declaration.
    Reflected(TypeId),
    /// A stable, caller-supplied identity for an unreflected trait.
    External(ExternalTraitId),
}
