//! Typed capability contracts and immutable per-target capability sets.

pub(crate) mod builtin;
pub(crate) mod capabilities;
pub(crate) mod capability;
pub(crate) mod capability_error;
pub(crate) mod capability_id;
pub(crate) mod capability_origin;
pub(crate) mod capability_registration;
pub(crate) mod capability_target;

pub(crate) use builtin::{
    CloneAdapter, CloneCapability, DefaultAdapter, DefaultCapability, SendCapability,
    SyncCapability,
};
pub(crate) use capabilities::{Capabilities, CapabilityRecord};
pub(crate) use capability::{Capability, CapabilityDescriptor, CapabilitySpec};
pub(crate) use capability_error::{
    CapabilityContractError, CapabilityError, CapabilityInvocationError, CapabilityInvocationMode,
    CapabilityRegistrationError,
};
pub(crate) use capability_id::CapabilityIdentifier as CapabilityId;
pub(crate) use capability_origin::CapabilityOrigin;
pub(crate) use capability_registration::CapabilityRegistration;
pub(crate) use capability_target::CapabilityTargetId;
