use std::any::Any;

use super::builtin::{
    CloneAdapter, CloneCapability, CoreCapabilitySpec, DefaultAdapter, DefaultCapability,
    SendCapability, SyncCapability,
};
use super::capability::CapabilitySpec;
use super::capability_error::CapabilityRegistrationError;
use super::capability_id::validate_capability_id;
use super::capability_origin::CapabilityOrigin;
use super::capability_target::CapabilityTargetId;

/// A validated registration carrying an adapter under its declared contract.
pub(crate) struct CapabilityRegistration {
    pub(crate) target: CapabilityTargetId,
    pub(crate) id: crate::identity::CapabilityId,
    pub(crate) adapter: Box<dyn Any + Send + Sync>,
    pub(crate) origin: CapabilityOrigin,
}

impl CapabilityRegistration {
    pub(crate) fn new<C: CapabilitySpec>(
        target: CapabilityTargetId,
        adapter: C::Adapter,
        origin: CapabilityOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        let id = validate_capability_id(C::ID).map_err(CapabilityRegistrationError::InvalidId)?;
        Self::with_id::<C>(target, id, adapter, origin)
    }

    fn new_core<C: CapabilitySpec + CoreCapabilitySpec>(
        target: CapabilityTargetId,
        adapter: C::Adapter,
        origin: CapabilityOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        let id = C::core_id().map_err(CapabilityRegistrationError::InvalidId)?;
        Self::with_id::<C>(target, id, adapter, origin)
    }

    fn with_id<C: CapabilitySpec>(
        target: CapabilityTargetId,
        id: crate::identity::CapabilityId,
        adapter: C::Adapter,
        origin: CapabilityOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        Ok(Self {
            target,
            id,
            adapter: Box::new(adapter),
            origin,
        })
    }

    pub(crate) fn send<T: Send + 'static>(
        origin: CapabilityOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        Self::new_core::<SendCapability>(type_target::<T>(), (), origin)
    }

    pub(crate) fn sync<T: Sync + 'static>(
        origin: CapabilityOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        Self::new_core::<SyncCapability>(type_target::<T>(), (), origin)
    }

    pub(crate) fn clone<T: Clone + crate::runtime::descriptor::Reflect + 'static>(
        origin: CapabilityOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        Self::new_core::<CloneCapability>(type_target::<T>(), CloneAdapter::new::<T>(), origin)
    }

    pub(crate) fn clone_thread_safe<
        T: Clone + crate::runtime::value::ValueFor<crate::runtime::value::ThreadSafe> + Send + Sync,
    >(
        origin: CapabilityOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        Self::new_core::<CloneCapability>(
            type_target::<T>(),
            CloneAdapter::new_thread_safe::<T>(),
            origin,
        )
    }

    pub(crate) fn default<T: Default + crate::runtime::descriptor::Reflect + 'static>(
        origin: CapabilityOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        Self::new_core::<DefaultCapability>(type_target::<T>(), DefaultAdapter::new::<T>(), origin)
    }

    pub(crate) fn default_thread_safe<
        T: Default + crate::runtime::value::ValueFor<crate::runtime::value::ThreadSafe> + Send + Sync,
    >(
        origin: CapabilityOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        Self::new_core::<DefaultCapability>(
            type_target::<T>(),
            DefaultAdapter::new_thread_safe::<T>(),
            origin,
        )
    }
}

fn type_target<T: 'static>() -> CapabilityTargetId {
    CapabilityTargetId::Type(crate::runtime::identity::TypeIdentity {
        type_id: std::any::TypeId::of::<T>(),
        rust_name: std::any::type_name::<T>(),
    })
}
