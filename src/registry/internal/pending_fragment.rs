// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Private pre-materialization registry fragment state.

use crate::identity::FragmentIdentity;
use crate::registry::fragment::RegistrationFragment;

/// A sorted fragment paired with its already materialized stable identity.
pub(crate) struct PendingFragment {
    /// Static fragment factory to invoke after deterministic sorting.
    pub(crate) fragment: &'static RegistrationFragment,
    /// Owned stable identity used to order and diagnose the fragment.
    pub(crate) identity: FragmentIdentity,
}
