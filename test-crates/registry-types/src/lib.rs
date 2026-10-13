// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Shared reflected root used by the cross-crate registry fixture.

mod generic_bound_service;
mod registry_dyn_parent;
mod registry_user;

pub use generic_bound_service::BoundedConstService;
pub use registry_dyn_parent::RegistryDynParent;
pub use registry_user::CrossCrateService;
pub use registry_user::RegistryUser;
