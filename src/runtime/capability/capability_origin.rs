// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::context_registration_origin::ContextRegistrationOrigin;

/// Describes how a capability is provided in the context that exposed it.
#[derive(Clone)]
pub enum CapabilityOrigin<'ctx> {
    /// The capability came from statically linked registration metadata.
    Linked {
        /// Context-bound source registration information.
        registration: ContextRegistrationOrigin<'ctx>,
    },
    /// The capability was added explicitly while building the context.
    Explicit {
        /// Context-bound source registration information.
        registration: ContextRegistrationOrigin<'ctx>,
    },
}
