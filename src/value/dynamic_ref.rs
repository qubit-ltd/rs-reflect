//! Shared borrowed dynamic values with mode-specific erased storage.

use std::any::Any;
use std::marker::PhantomData;

use crate::value::mode::Mode;
use crate::value::storage::{LocalRefStorage, ThreadSafeRefStorage};
use crate::value::{Local, ThreadSafe};

/// A shared dynamic value borrow whose erased boundary is selected by `M`.
///
/// [`Local`] preserves the ordinary local `Any` boundary and intentionally does
/// not implement `Send` or `Sync`. [`ThreadSafe`] stores a `Sync` shared
/// borrow, so its wrapper can implement both auto traits. The lifetime `'a` is
/// the lifetime of the original borrow and prevents this wrapper from escaping
/// that borrow.
///
/// Sized values enter through [`Self::new`]. `str` is unsized and therefore
/// uses the dedicated [`Self::new_str`] variant; it is accessed through
/// [`Self::as_str`] and never masquerades as `Any`.
pub struct DynamicRef<'a, M: Mode> {
    storage: M::RefStorage<'a>,
    marker: PhantomData<M::Marker>,
}

impl<'a> DynamicRef<'a, Local> {
    /// Wraps a sized `'static` value as a local shared dynamic borrow.
    pub fn new<T: Sized + 'static>(value: &'a T) -> Self {
        Self {
            storage: LocalRefStorage::Any(value),
            marker: PhantomData,
        }
    }

    /// Wraps `value` using the dedicated `str` dynamic variant.
    ///
    /// The resulting value is not `Any`-compatible; use [`Self::as_str`] to
    /// access the original borrow.
    pub fn new_str(value: &'a str) -> Self {
        Self {
            storage: LocalRefStorage::Str(value),
            marker: PhantomData,
        }
    }

    /// Returns whether the stored `Any` value has the exact type `T`.
    ///
    /// Returns `false` for the dedicated `str` variant.
    pub fn is<T: 'static>(&self) -> bool {
        self.as_any()
            .is_some_and(|value| (value as &dyn Any).is::<T>())
    }

    /// Returns the stored `Any` value as `T` when its exact type matches.
    ///
    /// Returns `None` for a type mismatch or the dedicated `str` variant.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.as_any()
            .and_then(|value| (value as &dyn Any).downcast_ref::<T>())
    }

    /// Returns this value through its local `Any` boundary.
    ///
    /// Returns `None` when this wrapper holds a dedicated `str` borrow.
    pub fn as_any(&self) -> Option<&dyn Any> {
        match &self.storage {
            LocalRefStorage::Any(value) => Some(*value),
            LocalRefStorage::Str(_) => None,
        }
    }

    /// Returns the dedicated `str` borrow when this wrapper contains one.
    ///
    /// Returns `None` when this wrapper holds an `Any`-compatible value.
    pub fn as_str(&self) -> Option<&str> {
        match &self.storage {
            LocalRefStorage::Any(_) => None,
            LocalRefStorage::Str(value) => Some(value),
        }
    }
}

impl<'a> DynamicRef<'a, ThreadSafe> {
    /// Wraps a sized, `'static`, and `Sync` value as a thread-safe shared borrow.
    pub fn new<T: Sized + 'static + Sync>(value: &'a T) -> Self {
        Self {
            storage: ThreadSafeRefStorage::Any(value),
            marker: PhantomData,
        }
    }

    /// Wraps `value` using the dedicated `str` dynamic variant.
    ///
    /// The resulting value is not `Any`-compatible; use [`Self::as_str`] to
    /// access the original borrow.
    pub fn new_str(value: &'a str) -> Self {
        Self {
            storage: ThreadSafeRefStorage::Str(value),
            marker: PhantomData,
        }
    }

    /// Returns whether the stored `Any` value has the exact type `T`.
    ///
    /// Returns `false` for the dedicated `str` variant.
    pub fn is<T: 'static>(&self) -> bool {
        self.as_any()
            .is_some_and(|value| (value as &dyn Any).is::<T>())
    }

    /// Returns the stored `Any` value as `T` when its exact type matches.
    ///
    /// Returns `None` for a type mismatch or the dedicated `str` variant.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.as_any()
            .and_then(|value| (value as &dyn Any).downcast_ref::<T>())
    }

    /// Returns this value through its thread-safe `Any` boundary.
    ///
    /// Returns `None` when this wrapper holds a dedicated `str` borrow.
    pub fn as_any(&self) -> Option<&(dyn Any + Sync)> {
        match &self.storage {
            ThreadSafeRefStorage::Any(value) => Some(*value),
            ThreadSafeRefStorage::Str(_) => None,
        }
    }

    /// Returns the dedicated `str` borrow when this wrapper contains one.
    ///
    /// Returns `None` when this wrapper holds an `Any`-compatible value.
    pub fn as_str(&self) -> Option<&str> {
        match &self.storage {
            ThreadSafeRefStorage::Any(_) => None,
            ThreadSafeRefStorage::Str(value) => Some(value),
        }
    }

    /// Downgrades this thread-safe borrow to the local mode without changing it.
    pub fn into_local(self) -> DynamicRef<'a, Local> {
        let Self { storage, .. } = self;
        let storage = match storage {
            ThreadSafeRefStorage::Any(value) => LocalRefStorage::Any(value),
            ThreadSafeRefStorage::Str(value) => LocalRefStorage::Str(value),
        };
        DynamicRef {
            storage,
            marker: PhantomData,
        }
    }
}
