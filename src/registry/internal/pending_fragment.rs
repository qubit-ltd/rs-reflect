// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================

//! Private pre-materialization registry fragment state.

use crate::identity::FragmentIdentity;
use crate::registry::fragment::RegistrationFragment;

/// A sorted fragment paired with its already materialized stable identity.
pub(crate) struct PendingFragment {
    pub(crate) fragment: &'static RegistrationFragment,
    pub(crate) identity: FragmentIdentity,
}
