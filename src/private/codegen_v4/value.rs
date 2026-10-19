// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed dynamic-value wrappers used by generated adapters.

use std::fmt;
use std::marker::PhantomData;

use crate::runtime::descriptor::Reflect;
use crate::runtime::value as runtime_value;

pub use crate::runtime::value::{Local, ThreadSafe};

mod sealed {
    use super::{Reflect, runtime_value};

    pub(crate) trait Mode: 'static {
        type Runtime: runtime_value::RuntimeValueMode;
    }

    pub(crate) trait OwnedValue<M: Mode>:
        Reflect + Sized + 'static + runtime_value::RuntimeValueFor<M::Runtime>
    {
    }

    pub(crate) trait SharedValue<M: Mode>:
        Reflect + runtime_value::RuntimeValueForRef<M::Runtime>
    {
    }

    pub(crate) trait MutableValue<M: Mode>:
        Reflect + runtime_value::RuntimeValueForMut<M::Runtime>
    {
    }

    pub(crate) trait Sealed {}

    impl<T: Reflect + Sized + 'static, M: Mode> OwnedValue<M> for T where
        T: runtime_value::RuntimeValueFor<M::Runtime>
    {
    }

    impl<T: Reflect + ?Sized, M: Mode> SharedValue<M> for T where
        T: runtime_value::RuntimeValueForRef<M::Runtime>
    {
    }

    impl<T: Reflect + ?Sized, M: Mode> MutableValue<M> for T where
        T: runtime_value::RuntimeValueForMut<M::Runtime>
    {
    }
}

/// A sealed mode controlling which values can cross a thread boundary.
pub trait ValueMode: sealed::Mode + sealed::Sealed {}

impl sealed::Sealed for Local {}
impl sealed::Sealed for ThreadSafe {}
impl sealed::Mode for Local {
    type Runtime = runtime_value::Local;
}
impl sealed::Mode for ThreadSafe {
    type Runtime = runtime_value::ThreadSafe;
}
impl ValueMode for Local {}
impl ValueMode for ThreadSafe {}

/// A sealed marker for sized values that can be stored in a dynamic owner.
pub trait ValueFor<M: ValueMode>: Reflect + Sized + 'static + sealed::OwnedValue<M> {}

impl<T: Reflect + Sized + 'static> ValueFor<Local> for T where
    T: runtime_value::RuntimeValueFor<runtime_value::Local>
{
}

impl<T: Reflect + Send + Sync + Sized + 'static> ValueFor<ThreadSafe> for T where
    T: runtime_value::RuntimeValueFor<runtime_value::ThreadSafe>
{
}

/// A sealed marker for values that can be represented by a shared dynamic borrow.
pub trait ValueForRef<M: ValueMode>: Reflect + sealed::SharedValue<M> {}

impl<T: Reflect + ?Sized> ValueForRef<Local> for T where
    T: runtime_value::RuntimeValueForRef<runtime_value::Local>
{
}

impl<T: Reflect + Sync + ?Sized> ValueForRef<ThreadSafe> for T where
    T: runtime_value::RuntimeValueForRef<runtime_value::ThreadSafe>
{
}

/// A sealed marker for values that can be represented by a mutable dynamic borrow.
pub trait ValueForMut<M: ValueMode>: Reflect + sealed::MutableValue<M> {}

impl<T: Reflect + ?Sized> ValueForMut<Local> for T where
    T: runtime_value::RuntimeValueForMut<runtime_value::Local>
{
}

impl<T: Reflect + Send + Sync + ?Sized> ValueForMut<ThreadSafe> for T where
    T: runtime_value::RuntimeValueForMut<runtime_value::ThreadSafe>
{
}

/// A failed exact downcast that retains its dynamic value.
#[must_use]
pub struct DowncastFailure<V> {
    value: V,
}

impl<V> DowncastFailure<V> {
    fn new(value: V) -> Self {
        Self { value }
    }

    /// Borrows the value that did not match the requested type.
    pub fn value(&self) -> &V {
        &self.value
    }

    /// Recovers the value that did not match the requested type.
    pub fn into_value(self) -> V {
        self.value
    }
}

impl<V> fmt::Debug for DowncastFailure<V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DowncastFailure")
            .field("value", &std::any::type_name::<V>())
            .finish()
    }
}

/// An owned dynamic value with a sealed value mode.
pub struct DynamicOwned<M: ValueMode> {
    inner: runtime_value::DynamicOwned<<M as sealed::Mode>::Runtime>,
    mode: PhantomData<M>,
}

impl<M: ValueMode> DynamicOwned<M> {
    /// Stores a reflected value using the selected mode's compile-time bounds.
    pub fn new<T: ValueFor<M>>(value: T) -> Self {
        Self {
            inner: runtime_value::DynamicOwned::new(value),
            mode: PhantomData,
        }
    }

    pub(crate) fn into_runtime(self) -> runtime_value::DynamicOwned<<M as sealed::Mode>::Runtime> {
        self.inner
    }

    pub(crate) fn from_runtime(
        inner: runtime_value::DynamicOwned<<M as sealed::Mode>::Runtime>,
    ) -> Self {
        Self {
            inner,
            mode: PhantomData,
        }
    }

    /// Returns whether the exact stored type is `T`.
    pub fn is<T: ValueFor<M>>(&self) -> bool {
        self.inner.is::<T>()
    }

    /// Borrows the stored value as `T` when its exact type matches.
    pub fn downcast_ref<T: ValueFor<M>>(&self) -> Option<&T> {
        self.inner.downcast_ref::<T>()
    }

    /// Mutably borrows the stored value as `T` when its exact type matches.
    pub fn downcast_mut<T: ValueFor<M>>(&mut self) -> Option<&mut T> {
        self.inner.downcast_mut::<T>()
    }

    /// Takes the stored value as `T`, retaining it if its type does not match.
    pub fn downcast<T: ValueFor<M>>(self) -> Result<T, DowncastFailure<Self>> {
        self.inner.downcast::<T>().map_err(|failure| {
            DowncastFailure::new(Self {
                inner: failure.into_value(),
                mode: PhantomData,
            })
        })
    }

    /// Borrows the owned value through a shared dynamic wrapper.
    pub fn as_ref(&self) -> DynamicRef<'_, M> {
        DynamicRef {
            inner: self.inner.as_ref(),
            mode: PhantomData,
        }
    }

    /// Borrows the owned value through a mutable dynamic wrapper.
    pub fn as_mut(&mut self) -> DynamicMut<'_, M> {
        DynamicMut {
            inner: self.inner.as_mut(),
            mode: PhantomData,
        }
    }
}

/// A shared dynamic borrow with its original borrow lifetime.
pub struct DynamicRef<'a, M: ValueMode> {
    inner: runtime_value::DynamicRef<'a, <M as sealed::Mode>::Runtime>,
    mode: PhantomData<M>,
}

impl<'a, M: ValueMode> DynamicRef<'a, M> {
    /// Erases a reflected shared borrow.
    pub fn new<T: ValueForRef<M> + ?Sized>(value: &'a T) -> Self {
        Self {
            inner: runtime_value::DynamicRef::new(value),
            mode: PhantomData,
        }
    }

    /// Erases a string slice borrow.
    pub fn new_str(value: &'a str) -> Self
    where
        str: ValueForRef<M>,
    {
        Self::new(value)
    }

    /// Returns whether the exact borrowed type is `T`.
    pub fn is<T: ValueForRef<M> + ?Sized>(&self) -> bool {
        self.inner.clone().is::<T>()
    }

    /// Borrows the value as `T` when its exact type matches.
    pub fn downcast_ref<T: ValueForRef<M> + ?Sized>(&self) -> Option<&T> {
        self.inner.downcast_ref::<T>()
    }

    /// Recovers the original shared borrow, retaining the wrapper on mismatch.
    pub fn downcast<T: ValueForRef<M> + ?Sized>(self) -> Result<&'a T, DowncastFailure<Self>> {
        self.inner.downcast::<T>().map_err(|failure| {
            DowncastFailure::new(Self {
                inner: failure.into_value(),
                mode: PhantomData,
            })
        })
    }

    /// Recovers the original string slice borrow.
    pub fn into_str(self) -> Result<&'a str, DowncastFailure<Self>>
    where
        str: ValueForRef<M>,
    {
        self.downcast::<str>()
    }

    /// Creates a shared wrapper whose lifetime is shortened to this borrow.
    pub fn reborrow(&self) -> DynamicRef<'_, M> {
        DynamicRef {
            inner: self.inner.reborrow(),
            mode: PhantomData,
        }
    }
}

impl<M: ValueMode> Clone for DynamicRef<'_, M> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            mode: PhantomData,
        }
    }
}

/// A mutable dynamic borrow with its original borrow lifetime.
pub struct DynamicMut<'a, M: ValueMode> {
    inner: runtime_value::DynamicMut<'a, <M as sealed::Mode>::Runtime>,
    mode: PhantomData<M>,
}

impl<'a, M: ValueMode> DynamicMut<'a, M> {
    /// Erases a reflected mutable borrow.
    pub fn new<T: ValueForMut<M> + ?Sized>(value: &'a mut T) -> Self {
        Self {
            inner: runtime_value::DynamicMut::new(value),
            mode: PhantomData,
        }
    }

    /// Erases a mutable string slice borrow.
    pub fn new_str_mut(value: &'a mut str) -> Self
    where
        str: ValueForMut<M>,
    {
        Self::new(value)
    }

    /// Returns whether the exact borrowed type is `T`.
    pub fn is<T: ValueForMut<M> + ?Sized>(&self) -> bool {
        self.inner.is::<T>()
    }

    /// Borrows the value as `T` when its exact type matches.
    pub fn downcast_ref<T: ValueForRef<M> + ?Sized>(&self) -> Option<&T> {
        self.inner.downcast_ref::<T>()
    }

    /// Mutably borrows the value as `T` when its exact type matches.
    pub fn downcast_mut<T: ValueForMut<M> + ?Sized>(&mut self) -> Option<&mut T> {
        self.inner.downcast_mut::<T>()
    }

    /// Recovers the original mutable borrow, retaining the wrapper on mismatch.
    pub fn downcast<T: ValueForMut<M> + ?Sized>(self) -> Result<&'a mut T, DowncastFailure<Self>> {
        self.inner.downcast::<T>().map_err(|failure| {
            DowncastFailure::new(Self {
                inner: failure.into_value(),
                mode: PhantomData,
            })
        })
    }

    /// Recovers the original mutable string slice borrow.
    pub fn into_str_mut(self) -> Result<&'a mut str, DowncastFailure<Self>>
    where
        str: ValueForMut<M>,
    {
        self.downcast::<str>()
    }

    /// Creates a shared wrapper whose lifetime is shortened to this borrow.
    pub fn as_ref(&self) -> DynamicRef<'_, M> {
        DynamicRef {
            inner: self.inner.as_ref(),
            mode: PhantomData,
        }
    }

    /// Creates a mutable wrapper with a reborrowed lifetime.
    pub fn reborrow(&mut self) -> DynamicMut<'_, M> {
        DynamicMut {
            inner: self.inner.reborrow(),
            mode: PhantomData,
        }
    }
}
