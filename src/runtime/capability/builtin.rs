use std::any::TypeId;

use super::capability::CapabilitySpec;
use super::capability_id::CapabilityIdentifier;
use crate::runtime::value::{DynamicOwned, DynamicRef, Local, ThreadSafe, ValueFor};

pub(super) trait CoreCapabilitySpec: CapabilitySpec {
    fn core_id() -> Result<CapabilityIdentifier, crate::error::IdError> {
        CapabilityIdentifier::new_core(Self::ID)
    }
}

pub(crate) struct SendCapability;
pub(crate) struct SyncCapability;
pub(crate) struct CloneCapability;
pub(crate) struct DefaultCapability;

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
    clone_local: for<'a> fn(DynamicRef<'a, Local>) -> Option<DynamicOwned<Local>>,
    clone_thread_safe:
        Option<for<'a> fn(DynamicRef<'a, ThreadSafe>) -> Option<DynamicOwned<ThreadSafe>>>,
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

    pub(crate) fn clone_local(self, value: DynamicRef<'_, Local>) -> Option<DynamicOwned<Local>> {
        (self.clone_local)(value)
    }

    pub(crate) fn clone_thread_safe(
        self,
        value: DynamicRef<'_, ThreadSafe>,
    ) -> Option<DynamicOwned<ThreadSafe>> {
        (self.clone_thread_safe?)(value)
    }
}

fn clone_value<T: Clone + ValueFor<Local>>(
    value: DynamicRef<'_, Local>,
) -> Option<DynamicOwned<Local>> {
    value.downcast_ref::<T>().cloned().map(DynamicOwned::new)
}

fn clone_thread_safe_value<T: Clone + ValueFor<ThreadSafe> + Send + Sync>(
    value: DynamicRef<'_, ThreadSafe>,
) -> Option<DynamicOwned<ThreadSafe>> {
    value.downcast_ref::<T>().cloned().map(DynamicOwned::new)
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

    pub(crate) fn create_local(self) -> DynamicOwned<Local> {
        (self.create_local)()
    }

    pub(crate) fn create_thread_safe(self) -> Option<DynamicOwned<ThreadSafe>> {
        self.create_thread_safe.map(|create| create())
    }
}

fn create_default<T: Default + ValueFor<Local>>() -> DynamicOwned<Local> {
    DynamicOwned::new(T::default())
}

fn create_thread_safe_default<T: Default + ValueFor<ThreadSafe> + Send + Sync>()
-> DynamicOwned<ThreadSafe> {
    DynamicOwned::new(T::default())
}
