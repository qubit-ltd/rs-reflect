// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Determines whether an identifier is owned by this crate or an external
/// crate.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct IdAuthority {
    /// Whether identifiers using the reserved core namespace are accepted.
    is_core: bool,
}

impl IdAuthority {
    /// Identifies an external owner, which cannot use the reserved namespace.
    pub(crate) const EXTERNAL: Self = Self { is_core: false };

    /// Identifies this crate as the owner, allowing the reserved namespace.
    pub(crate) const CORE: Self = Self { is_core: true };
}
