// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Type identity and naming APIs.

mod capability_id;
mod external_trait_id;
mod fragment_identity;
mod member_id;
mod visibility;
mod visibility_kind;

pub use capability_id::CapabilityId;
pub use external_trait_id::ExternalTraitId;
pub use fragment_identity::FragmentIdentity;
pub use member_id::MemberId;
pub use visibility::Visibility;
pub use visibility_kind::VisibilityKind;
