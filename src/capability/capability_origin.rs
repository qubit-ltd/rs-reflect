// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

//! Origins retained for capability facts.

use crate::identity::FragmentIdentity;

/// Identifies where an effective capability fact came from.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum CapabilityOrigin {
    /// The capability was declared by the reflected type itself.
    Intrinsic,
    /// The capability was contributed by a registration fragment.
    Registered {
        /// The identity of the fragment that declared the capability.
        source: FragmentIdentity,
    },
}
