use std::any::TypeId;

use super::capability::CapabilitySpec;
use super::capability_error::{CapabilityInvocationError, CapabilityInvocationMode};
use super::capability_id::CapabilityIdentifier;
use crate::runtime::value::{DynamicOwned, DynamicRef, Local, ThreadSafe, ValueFor, ValueMode};

pub(super) trait CoreCapabilitySpec: CapabilitySpec {
    fn core_id() -> Result<CapabilityIdentifier, crate::error::IdError> {
        CapabilityIdentifier::new_core(Self::ID)
    }
}

pub(super) trait CapabilityValueMode: ValueMode + Sized {
    fn invocation_mode() -> CapabilityInvocationMode;

    fn clone_dynamic(
        adapter: &CloneAdapter,
        value: DynamicRef<'_, Self>,
    ) -> Result<DynamicOwned<Self>, CapabilityInvocationError>;

    fn create_dynamic(
        adapter: &DefaultAdapter,
    ) -> Result<DynamicOwned<Self>, CapabilityInvocationError>;
}

impl CapabilityValueMode for Local {
    fn invocation_mode() -> CapabilityInvocationMode {
        CapabilityInvocationMode::Local
    }

    fn clone_dynamic(
        adapter: &CloneAdapter,
        value: DynamicRef<'_, Self>,
    ) -> Result<DynamicOwned<Self>, CapabilityInvocationError> {
        (adapter.clone_local)(value)
    }

    fn create_dynamic(
        adapter: &DefaultAdapter,
    ) -> Result<DynamicOwned<Self>, CapabilityInvocationError> {
        Ok((adapter.create_local)())
    }
}

impl CapabilityValueMode for ThreadSafe {
    fn invocation_mode() -> CapabilityInvocationMode {
        CapabilityInvocationMode::ThreadSafe
    }

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

    fn create_dynamic(
        adapter: &DefaultAdapter,
    ) -> Result<DynamicOwned<Self>, CapabilityInvocationError> {
        let Some(create) = adapter.create_thread_safe else {
            return Err(CapabilityInvocationError::UnsupportedMode {
                expected_type: adapter.type_id,
                mode: Self::invocation_mode(),
            });
        };
        Ok(create())
    }
}

pub struct SendCapability;
pub struct SyncCapability;
pub struct CloneCapability;
pub struct DefaultCapability;

impl CapabilitySpec for SendCapability {
    type Adapter = ();
    const ID: &'static str = "qubit.reflect.send";
}

impl CoreCapabilitySpec for SendCapability {}

impl CapabilitySpec for SyncCapability {
    type Adapter = ();
    const ID: &'static str = "qubit.reflect.sync";
}

impl CoreCapabilitySpec for SyncCapability {}

impl CapabilitySpec for CloneCapability {
    type Adapter = CloneAdapter;
    const ID: &'static str = "qubit.reflect.clone";
}

impl CoreCapabilitySpec for CloneCapability {}

impl CapabilitySpec for DefaultCapability {
    type Adapter = DefaultAdapter;
    const ID: &'static str = "qubit.reflect.default";
}

impl CoreCapabilitySpec for DefaultCapability {}

/// Clone operations for values whose concrete type is proven at construction.
#[derive(Clone, Copy)]
pub(crate) struct CloneAdapter {
    type_id: TypeId,
    clone_local:
        for<'a> fn(DynamicRef<'a, Local>) -> Result<DynamicOwned<Local>, CapabilityInvocationError>,
    clone_thread_safe: Option<
        for<'a> fn(
            DynamicRef<'a, ThreadSafe>,
        ) -> Result<DynamicOwned<ThreadSafe>, CapabilityInvocationError>,
    >,
}

impl CloneAdapter {
    pub(crate) fn new<T: Clone + ValueFor<Local>>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            clone_local: clone_value::<T>,
            clone_thread_safe: None,
        }
    }

    pub(crate) fn new_thread_safe<T: Clone + ValueFor<ThreadSafe> + Send + Sync>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            clone_local: clone_value::<T>,
            clone_thread_safe: Some(clone_thread_safe_value::<T>),
        }
    }

    pub(crate) fn type_id(self) -> TypeId {
        self.type_id
    }

    pub(crate) fn clone_dynamic<M: CapabilityValueMode>(
        &self,
        value: DynamicRef<'_, M>,
    ) -> Result<DynamicOwned<M>, CapabilityInvocationError> {
        M::clone_dynamic(self, value)
    }
}

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
pub(crate) struct DefaultAdapter {
    type_id: TypeId,
    create_local: fn() -> DynamicOwned<Local>,
    create_thread_safe: Option<fn() -> DynamicOwned<ThreadSafe>>,
}

impl DefaultAdapter {
    pub(crate) fn new<T: Default + ValueFor<Local>>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            create_local: create_default::<T>,
            create_thread_safe: None,
        }
    }

    pub(crate) fn new_thread_safe<T: Default + ValueFor<ThreadSafe> + Send + Sync>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            create_local: create_default::<T>,
            create_thread_safe: Some(create_thread_safe_default::<T>),
        }
    }

    pub(crate) fn type_id(self) -> TypeId {
        self.type_id
    }

    pub(crate) fn create_dynamic<M: CapabilityValueMode>(
        &self,
    ) -> Result<DynamicOwned<M>, CapabilityInvocationError> {
        M::create_dynamic(self)
    }
}

fn create_default<T: Default + ValueFor<Local>>() -> DynamicOwned<Local> {
    DynamicOwned::new(T::default())
}

fn create_thread_safe_default<T: Default + ValueFor<ThreadSafe> + Send + Sync>()
-> DynamicOwned<ThreadSafe> {
    DynamicOwned::new(T::default())
}
