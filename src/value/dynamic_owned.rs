//! Owned dynamic values with mode-specific erased storage.

use std::any::Any;
use std::marker::PhantomData;

use crate::value::Local;
use crate::value::ThreadSafe;
use crate::value::mode::Mode;
use crate::value::storage::LocalOwnedStorage;
use crate::value::storage::ThreadSafeOwnedStorage;

/// An owned dynamic value whose erased boundary is selected by `M`.
///
/// [`Local`] retains an ordinary local `Any` value and intentionally does not
/// implement `Send` or `Sync`. [`ThreadSafe`] only accepts `Send + Sync`
/// values and keeps that boundary in its erased storage, so the wrapper can
/// implement both auto traits. Owned values require `'static` and do not carry
/// a borrow lifetime.
///
/// This wrapper only accepts sized `Any`-compatible values. Borrowed `str` is
/// represented exclusively by [`DynamicRef`](crate::value::DynamicRef) and
/// [`DynamicMut`](crate::value::DynamicMut)'s dedicated variants, never by an
/// owned dynamic value.
pub struct DynamicOwned<M: Mode> {
    storage: M::OwnedStorage,
    marker: PhantomData<M::Marker>,
}

impl DynamicOwned<Local> {
    /// Wraps `value` as a local owned dynamic value.
    ///
    /// The value must be `'static` so it can participate in `Any` downcasts.
    pub fn new<T: 'static>(value: T) -> Self {
        Self {
            storage: LocalOwnedStorage::Any(Box::new(value)),
            marker: PhantomData,
        }
    }

    /// Returns whether the stored value has the exact type `T`.
    pub fn is<T: 'static>(&self) -> bool {
        self.as_any().is_some_and(|value| value.is::<T>())
    }

    /// Returns the stored value as `T` when its exact type matches.
    ///
    /// Returns `None` when the requested type differs from the stored type.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.as_any().and_then(|value| value.downcast_ref::<T>())
    }

    /// Returns the stored value as mutable `T` when its exact type matches.
    ///
    /// Returns `None` when the requested type differs from the stored type.
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.as_any_mut().and_then(|value| value.downcast_mut::<T>())
    }

    /// Returns the stored value through its local `Any` boundary.
    ///
    /// Owned dynamic values currently always contain an `Any`-compatible value.
    pub fn as_any(&self) -> Option<&dyn Any> {
        let LocalOwnedStorage::Any(value) = &self.storage;
        Some(value.as_ref())
    }

    /// Returns the stored value through its mutable local `Any` boundary.
    ///
    /// Owned dynamic values currently always contain an `Any`-compatible value.
    pub fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        let LocalOwnedStorage::Any(value) = &mut self.storage;
        Some(value.as_mut())
    }

    /// Consumes this wrapper and returns its local `Any` storage.
    ///
    /// Returns the original wrapper only if a future non-`Any` owned variant is
    /// introduced.
    pub fn into_any(self) -> Result<Box<dyn Any>, Self> {
        let Self { storage, marker } = self;
        let LocalOwnedStorage::Any(value) = storage;
        let _ = marker;
        Ok(value)
    }

    /// Consumes this wrapper and returns `T` when its exact type matches.
    ///
    /// Returns the untouched original wrapper when the requested type differs.
    pub fn downcast<T: 'static>(self) -> Result<T, Self> {
        let Self { storage, marker } = self;
        let LocalOwnedStorage::Any(value) = storage;
        match value.downcast::<T>() {
            Ok(value) => Ok(*value),
            Err(value) => Err(Self {
                storage: LocalOwnedStorage::Any(value),
                marker,
            }),
        }
    }
}

impl DynamicOwned<ThreadSafe> {
    /// Wraps `value` as a thread-safe owned dynamic value.
    ///
    /// The value must be `'static + Send + Sync` so the wrapper can retain its
    /// thread-safe erased boundary.
    pub fn new<T: 'static + Send + Sync>(value: T) -> Self {
        Self {
            storage: ThreadSafeOwnedStorage::Any(Box::new(value)),
            marker: PhantomData,
        }
    }

    /// Returns whether the stored value has the exact type `T`.
    pub fn is<T: 'static>(&self) -> bool {
        self.as_any().is_some_and(|value| value.is::<T>())
    }

    /// Returns the stored value as `T` when its exact type matches.
    ///
    /// Returns `None` when the requested type differs from the stored type.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.as_any().and_then(|value| value.downcast_ref::<T>())
    }

    /// Returns the stored value as mutable `T` when its exact type matches.
    ///
    /// Returns `None` when the requested type differs from the stored type.
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.as_any_mut().and_then(|value| value.downcast_mut::<T>())
    }

    /// Returns the stored value through its thread-safe `Any` boundary.
    ///
    /// Owned dynamic values currently always contain an `Any`-compatible value.
    pub fn as_any(&self) -> Option<&(dyn Any + Send + Sync)> {
        let ThreadSafeOwnedStorage::Any(value) = &self.storage;
        Some(value.as_ref())
    }

    /// Returns the stored value through its mutable thread-safe `Any` boundary.
    ///
    /// Owned dynamic values currently always contain an `Any`-compatible value.
    pub fn as_any_mut(&mut self) -> Option<&mut (dyn Any + Send + Sync)> {
        let ThreadSafeOwnedStorage::Any(value) = &mut self.storage;
        Some(value.as_mut())
    }

    /// Consumes this wrapper and returns its thread-safe `Any` storage.
    ///
    /// Returns the original wrapper only if a future non-`Any` owned variant is
    /// introduced.
    pub fn into_any(self) -> Result<Box<dyn Any + Send + Sync>, Self> {
        let Self { storage, marker } = self;
        let ThreadSafeOwnedStorage::Any(value) = storage;
        let _ = marker;
        Ok(value)
    }

    /// Consumes this wrapper and returns `T` when its exact type matches.
    ///
    /// Returns the untouched original wrapper when the requested type differs.
    pub fn downcast<T: 'static>(self) -> Result<T, Self> {
        let Self { storage, marker } = self;
        let ThreadSafeOwnedStorage::Any(value) = storage;
        match value.downcast::<T>() {
            Ok(value) => Ok(*value),
            Err(value) => Err(Self {
                storage: ThreadSafeOwnedStorage::Any(value),
                marker,
            }),
        }
    }

    /// Downgrades this thread-safe wrapper to the local mode without changing
    /// its value.
    pub fn into_local(self) -> DynamicOwned<Local> {
        let Self { storage, .. } = self;
        let ThreadSafeOwnedStorage::Any(value) = storage;
        DynamicOwned {
            storage: LocalOwnedStorage::Any(value),
            marker: PhantomData,
        }
    }
}
