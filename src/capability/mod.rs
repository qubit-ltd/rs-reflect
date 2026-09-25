// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed, immutable capabilities attached to reflected concrete types.

mod builtin;
mod capability_access_error;
mod capability_descriptor;
mod capability_key;
mod capability_lookup;
mod capability_origin;
mod clone_adapter;
mod default_adapter;
mod registration;
mod set;

pub use builtin::clone_descriptor;
pub use builtin::clone_key;
pub use builtin::default_descriptor;
pub use builtin::default_key;
pub use builtin::send_descriptor;
pub use builtin::send_key;
pub use builtin::sync_descriptor;
pub use builtin::sync_key;
pub use capability_access_error::CapabilityAccessError;
pub use capability_descriptor::CapabilityDescriptor;
pub use capability_key::CapabilityKey;
pub use capability_lookup::CapabilityLookup;
pub use capability_origin::CapabilityOrigin;
pub use clone_adapter::CloneAdapter;
pub use default_adapter::DefaultAdapter;
pub use set::CapabilityConflict;
pub use set::CapabilityConflictKind;
pub use set::TypeCapabilities;
pub use set::TypeCapabilitiesResult;
pub(crate) use set::empty_capabilities;
