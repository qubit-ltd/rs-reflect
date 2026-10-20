// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::runtime::identity::RegistrationOrigin;

/// Records the registration source in a context-independent capability record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CapabilityRegistrationOrigin {
    /// The capability came from statically linked registration metadata.
    Linked(RegistrationOrigin),
    /// The capability was added explicitly while building a context.
    Explicit(RegistrationOrigin),
}
