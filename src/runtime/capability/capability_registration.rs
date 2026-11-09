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
    /// Target identity where this adapter should be registered.
    pub(crate) target: CapabilityTargetId,
    /// Opaque typed capability ID.
    pub(crate) id: crate::identity::CapabilityId,
    /// Static text form of the capability's declared ID.
    pub(crate) id_str: &'static str,
    /// Type-erased adapter payload retained by the capability set.
    pub(crate) adapter: Box<dyn Any + Send + Sync>,
    /// Origin metadata used in registration diagnostics.
    pub(crate) origin: CapabilityRegistrationOrigin,
    /// Deferred validation error for registrations built through infallible
    /// APIs.
    pub(crate) validation_error: Option<CapabilityRegistrationError>,
}

impl CapabilityRegistration {
    /// Creates a typed capability registration for one concrete Rust type.
    ///
    /// # Type Parameters
    /// - `T`: concrete reflected target type.
    /// - `C`: capability contract implemented by the adapter.
    ///
    /// # Parameters
    /// - `adapter`: implementation of the declared capability contract.
    pub fn for_type<T: crate::runtime::descriptor::Reflect + ?Sized, C: CapabilitySpec>(adapter: C::Adapter) -> Self {
        Self::typed::<C>(type_target::<T>(), adapter, explicit_origin(std::any::type_name::<T>()))
    }

    /// Creates a typed capability registration for one reflected generic
    /// definition.
    ///
    /// The opaque handle supplies its process-local identity without exposing
    /// the underlying marker type or allowing callers to construct new
    /// identities.
    ///
    /// # Type Parameters
    /// - `C`: capability contract implemented by the adapter.
    ///
    /// # Parameters
    /// - `definition`: reflected generic type definition to target.
    /// - `adapter`: implementation of the declared capability contract.
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
    ///
    /// # Type Parameters
    /// - `TMarker`: generated marker identifying the generic definition.
    /// - `C`: capability contract implemented by the adapter.
    ///
    /// # Parameters
    /// - `adapter`: implementation of the declared capability contract.
    pub fn for_type_definition<TMarker: 'static, C: CapabilitySpec>(adapter: C::Adapter) -> Self {
        Self::typed::<C>(
            CapabilityTargetId::Definition(crate::runtime::descriptor::generic::TypeDefinitionId::of::<TMarker>()),
            adapter,
            explicit_origin(std::any::type_name::<TMarker>()),
        )
    }

    /// Builds a registration after validating the contract's stable ID.
    ///
    /// # Type Parameters
    /// - `C`: capability contract implemented by the adapter.
    ///
    /// # Parameters
    /// - `target`: reflected target receiving the capability.
    /// - `adapter`: implementation of the declared contract.
    /// - `origin`: registration provenance for diagnostics.
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

    /// Creates a registration and returns any invalid ID as an error.
    ///
    /// # Type Parameters
    /// - `C`: capability contract implemented by the adapter.
    ///
    /// # Parameters
    /// - `target`: reflected target receiving the capability.
    /// - `adapter`: implementation of the declared contract.
    /// - `origin`: registration provenance for diagnostics.
    pub(crate) fn new<C: CapabilitySpec>(
        target: CapabilityTargetId,
        adapter: C::Adapter,
        origin: CapabilityRegistrationOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        let id = validate_capability_id(C::ID).map_err(CapabilityRegistrationError::InvalidId)?;
        Self::with_id::<C>(target, id, adapter, origin)
    }

    /// Creates a registration using an identifier reserved for a core contract.
    ///
    /// # Type Parameters
    /// - `C`: core capability contract implemented by the adapter.
    ///
    /// # Parameters
    /// - `target`: reflected target receiving the capability.
    /// - `adapter`: implementation of the declared contract.
    /// - `origin`: registration provenance for diagnostics.
    fn new_core<C: CapabilitySpec + CoreCapabilitySpec>(
        target: CapabilityTargetId,
        adapter: C::Adapter,
        origin: CapabilityRegistrationOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        let id = C::core_id().map_err(CapabilityRegistrationError::InvalidId)?;
        Self::with_id::<C>(target, id, adapter, origin)
    }

    /// Packages a validated identifier and typed adapter into a registration.
    ///
    /// # Type Parameters
    /// - `C`: capability contract implemented by the adapter.
    ///
    /// # Parameters
    /// - `target`: reflected target receiving the capability.
    /// - `id`: validated typed identifier.
    /// - `adapter`: implementation of the declared contract.
    /// - `origin`: registration provenance for diagnostics.
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

    /// Creates the built-in `Send` capability registration for `T`.
    ///
    /// # Type Parameters
    /// - `T`: target type that must implement `Send`.
    ///
    /// # Parameters
    /// - `origin`: registration provenance for diagnostics.
    pub(crate) fn send<T: Send + 'static>(
        origin: CapabilityRegistrationOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        Self::new_core::<SendCapability>(type_target::<T>(), (), origin)
    }

    /// Creates the built-in `Sync` capability registration for `T`.
    ///
    /// # Type Parameters
    /// - `T`: target type that must implement `Sync`.
    ///
    /// # Parameters
    /// - `origin`: registration provenance for diagnostics.
    pub(crate) fn sync<T: Sync + 'static>(
        origin: CapabilityRegistrationOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        Self::new_core::<SyncCapability>(type_target::<T>(), (), origin)
    }

    /// Creates a local clone capability registration for `T`.
    ///
    /// # Type Parameters
    /// - `T`: reflected target type with a local clone adapter.
    ///
    /// # Parameters
    /// - `origin`: registration provenance for diagnostics.
    pub(crate) fn clone<T: Clone + crate::runtime::descriptor::Reflect + 'static>(
        origin: CapabilityRegistrationOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        Self::new_core::<CloneCapability>(type_target::<T>(), CloneAdapter::new::<T>(), origin)
    }

    /// Creates a thread-safe clone capability registration for `T`.
    ///
    /// # Type Parameters
    /// - `T`: target type with a thread-safe clone adapter.
    ///
    /// # Parameters
    /// - `origin`: registration provenance for diagnostics.
    pub(crate) fn clone_thread_safe<
        T: Clone + crate::runtime::value::ValueFor<crate::runtime::value::ThreadSafe> + Send + Sync,
    >(
        origin: CapabilityRegistrationOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        Self::new_core::<CloneCapability>(type_target::<T>(), CloneAdapter::new_thread_safe::<T>(), origin)
    }

    /// Creates a local default capability registration for `T`.
    ///
    /// # Type Parameters
    /// - `T`: reflected target type with a local default adapter.
    ///
    /// # Parameters
    /// - `origin`: registration provenance for diagnostics.
    pub(crate) fn default<T: Default + crate::runtime::descriptor::Reflect + 'static>(
        origin: CapabilityRegistrationOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        Self::new_core::<DefaultCapability>(type_target::<T>(), DefaultAdapter::new::<T>(), origin)
    }

    /// Creates a thread-safe default capability registration for `T`.
    ///
    /// # Type Parameters
    /// - `T`: target type with a thread-safe default adapter.
    ///
    /// # Parameters
    /// - `origin`: registration provenance for diagnostics.
    pub(crate) fn default_thread_safe<
        T: Default + crate::runtime::value::ValueFor<crate::runtime::value::ThreadSafe> + Send + Sync,
    >(
        origin: CapabilityRegistrationOrigin,
    ) -> Result<Self, CapabilityRegistrationError> {
        Self::new_core::<DefaultCapability>(type_target::<T>(), DefaultAdapter::new_thread_safe::<T>(), origin)
    }
}

/// Creates the capability target identity for a concrete Rust type.
///
/// # Type Parameters
/// - `T`: Rust type represented by the target identity.
fn type_target<T: 'static + ?Sized>() -> CapabilityTargetId {
    CapabilityTargetId::Type(crate::runtime::identity::TypeIdentity {
        type_id: std::any::TypeId::of::<T>(),
        rust_name: std::any::type_name::<T>(),
    })
}

/// Creates provenance metadata for a direct registration call.
///
/// # Parameters
/// - `module_path`: path or descriptive label identifying the caller.
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
