// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed capability contracts and immutable per-target capability sets.

// These runtime descriptors are instantiated by generated code in downstream
// crates, which this crate's own dead-code analysis cannot observe.
#![allow(dead_code)]
// Capability errors keep the complete typed conflict context inline so callers
// can inspect it without allocating or unpacking opaque boxed payloads.
#![allow(clippy::result_large_err)]

pub(crate) mod builtin;
pub(crate) mod capabilities;
pub(crate) mod capabilities_view;
#[allow(clippy::module_inception)]
pub(crate) mod capability;
pub(crate) mod capability_error;
pub(crate) mod capability_id;
pub(crate) mod capability_origin;
pub(crate) mod capability_registration;
pub(crate) mod capability_registration_origin;
pub(crate) mod capability_target;
pub(crate) mod capability_target_id;
pub(crate) mod context_registration_origin;

pub use builtin::CapabilityValueMode;
pub use builtin::CloneAdapter;
pub use builtin::CloneCapability;
pub use builtin::DefaultAdapter;
pub use builtin::DefaultCapability;
pub use builtin::SendCapability;
pub use builtin::SyncCapability;
pub use capabilities_view::CapabilitiesView as Capabilities;
pub use capability::Capability;
pub use capability::CapabilityDescriptor;
pub use capability::CapabilitySpec;
pub use capability_error::CapabilityContractError;
pub use capability_error::CapabilityError;
pub use capability_error::CapabilityInvocationError;
pub use capability_origin::CapabilityOrigin;
pub use capability_registration::CapabilityRegistration;
pub use capability_registration_origin::CapabilityRegistrationOrigin;
pub use capability_target::CapabilityTarget;
pub use capability_target_id::CapabilityTargetId;
pub use context_registration_origin::ContextRegistrationOrigin;
