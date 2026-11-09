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

use crate::runtime::value as runtime_value;
pub use crate::runtime::value::Local;
pub use crate::runtime::value::ThreadSafe;

mod sealed;
mod value_for;
mod value_for_mut;
mod value_for_ref;

use sealed::Mode as SealedMode;
use sealed::Sealed;
pub use value_for::ValueFor;
pub use value_for_mut::ValueForMut;
pub use value_for_ref::ValueForRef;

/// A sealed mode controlling which values can cross a thread boundary.
pub trait ValueMode: SealedMode + Sealed {
    // empty
}

impl Sealed for Local {}
impl Sealed for ThreadSafe {}
impl ValueMode for Local {}
impl ValueMode for ThreadSafe {}

/// A failed exact downcast that retains its dynamic value.
///
/// # Type Parameters
///
/// * `V` — dynamic wrapper that retains the unmatched value.
#[must_use]
pub struct DowncastFailure<V> {
    /// Dynamic value retained after its type did not match.
    value: V,
}

impl<V> DowncastFailure<V> {
    /// Retains a value after a failed exact type conversion.
    ///
    /// # Parameters
    ///
    /// * `value` — original value that could not be converted.
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
    /// Formats the type of the retained value without requiring `V: Debug`.
    ///
    /// # Parameters
    ///
    /// * `formatter` — formatter receiving the failure description.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DowncastFailure")
            .field("value", &std::any::type_name::<V>())
            .finish()
    }
}

/// An owned dynamic value with a sealed value mode.
///
/// # Type Parameters
///
/// * `M` — value mode that controls supported dynamic conversions.
pub struct DynamicOwned<M: ValueMode> {
    /// Runtime wrapper that owns the erased value.
    inner: runtime_value::DynamicOwned<<M as sealed::Mode>::Runtime>,
    /// Associates the wrapper with its public value mode.
    mode: PhantomData<M>,
}

impl<M: ValueMode> DynamicOwned<M> {
    /// Stores a reflected value using the selected mode's compile-time bounds.
    ///
    /// # Type Parameters
    ///
    /// * `T` — concrete reflected value type to store.
    ///
    /// # Parameters
    ///
    /// * `value` — value transferred into the dynamic owner.
    pub fn new<T: ValueFor<M>>(value: T) -> Self {
        Self {
            inner: runtime_value::DynamicOwned::new(value),
            mode: PhantomData,
        }
    }

    /// Converts this wrapper into its underlying runtime representation.
    pub(crate) fn into_runtime(self) -> runtime_value::DynamicOwned<<M as sealed::Mode>::Runtime> {
        self.inner
    }

    /// Wraps a runtime-owned value in the sealed public mode type.
    ///
    /// # Parameters
    ///
    /// * `inner` — owned runtime representation to wrap.
    pub(crate) fn from_runtime(inner: runtime_value::DynamicOwned<<M as sealed::Mode>::Runtime>) -> Self {
        Self {
            inner,
            mode: PhantomData,
        }
    }

    /// Returns whether the exact stored type is `T`.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected concrete type to check.
    pub fn is<T: ValueFor<M>>(&self) -> bool {
        self.inner.is::<T>()
    }

    /// Borrows the stored value as `T` when its exact type matches.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected concrete type to borrow.
    pub fn downcast_ref<T: ValueFor<M>>(&self) -> Option<&T> {
        self.inner.downcast_ref::<T>()
    }

    /// Mutably borrows the stored value as `T` when its exact type matches.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected concrete type to borrow mutably.
    pub fn downcast_mut<T: ValueFor<M>>(&mut self) -> Option<&mut T> {
        self.inner.downcast_mut::<T>()
    }

    /// Takes the stored value as `T`, retaining it if its type does not match.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected concrete type to recover.
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
///
/// # Type Parameters
///
/// * `'a` — lifetime of the retained shared borrow.
/// * `M` — value mode that controls supported dynamic conversions.
pub struct DynamicRef<'a, M: ValueMode> {
    /// Runtime wrapper retaining the original shared borrow.
    inner: runtime_value::DynamicRef<'a, <M as sealed::Mode>::Runtime>,
    /// Associates the wrapper with its public value mode.
    mode: PhantomData<M>,
}

impl<'a, M: ValueMode> DynamicRef<'a, M> {
    /// Erases a reflected shared borrow.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected value type being borrowed.
    ///
    /// # Parameters
    ///
    /// * `value` — shared borrow retained by the dynamic wrapper.
    pub fn new<T: ValueForRef<M> + ?Sized>(value: &'a T) -> Self {
        Self {
            inner: runtime_value::DynamicRef::new(value),
            mode: PhantomData,
        }
    }

    /// Erases a string slice borrow.
    ///
    /// # Parameters
    ///
    /// * `value` — string slice borrow retained by the wrapper.
    pub fn new_str(value: &'a str) -> Self
    where
        str: ValueForRef<M>,
    {
        Self::new(value)
    }

    /// Returns whether the exact borrowed type is `T`.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected type to check.
    pub fn is<T: ValueForRef<M> + ?Sized>(&self) -> bool {
        self.inner.clone().is::<T>()
    }

    /// Borrows the value as `T` when its exact type matches.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected type to borrow.
    pub fn downcast_ref<T: ValueForRef<M> + ?Sized>(&self) -> Option<&T> {
        self.inner.downcast_ref::<T>()
    }

    /// Recovers the original shared borrow, retaining the wrapper on mismatch.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected type to recover.
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
    /// Clones the shared dynamic borrow while preserving its lifetime.
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            mode: PhantomData,
        }
    }
}

/// A mutable dynamic borrow with its original borrow lifetime.
///
/// # Type Parameters
///
/// * `'a` — lifetime of the retained exclusive borrow.
/// * `M` — value mode that controls supported dynamic conversions.
pub struct DynamicMut<'a, M: ValueMode> {
    /// Runtime wrapper retaining the original exclusive borrow.
    inner: runtime_value::DynamicMut<'a, <M as sealed::Mode>::Runtime>,
    /// Associates the wrapper with its public value mode.
    mode: PhantomData<M>,
}

impl<'a, M: ValueMode> DynamicMut<'a, M> {
    /// Erases a reflected mutable borrow.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected value type being borrowed.
    ///
    /// # Parameters
    ///
    /// * `value` — exclusive borrow retained by the dynamic wrapper.
    pub fn new<T: ValueForMut<M> + ?Sized>(value: &'a mut T) -> Self {
        Self {
            inner: runtime_value::DynamicMut::new(value),
            mode: PhantomData,
        }
    }

    /// Erases a mutable string slice borrow.
    ///
    /// # Parameters
    ///
    /// * `value` — mutable string slice borrow retained by the wrapper.
    pub fn new_str_mut(value: &'a mut str) -> Self
    where
        str: ValueForMut<M>,
    {
        Self::new(value)
    }

    /// Returns whether the exact borrowed type is `T`.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected type to check.
    pub fn is<T: ValueForMut<M> + ?Sized>(&self) -> bool {
        self.inner.is::<T>()
    }

    /// Borrows the value as `T` when its exact type matches.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected type to borrow.
    pub fn downcast_ref<T: ValueForRef<M> + ?Sized>(&self) -> Option<&T> {
        self.inner.downcast_ref::<T>()
    }

    /// Mutably borrows the value as `T` when its exact type matches.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected type to borrow mutably.
    pub fn downcast_mut<T: ValueForMut<M> + ?Sized>(&mut self) -> Option<&mut T> {
        self.inner.downcast_mut::<T>()
    }

    /// Recovers the original mutable borrow, retaining the wrapper on mismatch.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected type to recover.
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

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::DynamicMut;
    use super::DynamicOwned;
    use super::DynamicRef;
    use super::Local;
    use super::ThreadSafe;
    use crate::runtime::descriptor::Reflect;

    struct LocalOnly(Rc<Cell<u32>>);

    impl Reflect for LocalOnly {}

    struct SharedCounter(u32);

    impl Reflect for SharedCounter {}

    #[test]
    fn test_local_owned_adapter_preserves_typed_and_dynamic_values() {
        let mut value = DynamicOwned::<Local>::new(LocalOnly(Rc::new(Cell::new(7))));
        assert!(value.is::<LocalOnly>());
        assert_eq!(value.downcast_ref::<LocalOnly>().map(|value| value.0.get()), Some(7));
        value.downcast_mut::<LocalOnly>().expect("stored type").0.set(8);

        let runtime = value.into_runtime();
        let value = DynamicOwned::<Local>::from_runtime(runtime);
        assert_eq!(value.downcast::<LocalOnly>().expect("matching type").0.get(), 8);

        let failed = DynamicOwned::<Local>::new(LocalOnly(Rc::new(Cell::new(9))))
            .downcast::<u32>()
            .expect_err("wrong requested type must retain the dynamic owner");
        assert_eq!(
            failed.value().downcast_ref::<LocalOnly>().map(|value| value.0.get()),
            Some(9)
        );
        assert_eq!(
            failed
                .into_value()
                .downcast::<LocalOnly>()
                .expect("retained value")
                .0
                .get(),
            9
        );
    }

    #[test]
    fn test_thread_safe_owned_adapter_preserves_typed_and_dynamic_values() {
        let mut value = DynamicOwned::<ThreadSafe>::new(SharedCounter(12));
        assert!(value.is::<SharedCounter>());
        assert_eq!(value.downcast_ref::<SharedCounter>().map(|value| value.0), Some(12));
        value.downcast_mut::<SharedCounter>().expect("stored type").0 += 1;

        let runtime = value.into_runtime();
        let value = DynamicOwned::<ThreadSafe>::from_runtime(runtime);
        assert_eq!(value.downcast::<SharedCounter>().expect("matching type").0, 13);

        let failed = DynamicOwned::<ThreadSafe>::new(SharedCounter(14))
            .downcast::<u32>()
            .expect_err("wrong requested type must retain the dynamic owner");
        assert_eq!(
            failed.value().downcast_ref::<SharedCounter>().map(|value| value.0),
            Some(14)
        );
    }

    #[test]
    fn test_local_borrowed_adapters_recover_and_retain_mismatches() {
        let local = LocalOnly(Rc::new(Cell::new(21)));
        let shared = DynamicRef::<Local>::new(&local);
        assert!(shared.is::<LocalOnly>());
        assert_eq!(shared.downcast_ref::<LocalOnly>().map(|value| value.0.get()), Some(21));
        assert!(shared.clone().downcast::<u32>().is_err());
        assert_eq!(
            shared.downcast::<LocalOnly>().expect("matching shared borrow").0.get(),
            21
        );

        let mut local = LocalOnly(Rc::new(Cell::new(30)));
        let mut exclusive = DynamicMut::<Local>::new(&mut local);
        assert_eq!(
            exclusive.downcast_ref::<LocalOnly>().map(|value| value.0.get()),
            Some(30)
        );
        exclusive.downcast_mut::<LocalOnly>().expect("stored type").0.set(31);
        let retained = exclusive.downcast::<u32>().expect_err("mismatch keeps mutable borrow");
        retained
            .into_value()
            .downcast::<LocalOnly>()
            .expect("retained mutable borrow")
            .0
            .set(32);
        assert_eq!(local.0.get(), 32);
    }

    #[test]
    fn test_thread_safe_borrowed_adapters_support_sized_and_slice_values() {
        let value = SharedCounter(40);
        let shared = DynamicRef::<ThreadSafe>::new(&value);
        assert_eq!(shared.downcast_ref::<SharedCounter>().map(|value| value.0), Some(40));
        assert!(shared.clone().downcast::<u32>().is_err());
        assert_eq!(
            shared.downcast::<SharedCounter>().expect("matching shared borrow").0,
            40
        );

        let text = String::from("hello");
        let shared_text = DynamicRef::<ThreadSafe>::new_str(text.as_str());
        assert!(shared_text.is::<str>());
        assert_eq!(shared_text.downcast_ref::<str>(), Some("hello"));
        assert_eq!(shared_text.into_str().expect("matching DST borrow"), "hello");

        let mut text = String::from("world");
        let mut text_slice = DynamicMut::<ThreadSafe>::new_str_mut(text.as_mut_str());
        assert_eq!(text_slice.downcast_ref::<str>(), Some("world"));
        text_slice.downcast_mut::<str>().expect("matching mutable DST")[0..1].make_ascii_uppercase();
        assert_eq!(text_slice.into_str_mut().expect("matching mutable DST"), "World");
    }
}
