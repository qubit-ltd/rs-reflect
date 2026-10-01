// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Owned dynamic values with mode-specific erased storage.

use std::any::Any;
use std::marker::PhantomData;

use crate::value::DynamicMut;
use crate::value::DynamicRef;
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
/// represented exclusively by [`DynamicRef`] and
/// [`DynamicMut`]'s dedicated variants, never by an
/// owned dynamic value.
///
/// # Examples
///
/// ```
/// use qubit_reflect::value::{DynamicOwned, Local};
///
/// let value = DynamicOwned::<Local>::new(String::from("ready"));
/// assert_eq!(value.downcast_ref::<String>().map(String::as_str), Some("ready"));
/// ```
pub struct DynamicOwned<M: Mode> {
    /// Mode-specific erased owner of the concrete value.
    storage: M::OwnedStorage,
    /// Keeps the mode's auto-trait boundary in the wrapper type.
    marker: PhantomData<M::Marker>,
}

impl DynamicOwned<Local> {
    /// Wraps `value` as a local owned dynamic value.
    ///
    /// The value must be `'static` so it can participate in `Any` downcasts.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete owned value type.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value to erase.
    ///
    /// # Returns
    ///
    /// Returns a local dynamic wrapper containing `value`.
    pub fn new<T: 'static>(value: T) -> Self {
        Self {
            storage: LocalOwnedStorage::Any(Box::new(value)),
            marker: PhantomData,
        }
    }

    /// Returns the exact identity of the owned value.
    ///
    /// # Returns
    ///
    /// Returns the concrete value's process-local `TypeId`.
    #[must_use]
    #[inline]
    pub fn value_type_id(&self) -> std::any::TypeId {
        self.as_any().expect("owned values are Any-compatible").type_id()
    }

    /// Borrows the owned erased value without exposing its concrete type.
    ///
    /// # Returns
    ///
    /// Returns a shared dynamic borrow tied to this wrapper's borrow.
    #[must_use]
    #[inline]
    pub fn as_reflected_ref(&self) -> DynamicRef<'_, Local> {
        let LocalOwnedStorage::Any(value) = &self.storage;
        DynamicRef::<Local>::from_any(value.as_ref())
    }

    /// Mutably borrows the owned erased value without exposing its concrete
    /// type.
    ///
    /// # Returns
    ///
    /// Returns an exclusive dynamic borrow tied to this wrapper's borrow.
    #[must_use]
    #[inline]
    pub fn as_reflected_mut(&mut self) -> DynamicMut<'_, Local> {
        let LocalOwnedStorage::Any(value) = &mut self.storage;
        DynamicMut::<Local>::from_any(value.as_mut())
    }

    /// Returns whether the stored value has the exact type `T`.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete type to compare with the stored value.
    ///
    /// # Returns
    ///
    /// Returns `true` only when the stored value has exactly type `T`.
    #[must_use]
    pub fn is<T: 'static>(&self) -> bool {
        self.as_any().is_some_and(|value| value.is::<T>())
    }

    /// Returns the stored value as `T` when its exact type matches.
    ///
    /// Returns `None` when the requested type differs from the stored type.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete type requested for the shared borrow.
    ///
    /// # Returns
    ///
    /// Returns a shared `T` reference on an exact type match, or `None`.
    #[must_use]
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.as_any().and_then(|value| value.downcast_ref::<T>())
    }

    /// Returns the stored value as mutable `T` when its exact type matches.
    ///
    /// Returns `None` when the requested type differs from the stored type.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete type requested for the exclusive borrow.
    ///
    /// # Returns
    ///
    /// Returns a mutable `T` reference on an exact type match, or `None`.
    #[must_use]
    #[inline]
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.as_any_mut().and_then(|value| value.downcast_mut::<T>())
    }

    /// Returns the stored value through its local `Any` boundary.
    ///
    /// Owned dynamic values currently always contain an `Any`-compatible value.
    ///
    /// # Returns
    ///
    /// Returns the erased value; the option is retained for mode symmetry.
    #[must_use]
    #[inline]
    pub fn as_any(&self) -> Option<&dyn Any> {
        let LocalOwnedStorage::Any(value) = &self.storage;
        Some(value.as_ref())
    }

    /// Returns the stored value through its mutable local `Any` boundary.
    ///
    /// Owned dynamic values currently always contain an `Any`-compatible value.
    ///
    /// # Returns
    ///
    /// Returns mutable erased access; the option is retained for mode symmetry.
    #[must_use]
    #[inline]
    pub fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        let LocalOwnedStorage::Any(value) = &mut self.storage;
        Some(value.as_mut())
    }

    /// Consumes this wrapper and returns its local `Any` storage.
    ///
    /// Returns the original wrapper only if a future non-`Any` owned variant is
    /// introduced.
    ///
    /// # Returns
    ///
    /// Returns the boxed erased value, or the original wrapper if it cannot be
    /// represented by `Any`.
    pub fn into_any(self) -> Result<Box<dyn Any>, Self> {
        let Self { storage, marker } = self;
        let LocalOwnedStorage::Any(value) = storage;
        let _ = marker;
        Ok(value)
    }

    /// Consumes this wrapper and returns `T` when its exact type matches.
    ///
    /// Returns the untouched original wrapper when the requested type differs.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete owned value type requested by the caller.
    ///
    /// # Returns
    ///
    /// Returns the owned `T`, or the original wrapper on a type mismatch.
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
    /// Returns the exact identity of the owned value.
    ///
    /// # Returns
    ///
    /// Returns the concrete value's process-local `TypeId`.
    #[must_use]
    #[inline]
    pub fn value_type_id(&self) -> std::any::TypeId {
        self.as_any().expect("owned values are Any-compatible").type_id()
    }
    /// Wraps `value` as a thread-safe owned dynamic value.
    ///
    /// The value must be `'static + Send + Sync` so the wrapper can retain its
    /// thread-safe erased boundary.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Owned value type satisfying the thread-safe storage bounds.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value to erase.
    ///
    /// # Returns
    ///
    /// Returns a thread-safe dynamic wrapper containing `value`.
    pub fn new<T: 'static + Send + Sync>(value: T) -> Self {
        Self {
            storage: ThreadSafeOwnedStorage::Any(Box::new(value)),
            marker: PhantomData,
        }
    }

    /// Borrows the owned value while preserving the thread-safe erased mode.
    ///
    /// # Returns
    ///
    /// Returns a thread-safe shared dynamic borrow tied to this wrapper.
    #[must_use]
    #[inline]
    pub fn as_reflected_ref(&self) -> DynamicRef<'_, ThreadSafe> {
        let ThreadSafeOwnedStorage::Any(value) = &self.storage;
        DynamicRef::<ThreadSafe>::from_any(value.as_ref())
    }

    /// Mutably borrows the owned value while preserving the thread-safe erased
    /// mode.
    ///
    /// # Returns
    ///
    /// Returns a thread-safe exclusive dynamic borrow tied to this wrapper.
    #[must_use]
    #[inline]
    pub fn as_reflected_mut(&mut self) -> DynamicMut<'_, ThreadSafe> {
        let ThreadSafeOwnedStorage::Any(value) = &mut self.storage;
        DynamicMut::<ThreadSafe>::from_any(value.as_mut())
    }

    /// Returns whether the stored value has the exact type `T`.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete type to compare with the stored value.
    ///
    /// # Returns
    ///
    /// Returns `true` only when the stored value has exactly type `T`.
    #[must_use]
    pub fn is<T: 'static>(&self) -> bool {
        self.as_any().is_some_and(|value| value.is::<T>())
    }

    /// Returns the stored value as `T` when its exact type matches.
    ///
    /// Returns `None` when the requested type differs from the stored type.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete type requested for the shared borrow.
    ///
    /// # Returns
    ///
    /// Returns a shared `T` reference on an exact type match, or `None`.
    #[must_use]
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.as_any().and_then(|value| value.downcast_ref::<T>())
    }

    /// Returns the stored value as mutable `T` when its exact type matches.
    ///
    /// Returns `None` when the requested type differs from the stored type.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete type requested for the exclusive borrow.
    ///
    /// # Returns
    ///
    /// Returns a mutable `T` reference on an exact type match, or `None`.
    #[must_use]
    #[inline]
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.as_any_mut().and_then(|value| value.downcast_mut::<T>())
    }

    /// Returns the stored value through its thread-safe `Any` boundary.
    ///
    /// Owned dynamic values currently always contain an `Any`-compatible value.
    ///
    /// # Returns
    ///
    /// Returns the erased value with its `Send + Sync` boundary.
    #[must_use]
    #[inline]
    pub fn as_any(&self) -> Option<&(dyn Any + Send + Sync)> {
        let ThreadSafeOwnedStorage::Any(value) = &self.storage;
        Some(value.as_ref())
    }

    /// Returns the stored value through its mutable thread-safe `Any` boundary.
    ///
    /// Owned dynamic values currently always contain an `Any`-compatible value.
    ///
    /// # Returns
    ///
    /// Returns mutable erased access with its `Send + Sync` boundary.
    #[must_use]
    #[inline]
    pub fn as_any_mut(&mut self) -> Option<&mut (dyn Any + Send + Sync)> {
        let ThreadSafeOwnedStorage::Any(value) = &mut self.storage;
        Some(value.as_mut())
    }

    /// Consumes this wrapper and returns its thread-safe `Any` storage.
    ///
    /// Returns the original wrapper only if a future non-`Any` owned variant is
    /// introduced.
    ///
    /// # Returns
    ///
    /// Returns the boxed erased value, or the original wrapper if unavailable.
    pub fn into_any(self) -> Result<Box<dyn Any + Send + Sync>, Self> {
        let Self { storage, marker } = self;
        let ThreadSafeOwnedStorage::Any(value) = storage;
        let _ = marker;
        Ok(value)
    }

    /// Consumes this wrapper and returns `T` when its exact type matches.
    ///
    /// Returns the untouched original wrapper when the requested type differs.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete owned value type requested by the caller.
    ///
    /// # Returns
    ///
    /// Returns the owned `T`, or the original wrapper on a type mismatch.
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
    ///
    /// # Returns
    ///
    /// Returns the same owned value in local mode.
    #[must_use]
    pub fn into_local(self) -> DynamicOwned<Local> {
        let Self { storage, .. } = self;
        let ThreadSafeOwnedStorage::Any(value) = storage;
        DynamicOwned {
            storage: LocalOwnedStorage::Any(value),
            marker: PhantomData,
        }
    }
}
