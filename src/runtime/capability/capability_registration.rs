// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::Any;

use super::builtin::CloneAdapter;
use super::builtin::CloneCapability;
use super::builtin::CoreCapabilitySpec;
use super::builtin::DefaultAdapter;
use super::builtin::DefaultCapability;
use super::builtin::SendCapability;
use super::builtin::SyncCapability;
use super::capability::CapabilitySpec;
use super::capability_error::CapabilityRegistrationError;
use super::capability_id::validate_capability_id;
use super::capability_registration_origin::CapabilityRegistrationOrigin;
use super::capability_target_id::CapabilityTargetId;

/// A typed capability registration with an opaque erased adapter payload.
pub struct CapabilityRegistration {
    pub(crate) target: CapabilityTargetId,
    pub(crate) id: crate::identity::CapabilityId,
    pub(crate) id_str: &'static str,
    pub(crate) adapter: Box<dyn Any + Send + Sync>,
    pub(crate) origin: CapabilityRegistrationOrigin,
    pub(crate) validation_error: Option<CapabilityRegistrationError>,
}

impl CapabilityRegistration {
    /// Creates a typed capability registration for one concrete Rust type.
    pub fn for_type<T: crate::runtime::descriptor::Reflect + ?Sized, C: CapabilitySpec>(adapter: C::Adapter) -> Self {
        Self::typed::<C>(type_target::<T>(), adapter, explicit_origin(std::any::type_name::<T>()))
    }

    /// Creates a typed capability registration for one reflected generic
    /// definition.
    ///
    /// The opaque handle supplies its process-local identity without exposing
    /// the underlying marker type or allowing callers to construct new
    /// identities.
    pub fn for_definition<C: CapabilitySpec>(
        definition: crate::runtime::descriptor::generic::TypeDefinitionId,
        adapter: C::Adapter,
    ) -> Self {
        Self::typed::<C>(
            CapabilityTargetId::Definition(definition),
            adapter,
            explicit_origin("type definition"),
        )
    }

    /// Creates a typed capability registration for a reflected generic
    /// definition identified by its generated marker type.
    pub fn for_type_definition<TMarker: 'static, C: CapabilitySpec>(adapter: C::Adapter) -> Self {
        Self::typed::<C>(
            CapabilityTargetId::Definition(crate::runtime::descriptor::generic::TypeDefinitionId::of::<TMarker>()),
            adapter,
            explicit_origin(std::any::type_name::<TMarker>()),
        )
    }

    fn typed<C: CapabilitySpec>(
        target: CapabilityTargetId,
        adapter: C::Adapter,
        origin: CapabilityRegistrationOrigin,
    ) -> Self {
        match validate_capability_id(C::ID) {
            Ok(id) => Self::with_id::<C>(target, id, adapter, origin)
                .expect("constructing a validated capability registration is infallible"),
            Err(error) => {
                let id = crate::identity::CapabilityId::new("qubit.invalid_registration")
                    .expect("the internal placeholder capability ID is valid");
                Self {
                    target,
                    id,
                    id_str: C::ID,
                    adapter: Box::new(adapter),
                    origin,
                    validation_error: Some(CapabilityRegistrationError::InvalidId(error)),
                }
            }
        }
    }

    pub(crate) fn new<C: CapabilitySpec>(
        target: CapabilityTargetId,
        adapter: C::Adapter,
        origin: CapabilityRegistrationOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        let id = validate_capability_id(C::ID).map_err(CapabilityRegistrationError::InvalidId)?;
        Self::with_id::<C>(target, id, adapter, origin)
    }

    fn new_core<C: CapabilitySpec + CoreCapabilitySpec>(
        target: CapabilityTargetId,
        adapter: C::Adapter,
        origin: CapabilityRegistrationOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        let id = C::core_id().map_err(CapabilityRegistrationError::InvalidId)?;
        Self::with_id::<C>(target, id, adapter, origin)
    }

    fn with_id<C: CapabilitySpec>(
        target: CapabilityTargetId,
        id: crate::identity::CapabilityId,
        adapter: C::Adapter,
        origin: CapabilityRegistrationOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        Ok(Self {
            target,
            id,
            id_str: C::ID,
            adapter: Box::new(adapter),
            origin,
            validation_error: None,
        })
    }

    pub(crate) fn send<T: Send + 'static>(
        origin: CapabilityRegistrationOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        Self::new_core::<SendCapability>(type_target::<T>(), (), origin)
    }

    pub(crate) fn sync<T: Sync + 'static>(
        origin: CapabilityRegistrationOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        Self::new_core::<SyncCapability>(type_target::<T>(), (), origin)
    }

    pub(crate) fn clone<T: Clone + crate::runtime::descriptor::Reflect + 'static>(
        origin: CapabilityRegistrationOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        Self::new_core::<CloneCapability>(type_target::<T>(), CloneAdapter::new::<T>(), origin)
    }

    pub(crate) fn clone_thread_safe<
        T: Clone + crate::runtime::value::ValueFor<crate::runtime::value::ThreadSafe> + Send + Sync,
    >(
        origin: CapabilityRegistrationOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        Self::new_core::<CloneCapability>(type_target::<T>(), CloneAdapter::new_thread_safe::<T>(), origin)
    }

    pub(crate) fn default<T: Default + crate::runtime::descriptor::Reflect + 'static>(
        origin: CapabilityRegistrationOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        Self::new_core::<DefaultCapability>(type_target::<T>(), DefaultAdapter::new::<T>(), origin)
    }

    pub(crate) fn default_thread_safe<
        T: Default + crate::runtime::value::ValueFor<crate::runtime::value::ThreadSafe> + Send + Sync,
    >(
        origin: CapabilityRegistrationOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        Self::new_core::<DefaultCapability>(type_target::<T>(), DefaultAdapter::new_thread_safe::<T>(), origin)
    }
}

fn type_target<T: 'static + ?Sized>() -> CapabilityTargetId {
    CapabilityTargetId::Type(crate::runtime::identity::TypeIdentity {
        type_id: std::any::TypeId::of::<T>(),
        rust_name: std::any::type_name::<T>(),
    })
}

fn explicit_origin(module_path: &'static str) -> CapabilityRegistrationOrigin {
    CapabilityRegistrationOrigin::Explicit(crate::runtime::identity::RegistrationOrigin {
        crate_name: "explicit",
        crate_version: None,
        module_path,
        file: None,
        line: None,
        column: None,
    })
}
