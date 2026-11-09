// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Shared reflected root used by the cross-crate registry fixture.

mod bounded_const_service;
mod cross_crate_service;
pub mod registry_dyn_parent;
mod registry_user;

pub use bounded_const_service::BoundedConstService;
pub use cross_crate_service::CrossCrateService;
pub use registry_dyn_parent::RegistryDynParent;
pub use registry_user::RegistryUser;
