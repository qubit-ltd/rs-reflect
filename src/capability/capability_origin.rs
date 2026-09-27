// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Origins retained for capability facts.

use std::any::TypeId;

use crate::identity::FragmentIdentity;

/// Identifies where an effective capability fact came from.
///
/// # Examples
///
/// ```
/// use std::any::TypeId;
/// use qubit_reflect::capability::CapabilityOrigin;
///
/// let origin = CapabilityOrigin::Intrinsic { type_id: TypeId::of::<u32>() };
/// assert!(matches!(origin, CapabilityOrigin::Intrinsic { .. }));
/// ```
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum CapabilityOrigin {
    /// The capability was declared by the reflected type itself.
    Intrinsic {
        /// The concrete type whose declaration supplied the capability.
        type_id: TypeId,
    },
    /// The capability was contributed by a registration fragment.
    Registered {
        /// The identity of the fragment that declared the capability.
        source: FragmentIdentity,
    },
}
