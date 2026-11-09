// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::Any;
use std::any::TypeId;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) mod sealed {
    /// Marker implemented only by the crate's supported value modes.
    pub trait Sealed {
        // empty
    }
}

/// Object-safe access to an erased shared or mutable value holder.
pub(crate) trait ErasedRef {
    /// Returns the concrete Rust type represented by this holder.
    fn type_id(&self) -> TypeId;
    /// Returns the address of the underlying owned holder.
    fn holder_ptr(&self) -> *const ();
    /// Reports whether this view originated from a mutable borrow.
    fn is_mutable(&self) -> bool;
}

/// Object-safe access to an erased mutable holder in local mode.
pub(crate) trait ErasedMut {
    /// Returns the concrete Rust type represented by this holder.
    fn type_id(&self) -> TypeId;
    /// Returns the address of the underlying owned holder.
    fn holder_ptr(&self) -> *const ();
    /// Returns the mutable address of the underlying owned holder.
    fn holder_ptr_mut(&mut self) -> *mut ();
    /// Creates a shared local view tied to this holder's borrow.
    fn shared_view_local(&self) -> Rc<dyn ErasedRef + '_>;
    /// Reborrows this mutable holder for a shorter lifetime.
    fn reborrow<'b>(&'b mut self) -> Box<dyn ErasedMut + 'b>;
}

/// Erased mutable access that can be shared across threads.
pub(crate) trait ErasedMutThreadSafe: ErasedMut + Send + Sync {
    /// Creates a thread-safe shared view tied to this holder's borrow.
    fn shared_view_thread_safe(&self) -> Arc<dyn ErasedRef + Send + Sync + '_>;
    /// Reborrows this holder while retaining its thread-safety guarantees.
    fn reborrow_thread_safe<'b>(&'b mut self) -> Box<dyn ErasedMutThreadSafe + 'b>;
}

/// Type-erased ownership operations for a dynamic value holder.
pub(crate) trait ErasedOwned {
    /// Returns the concrete Rust type represented by this owner.
    fn type_id(&self) -> TypeId;
    /// Borrows the concrete value through `Any` for checked downcasting.
    fn as_any(&self) -> &dyn Any;
    /// Mutably borrows the concrete value through `Any` for checked
    /// downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// Consumes the owner and recovers its type-erased allocation.
    fn into_holder_any(self: Box<Self>) -> Box<dyn Any>;
    /// Creates a shared view using the local reference-counted mode.
    fn shared_local(&self) -> Rc<dyn ErasedRef + '_>;
    /// Creates a mutable view using the local borrow mode.
    fn mutable_local(&mut self) -> Box<dyn ErasedMut + '_>;
}

/// Type-erased ownership operations for thread-safe dynamic values.
pub(crate) trait ErasedOwnedThreadSafe: ErasedOwned + Send + Sync {
    /// Creates a shared view that is safe to transfer across threads.
    fn shared_thread_safe(&self) -> Arc<dyn ErasedRef + Send + Sync + '_>;
    /// Creates a mutable view that retains thread-safety guarantees.
    fn mutable_thread_safe(&mut self) -> Box<dyn ErasedMutThreadSafe + '_>;
}

/// Selects the storage and invocation guarantees for dynamic values.
///
/// The crate provides [`Local`] and [`ThreadSafe`] modes; the trait is sealed
/// so downstream crates cannot introduce modes with incompatible guarantees.
#[allow(private_bounds)]
pub trait ValueMode: sealed::Sealed + ValueModeInternal + crate::runtime::invoke::ModeAdapter + 'static {
    // empty
}

/// Runtime details kept out of the public value-mode contract.
#[allow(private_bounds)]
pub(crate) trait ValueModeInternal {
    /// Owned wrapper stored by this mode.
    type Owned: 'static;
    /// Marker type used to distinguish value modes.
    type Marker;
    /// Type-erased view used to expose values through `Any`.
    type AnyObject: ?Sized;
    /// Shared borrow representation for this mode.
    type Shared<'a>: Clone;
    /// Mutable borrow representation for this mode.
    type Mutable<'a>;

    /// Converts this mode's `Any` view to its common `Any` trait object.
    ///
    /// # Parameters
    /// - `value`: mode-specific erased object to expose through `Any`.
    fn any_object_as_any(value: &Self::AnyObject) -> &dyn Any;

    /// Returns the concrete type of an owned value.
    ///
    /// # Parameters
    /// - `value`: owned wrapper whose concrete type is requested.
    fn owned_type_id(value: &Self::Owned) -> TypeId;
    /// Borrows an owned value through `Any` for checked downcasting.
    ///
    /// # Parameters
    /// - `value`: owner to expose as `Any`.
    fn owned_as_any(value: &Self::Owned) -> &dyn Any;
    /// Returns the concrete type of a shared borrow.
    ///
    /// # Parameters
    /// - `value`: shared view whose concrete type is requested.
    fn shared_type_id(value: &Self::Shared<'_>) -> TypeId;
    /// Returns the concrete type of a mutable borrow.
    ///
    /// # Parameters
    /// - `value`: mutable view whose concrete type is requested.
    fn mutable_type_id(value: &Self::Mutable<'_>) -> TypeId;
    /// Consumes the mode-specific owner and returns its erased holder.
    ///
    /// # Parameters
    /// - `value`: owner to consume and erase.
    fn owned_into_holder_any(value: Self::Owned) -> Box<dyn Any>;
    /// Borrows an owner as the mode's shared representation.
    ///
    /// # Parameters
    /// - `value`: owner to borrow.
    fn shared_from_owned<'a>(value: &'a Self::Owned) -> Self::Shared<'a>;
    /// Shortens the lifetime of a shared borrow without changing its holder.
    ///
    /// # Parameters
    /// - `value`: shared view to reborrow for a shorter lifetime.
    fn shared_reborrow<'a, 'b>(value: &'b Self::Shared<'a>) -> Self::Shared<'b>
    where
        'a: 'b;
    /// Borrows an owner mutably using this mode's representation.
    ///
    /// # Parameters
    /// - `value`: owner to borrow mutably.
    fn mutable_from_owned<'a>(value: &'a mut Self::Owned) -> Self::Mutable<'a>;
    /// Creates a shared view from a mutable borrow.
    ///
    /// # Parameters
    /// - `value`: mutable view to expose through shared operations.
    fn shared_from_mut<'a, 'b>(value: &'b Self::Mutable<'a>) -> Self::Shared<'b>;
    /// Shortens the lifetime of a mutable borrow.
    ///
    /// # Parameters
    /// - `value`: mutable view to reborrow for a shorter lifetime.
    fn reborrow_mut<'a, 'b>(value: &'b mut Self::Mutable<'a>) -> Self::Mutable<'b>;
}

/// Value mode for single-threaded use without `Send` or `Sync` requirements.
pub enum Local {}
/// Value mode whose erased values and shared borrows are safe to transfer
/// across threads; concrete values must satisfy the applicable `Send + Sync`
/// bounds.
pub enum ThreadSafe {}

/// Owned dynamic value holder for the local storage mode.
pub(crate) struct LocalOwned(
    /// Erased value owner kept behind the local mode boundary.
    pub(super) Box<dyn ErasedOwned>,
);
/// Owned dynamic value holder for the thread-safe storage mode.
pub(crate) struct ThreadSafeOwned(
    /// Erased value owner kept behind the thread-safe mode boundary.
    pub(super) Box<dyn ErasedOwnedThreadSafe>,
);

impl sealed::Sealed for Local {}
impl sealed::Sealed for ThreadSafe {}

impl ValueMode for Local {}
impl ValueModeInternal for Local {
    /// Local owner wrapper.
    type Owned = LocalOwned;
    /// Reference-counted marker that prevents cross-thread transfer.
    type Marker = Rc<()>;
    /// Erased local value type.
    type AnyObject = dyn Any;
    /// Shared local borrows use `Rc`.
    type Shared<'a> = Rc<dyn ErasedRef + 'a>;
    /// Mutable local borrows use a boxed erased view.
    type Mutable<'a> = Box<dyn ErasedMut + 'a>;

    /// Returns a common `Any` view of a local erased value.
    ///
    /// # Parameters
    /// - `value`: local erased object to expose through `Any`.
    fn any_object_as_any(value: &Self::AnyObject) -> &dyn Any {
        value
    }

    /// Returns the concrete type of the local owner.
    ///
    /// # Parameters
    /// - `value`: local owner whose type is requested.
    fn owned_type_id(value: &Self::Owned) -> TypeId {
        ErasedOwned::type_id(value.0.as_ref())
    }
    /// Borrows the local owner as `Any` for checked downcasting.
    ///
    /// # Parameters
    /// - `value`: local owner to expose as `Any`.
    fn owned_as_any(value: &Self::Owned) -> &dyn Any {
        value.0.as_any()
    }
    /// Returns the concrete type of a local shared view.
    ///
    /// # Parameters
    /// - `value`: local shared view whose type is requested.
    fn shared_type_id(value: &Self::Shared<'_>) -> TypeId {
        ErasedRef::type_id(value.as_ref())
    }
    /// Returns the concrete type of a local mutable view.
    ///
    /// # Parameters
    /// - `value`: local mutable view whose type is requested.
    fn mutable_type_id(value: &Self::Mutable<'_>) -> TypeId {
        ErasedMut::type_id(value.as_ref())
    }
    /// Consumes the local owner and recovers its erased holder.
    ///
    /// # Parameters
    /// - `value`: local owner to consume.
    fn owned_into_holder_any(value: Self::Owned) -> Box<dyn Any> {
        value.0.into_holder_any()
    }
    /// Borrows a local owner as a shared view.
    ///
    /// # Parameters
    /// - `value`: local owner to borrow.
    fn shared_from_owned<'a>(value: &'a Self::Owned) -> Self::Shared<'a> {
        value.0.shared_local()
    }
    /// Shortens the lifetime of a local shared view while retaining its holder.
    ///
    /// # Parameters
    /// - `value`: local shared view to reborrow.
    fn shared_reborrow<'a, 'b>(value: &'b Self::Shared<'a>) -> Self::Shared<'b>
    where
        'a: 'b,
    {
        let view: Rc<dyn ErasedRef + 'b> = value.clone();
        view
    }
    /// Borrows a local owner mutably.
    ///
    /// # Parameters
    /// - `value`: local owner to borrow mutably.
    fn mutable_from_owned<'a>(value: &'a mut Self::Owned) -> Self::Mutable<'a> {
        value.0.mutable_local()
    }
    /// Creates a local shared view from a mutable borrow.
    ///
    /// # Parameters
    /// - `value`: local mutable view to expose as shared.
    fn shared_from_mut<'a, 'b>(value: &'b Self::Mutable<'a>) -> Self::Shared<'b> {
        value.shared_view_local()
    }
    /// Reborrows a local mutable view for a shorter lifetime.
    ///
    /// # Parameters
    /// - `value`: local mutable view to reborrow.
    fn reborrow_mut<'a, 'b>(value: &'b mut Self::Mutable<'a>) -> Self::Mutable<'b> {
        value.reborrow()
    }
}

impl ValueMode for ThreadSafe {}
impl ValueModeInternal for ThreadSafe {
    /// Thread-safe owner wrapper.
    type Owned = ThreadSafeOwned;
    /// Unit marker because thread safety is enforced by the owner bounds.
    type Marker = ();
    /// Erased value type restricted to `Sync` values.
    type AnyObject = dyn Any + Sync;
    /// Shared thread-safe borrows use `Arc`.
    type Shared<'a> = Arc<dyn ErasedRef + Send + Sync + 'a>;
    /// Mutable thread-safe borrows use a boxed thread-safe erased view.
    type Mutable<'a> = Box<dyn ErasedMutThreadSafe + 'a>;

    /// Returns a common `Any` view of a thread-safe erased value.
    ///
    /// # Parameters
    /// - `value`: thread-safe erased object to expose through `Any`.
    fn any_object_as_any(value: &Self::AnyObject) -> &dyn Any {
        value
    }

    /// Returns the concrete type of the thread-safe owner.
    ///
    /// # Parameters
    /// - `value`: thread-safe owner whose type is requested.
    fn owned_type_id(value: &Self::Owned) -> TypeId {
        ErasedOwned::type_id(value.0.as_ref())
    }
    /// Borrows the thread-safe owner as `Any` for checked downcasting.
    ///
    /// # Parameters
    /// - `value`: thread-safe owner to expose as `Any`.
    fn owned_as_any(value: &Self::Owned) -> &dyn Any {
        value.0.as_any()
    }
    /// Returns the concrete type of a thread-safe shared view.
    ///
    /// # Parameters
    /// - `value`: shared view whose type is requested.
    fn shared_type_id(value: &Self::Shared<'_>) -> TypeId {
        ErasedRef::type_id(value.as_ref())
    }
    /// Returns the concrete type of a thread-safe mutable view.
    ///
    /// # Parameters
    /// - `value`: mutable view whose type is requested.
    fn mutable_type_id(value: &Self::Mutable<'_>) -> TypeId {
        ErasedMut::type_id(value.as_ref())
    }
    /// Consumes the thread-safe owner and recovers its erased holder.
    ///
    /// # Parameters
    /// - `value`: thread-safe owner to consume.
    fn owned_into_holder_any(value: Self::Owned) -> Box<dyn Any> {
        let ThreadSafeOwned(value) = value;
        let value: Box<dyn ErasedOwned> = value;
        value.into_holder_any()
    }
    /// Borrows a thread-safe owner as a shared view.
    ///
    /// # Parameters
    /// - `value`: thread-safe owner to borrow.
    fn shared_from_owned<'a>(value: &'a Self::Owned) -> Self::Shared<'a> {
        value.0.shared_thread_safe()
    }
    /// Shortens a thread-safe shared borrow while retaining its holder.
    ///
    /// # Parameters
    /// - `value`: thread-safe shared view to reborrow.
    fn shared_reborrow<'a, 'b>(value: &'b Self::Shared<'a>) -> Self::Shared<'b>
    where
        'a: 'b,
    {
        let view: Arc<dyn ErasedRef + Send + Sync + 'b> = value.clone();
        view
    }
    /// Borrows a thread-safe owner mutably.
    ///
    /// # Parameters
    /// - `value`: thread-safe owner to borrow mutably.
    fn mutable_from_owned<'a>(value: &'a mut Self::Owned) -> Self::Mutable<'a> {
        value.0.mutable_thread_safe()
    }
    /// Creates a thread-safe shared view from a mutable borrow.
    ///
    /// # Parameters
    /// - `value`: mutable view to expose as shared.
    fn shared_from_mut<'a, 'b>(value: &'b Self::Mutable<'a>) -> Self::Shared<'b> {
        value.shared_view_thread_safe()
    }
    /// Reborrows a thread-safe mutable view for a shorter lifetime.
    ///
    /// # Parameters
    /// - `value`: thread-safe mutable view to reborrow.
    fn reborrow_mut<'a, 'b>(value: &'b mut Self::Mutable<'a>) -> Self::Mutable<'b> {
        value.reborrow_thread_safe()
    }
}

/// Shared access adapter over a mutable thread-safe holder.
struct MutRefView<'a>(
    /// Mutable erased holder viewed through its shared operations.
    &'a (dyn ErasedMut + Send + Sync),
);
impl ErasedRef for MutRefView<'_> {
    /// Returns the concrete type represented by the mutable holder.
    fn type_id(&self) -> TypeId {
        ErasedMut::type_id(self.0)
    }
    /// Returns the address of the mutable holder.
    fn holder_ptr(&self) -> *const () {
        ErasedMut::holder_ptr(self.0)
    }
    /// Reports that this shared view originated from mutable access.
    fn is_mutable(&self) -> bool {
        true
    }
}

/// Converts a thread-safe erased shared view into a local view adapter.
///
/// # Parameters
/// - `value`: thread-safe shared view to expose through local reference
///   counting.
pub(super) fn thread_safe_ref_into_local(value: Arc<dyn ErasedRef + Send + Sync + '_>) -> Rc<dyn ErasedRef + '_> {
    Rc::new(ArcRefView(value))
}

/// Local wrapper that forwards shared access to an `Arc`-backed erased view.
struct ArcRefView<'a>(
    /// Thread-safe shared view retained by the local wrapper.
    Arc<dyn ErasedRef + Send + Sync + 'a>,
);
impl ErasedRef for ArcRefView<'_> {
    /// Returns the concrete type represented by the wrapped view.
    fn type_id(&self) -> TypeId {
        ErasedRef::type_id(self.0.as_ref())
    }
    /// Returns the address of the wrapped value holder.
    fn holder_ptr(&self) -> *const () {
        self.0.holder_ptr()
    }
    /// Returns whether the wrapped view came from mutable access.
    fn is_mutable(&self) -> bool {
        self.0.is_mutable()
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;
    use std::rc::Rc;
    use std::sync::Arc;

    use super::ErasedRef;
    use super::Local;
    use super::MutRefView;
    use super::ThreadSafe;
    use super::ThreadSafeOwned;
    use super::ValueModeInternal;
    use super::thread_safe_ref_into_local;
    use crate::runtime::value::owned::ValueForInternal;

    #[test]
    fn test_local_mode_dispatches_type_identity_and_borrow_views() {
        let mut owned = <i32 as ValueForInternal<Local>>::erase(41);
        assert_eq!(Local::owned_type_id(&owned), TypeId::of::<i32>());
        assert_eq!(
            TypeId::of::<<Local as ValueModeInternal>::Marker>(),
            TypeId::of::<Rc<()>>()
        );

        let shared = Local::shared_from_owned(&owned);
        assert_eq!(Local::shared_type_id(&shared), TypeId::of::<i32>());
        assert!(!shared.is_mutable());
        let shared_reborrow = Local::shared_reborrow(&shared);
        assert_eq!(
            shared_reborrow.holder_ptr(),
            shared.holder_ptr(),
            "a local shared reborrow must preserve its erased holder"
        );
        drop(shared_reborrow);
        drop(shared);

        let mut mutable = Local::mutable_from_owned(&mut owned);
        assert_eq!(Local::mutable_type_id(&mutable), TypeId::of::<i32>());
        let holder = mutable.holder_ptr();
        assert_eq!(mutable.holder_ptr_mut(), holder as *mut ());
        {
            let shared_mutable = Local::shared_from_mut(&mutable);
            assert_eq!(Local::shared_type_id(&shared_mutable), TypeId::of::<i32>());
            assert!(shared_mutable.is_mutable());
            assert_eq!(shared_mutable.holder_ptr(), holder);
        }
        let mutable_reborrow = Local::reborrow_mut(&mut mutable);
        assert_eq!(Local::mutable_type_id(&mutable_reborrow), TypeId::of::<i32>());
        assert_eq!(mutable_reborrow.holder_ptr(), holder);
        drop(mutable_reborrow);
        drop(mutable);

        let holder_ptr = (&*owned.0) as *const dyn super::ErasedOwned as *const ();
        let erased = Local::owned_into_holder_any(owned);
        let holder = erased
            .downcast_ref::<crate::runtime::value::owned::OwnedValue<i32>>()
            .expect("owned holder type is preserved");
        assert_eq!(holder as *const _ as *const (), holder_ptr);
    }

    #[test]
    fn test_modes_expose_erased_any_views() {
        let local = String::from("local");
        let local_any: &<Local as ValueModeInternal>::AnyObject = &local;
        assert_eq!(
            <Local as ValueModeInternal>::any_object_as_any(local_any)
                .downcast_ref::<String>()
                .map(String::as_str),
            Some("local")
        );
        let local_owned = <String as ValueForInternal<Local>>::erase(String::from("owned-local"));
        assert_eq!(
            <Local as ValueModeInternal>::owned_as_any(&local_owned)
                .downcast_ref::<String>()
                .map(String::as_str),
            Some("owned-local")
        );

        let thread_safe = String::from("thread-safe");
        let thread_safe_any: &<ThreadSafe as ValueModeInternal>::AnyObject = &thread_safe;
        assert_eq!(
            <ThreadSafe as ValueModeInternal>::any_object_as_any(thread_safe_any)
                .downcast_ref::<String>()
                .map(String::as_str),
            Some("thread-safe")
        );
        let thread_safe_owned = <String as ValueForInternal<ThreadSafe>>::erase(String::from("owned-thread-safe"));
        assert_eq!(
            <ThreadSafe as ValueModeInternal>::owned_as_any(&thread_safe_owned)
                .downcast_ref::<String>()
                .map(String::as_str),
            Some("owned-thread-safe")
        );
    }

    #[test]
    fn test_thread_safe_mode_dispatches_and_converts_shared_views() {
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<ThreadSafeOwned>();
        assert_send_sync::<<ThreadSafe as ValueModeInternal>::Marker>();
        assert_send_sync::<<ThreadSafe as ValueModeInternal>::Shared<'static>>();
        assert_send_sync::<<ThreadSafe as ValueModeInternal>::Mutable<'static>>();
        assert_eq!(
            TypeId::of::<<ThreadSafe as ValueModeInternal>::Marker>(),
            TypeId::of::<()>()
        );

        let mut owned = <i32 as ValueForInternal<ThreadSafe>>::erase(73);
        assert_eq!(ThreadSafe::owned_type_id(&owned), TypeId::of::<i32>());
        let shared = ThreadSafe::shared_from_owned(&owned);
        assert_eq!(ThreadSafe::shared_type_id(&shared), TypeId::of::<i32>());
        assert!(!shared.is_mutable());
        let shared_reborrow = ThreadSafe::shared_reborrow(&shared);
        assert_eq!(shared_reborrow.holder_ptr(), shared.holder_ptr());
        let local_view = thread_safe_ref_into_local(Arc::clone(&shared));
        assert_eq!(local_view.type_id(), TypeId::of::<i32>());
        assert_eq!(local_view.holder_ptr(), shared.holder_ptr());
        assert!(!local_view.is_mutable());
        drop(local_view);
        drop(shared_reborrow);
        drop(shared);

        let mut mutable = ThreadSafe::mutable_from_owned(&mut owned);
        assert_eq!(ThreadSafe::mutable_type_id(&mutable), TypeId::of::<i32>());
        let holder = mutable.holder_ptr();
        assert_eq!(mutable.holder_ptr_mut(), holder as *mut ());
        {
            let shared_mutable = ThreadSafe::shared_from_mut(&mutable);
            assert_eq!(ThreadSafe::shared_type_id(&shared_mutable), TypeId::of::<i32>());
            assert!(shared_mutable.is_mutable());
            assert_eq!(shared_mutable.holder_ptr(), holder);
        }
        {
            let mut_ref_view = MutRefView(mutable.as_ref());
            assert_eq!(mut_ref_view.type_id(), TypeId::of::<i32>());
            assert_eq!(mut_ref_view.holder_ptr(), holder);
            assert!(mut_ref_view.is_mutable());
        }
        let mutable_reborrow = ThreadSafe::reborrow_mut(&mut mutable);
        assert_eq!(ThreadSafe::mutable_type_id(&mutable_reborrow), TypeId::of::<i32>());
        assert_eq!(mutable_reborrow.holder_ptr(), holder);
        drop(mutable_reborrow);
        drop(mutable);

        let holder_ptr = (&*owned.0) as *const dyn super::ErasedOwnedThreadSafe as *const ();
        let erased = ThreadSafe::owned_into_holder_any(owned);
        let holder = erased
            .downcast_ref::<crate::runtime::value::owned::OwnedValue<i32>>()
            .expect("owned holder type is preserved");
        assert_eq!(holder as *const _ as *const (), holder_ptr);
    }
}
