//! Typed capability contracts and immutable per-target capability sets.

pub(crate) mod builtin;
pub(crate) mod capabilities;
pub(crate) mod capability;
pub(crate) mod capability_error;
pub(crate) mod capability_id;
pub(crate) mod capability_origin;
pub(crate) mod capability_registration;
pub(crate) mod capability_target;
mod type_descriptor_ext;

pub use builtin::{
    CloneAdapter, CloneCapability, DefaultAdapter, DefaultCapability, SendCapability,
    SyncCapability, CapabilityValueMode,
};
pub use capabilities::Capabilities;
pub(crate) use capabilities::CapabilityRecord;
pub use capability::{Capability, CapabilityDescriptor, CapabilitySpec};
pub use capability_error::{
    CapabilityContractError, CapabilityError, CapabilityInvocationError, CapabilityInvocationMode,
};
pub(crate) use capability_error::CapabilityRegistrationError;
pub(crate) use capability_id::CapabilityIdentifier as CapabilityId;
pub use capability_origin::CapabilityOrigin;
pub use capability_registration::CapabilityRegistration;
pub use capability_target::CapabilityTargetId;
pub use type_descriptor_ext::{TypeDefinitionCapabilityExt, TypeDescriptorCapabilityExt};
