//! Typed, immutable capabilities attached to reflected concrete types.

mod builtin;
mod descriptor;
mod key;
mod registration;
mod set;

pub use builtin::{
    CloneAdapter, DefaultAdapter, clone_descriptor, clone_key, default_descriptor, default_key,
    send_descriptor, send_key, sync_descriptor, sync_key,
};
pub use descriptor::CapabilityDescriptor;
pub use key::CapabilityKey;
#[doc(hidden)]
pub use registration::{
    ReflectedTypeRegistration, TypeCapabilityRegistration, registered_reflected_type,
    registered_type_capabilities,
};
pub(crate) use set::empty_capabilities;
pub use set::{CapabilityConflict, CapabilityConflictKind, TypeCapabilities};

#[doc(hidden)]
pub use inventory as __inventory;
