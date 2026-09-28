// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Concrete trait applications and declaration substitutions.

mod applied_trait_id;
mod trait_application_substitutions;
mod trait_id;
mod trait_impl_payload;

pub use applied_trait_id::AppliedTraitId;
pub(crate) use trait_application_substitutions::TraitApplicationSubstitutions;
pub use trait_id::TraitId;
pub use trait_impl_payload::TraitImplPayload;
