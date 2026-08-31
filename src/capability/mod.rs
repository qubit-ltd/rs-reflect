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
#[doc(hidden)]
pub use inventory as __inventory;
pub use key::CapabilityKey;
#[doc(hidden)]
pub use registration::ReflectedTypeRegistration;
#[doc(hidden)]
pub use registration::TypeCapabilityRegistration;
#[doc(hidden)]
pub use registration::registered_reflected_type;
#[doc(hidden)]
pub use registration::registered_type_capabilities;
pub use set::CapabilityConflict;
pub use set::CapabilityConflictKind;
pub use set::TypeCapabilities;
pub(crate) use set::empty_capabilities;
