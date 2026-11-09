// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;

use super::capability::CapabilitySpec;
use super::capability_error::CapabilityInvocationError;
use super::capability_error::CapabilityInvocationMode;
use super::capability_id::CapabilityIdentifier;
use crate::runtime::value::DynamicOwned;
use crate::runtime::value::DynamicRef;
use crate::runtime::value::Local;
use crate::runtime::value::ThreadSafe;
use crate::runtime::value::ValueFor;
use crate::runtime::value::ValueMode;

/// Function used to clone values stored in thread-safe dynamic containers.
type ThreadSafeCloneFn =
    for<'a> fn(DynamicRef<'a, ThreadSafe>) -> Result<DynamicOwned<ThreadSafe>, CapabilityInvocationError>;

/// Marks a capability contract whose identifier is reserved by this crate.
pub(super) trait CoreCapabilitySpec: CapabilitySpec {
    /// Returns the validated core identifier reserved for this capability.
    fn core_id() -> Result<CapabilityIdentifier, crate::error::IdError> {
        CapabilityIdentifier::new_core(Self::ID)
    }
}

/// Connects a dynamic value mode to the corresponding built-in capability
/// operations.
pub trait CapabilityValueMode: ValueMode + Sized {
    /// Returns the invocation mode corresponding to this value mode.
    fn invocation_mode() -> CapabilityInvocationMode;

    /// Clones a dynamic value through the supplied adapter.
    ///
    /// # Parameters
    /// - `adapter`: generated clone functions for the concrete value type.
    /// - `value`: dynamic value to clone.
    fn clone_dynamic(
        adapter: &CloneAdapter,
        value: DynamicRef<'_, Self>,
    ) -> Result<DynamicOwned<Self>, CapabilityInvocationError>;

    /// Creates a dynamic value through the supplied default adapter.
    ///
    /// # Parameters
    /// - `adapter`: generated default constructors for the concrete type.
    fn create_dynamic(adapter: &DefaultAdapter) -> Result<DynamicOwned<Self>, CapabilityInvocationError>;
}

impl CapabilityValueMode for Local {
    /// Returns the local adapter invocation mode.
    fn invocation_mode() -> CapabilityInvocationMode {
        CapabilityInvocationMode::Local
    }

    /// Clones the local value through the generated adapter.
    ///
    /// # Parameters
    /// - `adapter`: generated local clone function.
    /// - `value`: local dynamic value to clone.
    fn clone_dynamic(
        adapter: &CloneAdapter,
        value: DynamicRef<'_, Self>,
    ) -> Result<DynamicOwned<Self>, CapabilityInvocationError> {
        (adapter.clone_local)(value)
    }

    /// Calls the local default constructor stored in the adapter.
    ///
    /// # Parameters
    /// - `adapter`: generated local default constructor.
    fn create_dynamic(adapter: &DefaultAdapter) -> Result<DynamicOwned<Self>, CapabilityInvocationError> {
        Ok((adapter.create_local)())
    }
}

impl CapabilityValueMode for ThreadSafe {
    /// Returns the thread-safe adapter invocation mode.
    fn invocation_mode() -> CapabilityInvocationMode {
        CapabilityInvocationMode::ThreadSafe
    }

    /// Calls the thread-safe clone function when one is available.
    ///
    /// # Parameters
    /// - `adapter`: generated thread-safe clone function, if available.
    /// - `value`: thread-safe dynamic value to clone.
    fn clone_dynamic(
        adapter: &CloneAdapter,
        value: DynamicRef<'_, Self>,
    ) -> Result<DynamicOwned<Self>, CapabilityInvocationError> {
        let Some(clone) = adapter.clone_thread_safe else {
            return Err(CapabilityInvocationError::UnsupportedMode {
                expected_type: adapter.type_id,
                mode: Self::invocation_mode(),
            });
        };
        clone(value)
    }

    /// Calls the thread-safe default constructor when one is available.
    ///
    /// # Parameters
    /// - `adapter`: generated thread-safe default constructor, if available.
    fn create_dynamic(adapter: &DefaultAdapter) -> Result<DynamicOwned<Self>, CapabilityInvocationError> {
        let Some(create) = adapter.create_thread_safe else {
            return Err(CapabilityInvocationError::UnsupportedMode {
                expected_type: adapter.type_id,
                mode: Self::invocation_mode(),
            });
        };
        Ok(create())
    }
}

/// Marker specification for the core `Send` capability.
#[derive(Clone, Copy, Debug)]
pub struct SendCapability;
/// Marker specification for the core `Sync` capability.
#[derive(Clone, Copy, Debug)]
pub struct SyncCapability;
/// Marker specification for the core `Clone` capability.
#[derive(Clone, Copy, Debug)]
pub struct CloneCapability;
/// Marker specification for the core `Default` capability.
#[derive(Clone, Copy, Debug)]
pub struct DefaultCapability;

impl CapabilitySpec for SendCapability {
    /// Unit adapter because the built-in marker has no runtime state.
    type Adapter = ();
    /// Reserved ID for the core send capability.
    const ID: &'static str = "qubit.reflect.send";
}

impl CoreCapabilitySpec for SendCapability {}

impl CapabilitySpec for SyncCapability {
    /// Unit adapter because the built-in marker has no runtime state.
    type Adapter = ();
    /// Reserved ID for the core sync capability.
    const ID: &'static str = "qubit.reflect.sync";
}

impl CoreCapabilitySpec for SyncCapability {}

impl CapabilitySpec for CloneCapability {
    /// Generated functions that clone supported dynamic values.
    type Adapter = CloneAdapter;
    /// Reserved ID for the core clone capability.
    const ID: &'static str = "qubit.reflect.clone";
}

impl CoreCapabilitySpec for CloneCapability {}

impl CapabilitySpec for DefaultCapability {
    /// Generated functions that construct supported dynamic values.
    type Adapter = DefaultAdapter;
    /// Reserved ID for the core default capability.
    const ID: &'static str = "qubit.reflect.default";
}

impl CoreCapabilitySpec for DefaultCapability {}

/// Clone operations for values whose concrete type is proven at construction.
#[derive(Clone, Copy)]
pub struct CloneAdapter {
    /// Concrete Rust type accepted by the clone functions.
    type_id: TypeId,
    /// Clone function for local dynamic values.
    clone_local: for<'a> fn(DynamicRef<'a, Local>) -> Result<DynamicOwned<Local>, CapabilityInvocationError>,
    /// Optional clone function for thread-safe dynamic values.
    clone_thread_safe: Option<ThreadSafeCloneFn>,
}

impl CloneAdapter {
    /// Creates a local clone adapter for `T`.
    ///
    /// # Type Parameters
    /// - `T`: concrete cloneable type stored in dynamic values.
    pub(crate) fn new<T: Clone + ValueFor<Local>>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            clone_local: clone_value::<T>,
            clone_thread_safe: None,
        }
    }

    /// Creates a clone adapter that supports both value modes for `T`.
    ///
    /// # Type Parameters
    /// - `T`: concrete cloneable, thread-safe type stored in dynamic values.
    pub(crate) fn new_thread_safe<T: Clone + ValueFor<ThreadSafe> + Send + Sync>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            clone_local: clone_value::<T>,
            clone_thread_safe: Some(clone_thread_safe_value::<T>),
        }
    }

    /// Clones a value when its concrete type and mode are supported.
    ///
    /// Returns a type mismatch if `value` is not the registered type, or an
    /// unsupported-mode error if no adapter was generated for `M`.
    ///
    /// # Type Parameters
    /// - `M`: value mode of the input and returned dynamic value.
    ///
    /// # Parameters
    /// - `value`: dynamic value to clone.
    pub fn clone_dynamic<M: CapabilityValueMode>(
        &self,
        value: DynamicRef<'_, M>,
    ) -> Result<DynamicOwned<M>, CapabilityInvocationError> {
        M::clone_dynamic(self, value)
    }
}

/// Clones a local dynamic value after checking its concrete Rust type.
///
/// # Type Parameters
/// - `T`: expected concrete value type.
///
/// # Parameters
/// - `value`: dynamic value to validate and clone.
fn clone_value<T: Clone + ValueFor<Local>>(
    value: DynamicRef<'_, Local>,
) -> Result<DynamicOwned<Local>, CapabilityInvocationError> {
    if let Some(value) = value.downcast_ref::<T>() {
        return Ok(DynamicOwned::new(value.clone()));
    }
    Err(CapabilityInvocationError::TypeMismatch {
        expected: TypeId::of::<T>(),
        actual: value.type_id(),
    })
}

/// Clones a thread-safe dynamic value after checking its concrete Rust type.
///
/// # Type Parameters
/// - `T`: expected concrete, thread-safe value type.
///
/// # Parameters
/// - `value`: dynamic value to validate and clone.
fn clone_thread_safe_value<T: Clone + ValueFor<ThreadSafe> + Send + Sync>(
    value: DynamicRef<'_, ThreadSafe>,
) -> Result<DynamicOwned<ThreadSafe>, CapabilityInvocationError> {
    if let Some(value) = value.downcast_ref::<T>() {
        return Ok(DynamicOwned::new(value.clone()));
    }
    Err(CapabilityInvocationError::TypeMismatch {
        expected: TypeId::of::<T>(),
        actual: value.type_id(),
    })
}

/// Default construction for values whose concrete type is proven at creation.
#[derive(Clone, Copy)]
pub struct DefaultAdapter {
    /// Concrete Rust type accepted by the default constructors.
    type_id: TypeId,
    /// Constructor for local dynamic values.
    create_local: fn() -> DynamicOwned<Local>,
    /// Optional constructor for thread-safe dynamic values.
    create_thread_safe: Option<fn() -> DynamicOwned<ThreadSafe>>,
}

impl DefaultAdapter {
    /// Creates a local default adapter for `T`.
    ///
    /// # Type Parameters
    /// - `T`: concrete type implementing `Default`.
    pub(crate) fn new<T: Default + ValueFor<Local>>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            create_local: create_default::<T>,
            create_thread_safe: None,
        }
    }

    /// Creates a default adapter that supports both value modes for `T`.
    ///
    /// # Type Parameters
    /// - `T`: concrete, thread-safe type implementing `Default`.
    pub(crate) fn new_thread_safe<T: Default + ValueFor<ThreadSafe> + Send + Sync>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            create_local: create_default::<T>,
            create_thread_safe: Some(create_thread_safe_default::<T>),
        }
    }

    /// Constructs a value when the adapter supports the requested mode.
    ///
    /// Returns an unsupported-mode error when no constructor was generated for
    /// `M`.
    ///
    /// # Type Parameters
    /// - `M`: value mode of the returned dynamic value.
    pub fn create_dynamic<M: CapabilityValueMode>(&self) -> Result<DynamicOwned<M>, CapabilityInvocationError> {
        M::create_dynamic(self)
    }
}

/// Constructs a local dynamic value by calling its type's default constructor.
///
/// # Type Parameters
/// - `T`: concrete local value type implementing `Default`.
fn create_default<T: Default + ValueFor<Local>>() -> DynamicOwned<Local> {
    DynamicOwned::new(T::default())
}

/// Constructs a thread-safe dynamic value using its default constructor.
///
/// # Type Parameters
/// - `T`: concrete thread-safe value type implementing `Default`.
fn create_thread_safe_default<T: Default + ValueFor<ThreadSafe> + Send + Sync>() -> DynamicOwned<ThreadSafe> {
    DynamicOwned::new(T::default())
}
