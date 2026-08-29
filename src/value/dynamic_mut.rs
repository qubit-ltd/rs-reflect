//! Mutable borrowed dynamic values with mode-specific erased storage.

use std::any::Any;
use std::marker::PhantomData;

use crate::value::mode::Mode;
use crate::value::storage::{LocalMutStorage, ThreadSafeMutStorage};
use crate::value::{Local, ThreadSafe};

/// A mutable dynamic value borrow whose erased boundary is selected by `M`.
///
/// [`Local`] preserves the ordinary local `Any` boundary and intentionally does
/// not implement `Send` or `Sync`. [`ThreadSafe`] only accepts `Send + Sync`
/// mutable borrows, so its wrapper can implement both auto traits. The
/// lifetime `'a` is the lifetime of the original exclusive borrow and prevents
/// this wrapper from escaping that borrow.
///
/// Sized values enter through [`Self::new`]. Mutable `str` uses the dedicated
/// [`Self::new_str_mut`] variant; it is accessed through [`Self::as_str`] or
/// [`Self::as_str_mut`] and never masquerades as `Any`.
pub struct DynamicMut<'a, M: Mode> {
    storage: M::MutStorage<'a>,
    marker: PhantomData<M::Marker>,
}

impl<'a> DynamicMut<'a, Local> {
    /// Wraps a sized `'static` value as a local mutable dynamic borrow.
    pub fn new<T: Sized + 'static>(value: &'a mut T) -> Self {
        Self {
            storage: LocalMutStorage::Any(value),
            marker: PhantomData,
        }
    }

    /// Wraps `value` using the dedicated mutable `str` dynamic variant.
    ///
    /// The resulting value is not `Any`-compatible; use [`Self::as_str_mut`]
    /// to access the original borrow.
    pub fn new_str_mut(value: &'a mut str) -> Self {
        Self {
            storage: LocalMutStorage::Str(value),
            marker: PhantomData,
        }
    }

    /// Returns whether the stored `Any` value has the exact type `T`.
    ///
    /// Returns `false` for the dedicated `str` variant.
    pub fn is<T: 'static>(&self) -> bool {
        self.as_any().is_some_and(|value| value.is::<T>())
    }

    /// Returns the stored `Any` value as `T` when its exact type matches.
    ///
    /// Returns `None` for a type mismatch or the dedicated `str` variant.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.as_any().and_then(|value| value.downcast_ref::<T>())
    }

    /// Returns the stored `Any` value as mutable `T` when its exact type matches.
    ///
    /// Returns `None` for a type mismatch or the dedicated `str` variant.
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.as_any_mut()
            .and_then(|value| value.downcast_mut::<T>())
    }

    /// Returns this value through its local `Any` boundary.
    ///
    /// Returns `None` when this wrapper holds a dedicated `str` borrow.
    pub fn as_any(&self) -> Option<&dyn Any> {
        match &self.storage {
            LocalMutStorage::Any(value) => Some(&**value),
            LocalMutStorage::Str(_) => None,
        }
    }

    /// Returns this value through its mutable local `Any` boundary.
    ///
    /// Returns `None` when this wrapper holds a dedicated `str` borrow.
    pub fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        match &mut self.storage {
            LocalMutStorage::Any(value) => Some(&mut **value),
            LocalMutStorage::Str(_) => None,
        }
    }

    /// Returns the dedicated `str` borrow when this wrapper contains one.
    ///
    /// Returns `None` when this wrapper holds an `Any`-compatible value.
    pub fn as_str(&self) -> Option<&str> {
        match &self.storage {
            LocalMutStorage::Any(_) => None,
            LocalMutStorage::Str(value) => Some(&**value),
        }
    }

    /// Returns the dedicated mutable `str` borrow when this wrapper contains one.
    ///
    /// Returns `None` when this wrapper holds an `Any`-compatible value.
    pub fn as_str_mut(&mut self) -> Option<&mut str> {
        match &mut self.storage {
            LocalMutStorage::Any(_) => None,
            LocalMutStorage::Str(value) => Some(&mut **value),
        }
    }
}

impl<'a> DynamicMut<'a, ThreadSafe> {
    /// Wraps a sized, `'static`, `Send`, and `Sync` value as a thread-safe mutable borrow.
    pub fn new<T: Sized + 'static + Send + Sync>(value: &'a mut T) -> Self {
        Self {
            storage: ThreadSafeMutStorage::Any(value),
            marker: PhantomData,
        }
    }

    /// Wraps `value` using the dedicated mutable `str` dynamic variant.
    ///
    /// The resulting value is not `Any`-compatible; use [`Self::as_str_mut`]
    /// to access the original borrow.
    pub fn new_str_mut(value: &'a mut str) -> Self {
        Self {
            storage: ThreadSafeMutStorage::Str(value),
            marker: PhantomData,
        }
    }

    /// Returns whether the stored `Any` value has the exact type `T`.
    ///
    /// Returns `false` for the dedicated `str` variant.
    pub fn is<T: 'static>(&self) -> bool {
        self.as_any().is_some_and(|value| value.is::<T>())
    }

    /// Returns the stored `Any` value as `T` when its exact type matches.
    ///
    /// Returns `None` for a type mismatch or the dedicated `str` variant.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.as_any().and_then(|value| value.downcast_ref::<T>())
    }

    /// Returns the stored `Any` value as mutable `T` when its exact type matches.
    ///
    /// Returns `None` for a type mismatch or the dedicated `str` variant.
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.as_any_mut()
            .and_then(|value| value.downcast_mut::<T>())
    }

    /// Returns this value through its thread-safe `Any` boundary.
    ///
    /// Returns `None` when this wrapper holds a dedicated `str` borrow.
    pub fn as_any(&self) -> Option<&(dyn Any + Send + Sync)> {
        match &self.storage {
            ThreadSafeMutStorage::Any(value) => Some(&**value),
            ThreadSafeMutStorage::Str(_) => None,
        }
    }

    /// Returns this value through its mutable thread-safe `Any` boundary.
    ///
    /// Returns `None` when this wrapper holds a dedicated `str` borrow.
    pub fn as_any_mut(&mut self) -> Option<&mut (dyn Any + Send + Sync)> {
        match &mut self.storage {
            ThreadSafeMutStorage::Any(value) => Some(&mut **value),
            ThreadSafeMutStorage::Str(_) => None,
        }
    }

    /// Returns the dedicated `str` borrow when this wrapper contains one.
    ///
    /// Returns `None` when this wrapper holds an `Any`-compatible value.
    pub fn as_str(&self) -> Option<&str> {
        match &self.storage {
            ThreadSafeMutStorage::Any(_) => None,
            ThreadSafeMutStorage::Str(value) => Some(&**value),
        }
    }

    /// Returns the dedicated mutable `str` borrow when this wrapper contains one.
    ///
    /// Returns `None` when this wrapper holds an `Any`-compatible value.
    pub fn as_str_mut(&mut self) -> Option<&mut str> {
        match &mut self.storage {
            ThreadSafeMutStorage::Any(_) => None,
            ThreadSafeMutStorage::Str(value) => Some(&mut **value),
        }
    }

    /// Downgrades this thread-safe mutable borrow to the local mode without changing it.
    pub fn into_local(self) -> DynamicMut<'a, Local> {
        let Self { storage, .. } = self;
        let storage = match storage {
            ThreadSafeMutStorage::Any(value) => LocalMutStorage::Any(value),
            ThreadSafeMutStorage::Str(value) => LocalMutStorage::Str(value),
        };
        DynamicMut {
            storage,
            marker: PhantomData,
        }
    }
}
