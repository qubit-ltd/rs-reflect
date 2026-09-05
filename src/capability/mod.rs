// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed, immutable capabilities attached to reflected concrete types.

mod builtin;
mod descriptor;
mod key;
mod registration;
mod set;

pub use builtin::CloneAdapter;
pub use builtin::DefaultAdapter;
pub use builtin::clone_descriptor;
pub use builtin::clone_key;
pub use builtin::default_descriptor;
pub use builtin::default_key;
pub use builtin::send_descriptor;
pub use builtin::send_key;
pub use builtin::sync_descriptor;
pub use builtin::sync_key;
pub use descriptor::CapabilityDescriptor;
pub use key::CapabilityKey;
pub use set::CapabilityConflict;
pub use set::CapabilityConflictKind;
pub use set::TypeCapabilities;
/// A lazily initialized capability set or its structural conflict.
pub type TypeCapabilitiesResult = Result<&'static TypeCapabilities, CapabilityConflict>;
pub(crate) use set::empty_capabilities;
