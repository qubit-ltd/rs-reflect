//! Typed capability contracts and immutable per-target capability sets.

mod builtin;
mod capabilities;
mod capability;
mod capability_error;
mod capability_id;
mod capability_origin;
mod capability_registration;
mod capability_target;

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

#[cfg(test)]
mod tests;
