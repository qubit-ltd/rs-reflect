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
    pub trait Sealed {}
}

pub(crate) trait ErasedRef {
    fn type_id(&self) -> TypeId;
    fn holder_ptr(&self) -> *const ();
    fn is_mutable(&self) -> bool;
}

pub(crate) trait ErasedMut {
    fn type_id(&self) -> TypeId;
    fn holder_ptr(&self) -> *const ();
    fn holder_ptr_mut(&mut self) -> *mut ();
    fn shared_view_local(&self) -> Rc<dyn ErasedRef + '_>;
    fn reborrow<'b>(&'b mut self) -> Box<dyn ErasedMut + 'b>;
}

pub(crate) trait ErasedMutThreadSafe: ErasedMut + Send + Sync {
    fn shared_view_thread_safe(&self) -> Arc<dyn ErasedRef + Send + Sync + '_>;
    fn reborrow_thread_safe<'b>(&'b mut self) -> Box<dyn ErasedMutThreadSafe + 'b>;
}

pub(crate) trait ErasedOwned {
    fn type_id(&self) -> TypeId;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    fn shared_local(&self) -> Rc<dyn ErasedRef + '_>;
    fn mutable_local(&mut self) -> Box<dyn ErasedMut + '_>;
}

pub(crate) trait ErasedOwnedThreadSafe: ErasedOwned + Send + Sync {
    fn shared_thread_safe(&self) -> Arc<dyn ErasedRef + Send + Sync + '_>;
    fn mutable_thread_safe(&mut self) -> Box<dyn ErasedMutThreadSafe + '_>;
}

#[allow(private_bounds)]
/// Selects the storage and invocation guarantees for dynamic values.
///
/// The crate provides [`Local`] and [`ThreadSafe`] modes; the trait is sealed
/// so downstream crates cannot introduce modes with incompatible guarantees.
pub trait ValueMode: sealed::Sealed + ValueModeInternal + crate::runtime::invoke::ModeAdapter + 'static {}

/// Runtime details kept out of the public value-mode contract.
#[allow(private_bounds)]
pub(crate) trait ValueModeInternal {
    type Owned: 'static;
    type Marker;
    type AnyObject: ?Sized;
    type Shared<'a>: Clone;
    type Mutable<'a>;

    fn any_object_as_any(value: &Self::AnyObject) -> &dyn Any;

    fn owned_type_id(value: &Self::Owned) -> TypeId;
    fn owned_as_any(value: &Self::Owned) -> &dyn Any;
    fn shared_type_id(value: &Self::Shared<'_>) -> TypeId;
    fn mutable_type_id(value: &Self::Mutable<'_>) -> TypeId;
    fn owned_into_any(value: Self::Owned) -> Box<dyn Any>;
    fn shared_from_owned<'a>(value: &'a Self::Owned) -> Self::Shared<'a>;
    fn shared_reborrow<'a, 'b>(value: &'b Self::Shared<'a>) -> Self::Shared<'b>
    where
        'a: 'b;
    fn mutable_from_owned<'a>(value: &'a mut Self::Owned) -> Self::Mutable<'a>;
    fn shared_from_mut<'a, 'b>(value: &'b Self::Mutable<'a>) -> Self::Shared<'b>;
    fn reborrow_mut<'a, 'b>(value: &'b mut Self::Mutable<'a>) -> Self::Mutable<'b>;
}

/// Value mode for single-threaded use without `Send` or `Sync` requirements.
pub enum Local {}
/// Value mode whose erased values and shared borrows are safe to transfer
/// across threads; concrete values must satisfy the applicable `Send + Sync`
/// bounds.
pub enum ThreadSafe {}

pub(crate) struct LocalOwned(pub(super) Box<dyn ErasedOwned>);
pub(crate) struct ThreadSafeOwned(pub(super) Box<dyn ErasedOwnedThreadSafe>);

impl sealed::Sealed for Local {}
impl sealed::Sealed for ThreadSafe {}

impl ValueMode for Local {}
impl ValueModeInternal for Local {
    type Owned = LocalOwned;
    type Marker = Rc<()>;
    type AnyObject = dyn Any;
    type Shared<'a> = Rc<dyn ErasedRef + 'a>;
    type Mutable<'a> = Box<dyn ErasedMut + 'a>;

    fn any_object_as_any(value: &Self::AnyObject) -> &dyn Any {
        value
    }

    fn owned_type_id(value: &Self::Owned) -> TypeId {
        ErasedOwned::type_id(value.0.as_ref())
    }
    fn owned_as_any(value: &Self::Owned) -> &dyn Any {
        value.0.as_any()
    }
    fn shared_type_id(value: &Self::Shared<'_>) -> TypeId {
        ErasedRef::type_id(value.as_ref())
    }
    fn mutable_type_id(value: &Self::Mutable<'_>) -> TypeId {
        ErasedMut::type_id(value.as_ref())
    }
    fn owned_into_any(value: Self::Owned) -> Box<dyn Any> {
        value.0.into_any()
    }
    fn shared_from_owned<'a>(value: &'a Self::Owned) -> Self::Shared<'a> {
        value.0.shared_local()
    }
    fn shared_reborrow<'a, 'b>(value: &'b Self::Shared<'a>) -> Self::Shared<'b>
    where
        'a: 'b,
    {
        let view: Rc<dyn ErasedRef + 'b> = value.clone();
        view
    }
    fn mutable_from_owned<'a>(value: &'a mut Self::Owned) -> Self::Mutable<'a> {
        value.0.mutable_local()
    }
    fn shared_from_mut<'a, 'b>(value: &'b Self::Mutable<'a>) -> Self::Shared<'b> {
        value.shared_view_local()
    }
    fn reborrow_mut<'a, 'b>(value: &'b mut Self::Mutable<'a>) -> Self::Mutable<'b> {
        value.reborrow()
    }
}

impl ValueMode for ThreadSafe {}
impl ValueModeInternal for ThreadSafe {
    type Owned = ThreadSafeOwned;
    type Marker = ();
    type AnyObject = dyn Any + Sync;
    type Shared<'a> = Arc<dyn ErasedRef + Send + Sync + 'a>;
    type Mutable<'a> = Box<dyn ErasedMutThreadSafe + 'a>;

    fn any_object_as_any(value: &Self::AnyObject) -> &dyn Any {
        value
    }

    fn owned_type_id(value: &Self::Owned) -> TypeId {
        ErasedOwned::type_id(value.0.as_ref())
    }
    fn owned_as_any(value: &Self::Owned) -> &dyn Any {
        value.0.as_any()
    }
    fn shared_type_id(value: &Self::Shared<'_>) -> TypeId {
        ErasedRef::type_id(value.as_ref())
    }
    fn mutable_type_id(value: &Self::Mutable<'_>) -> TypeId {
        ErasedMut::type_id(value.as_ref())
    }
    fn owned_into_any(value: Self::Owned) -> Box<dyn Any> {
        let ThreadSafeOwned(value) = value;
        let value: Box<dyn ErasedOwned> = value;
        value.into_any()
    }
    fn shared_from_owned<'a>(value: &'a Self::Owned) -> Self::Shared<'a> {
        value.0.shared_thread_safe()
    }
    fn shared_reborrow<'a, 'b>(value: &'b Self::Shared<'a>) -> Self::Shared<'b>
    where
        'a: 'b,
    {
        let view: Arc<dyn ErasedRef + Send + Sync + 'b> = value.clone();
        view
    }
    fn mutable_from_owned<'a>(value: &'a mut Self::Owned) -> Self::Mutable<'a> {
        value.0.mutable_thread_safe()
    }
    fn shared_from_mut<'a, 'b>(value: &'b Self::Mutable<'a>) -> Self::Shared<'b> {
        value.shared_view_thread_safe()
    }
    fn reborrow_mut<'a, 'b>(value: &'b mut Self::Mutable<'a>) -> Self::Mutable<'b> {
        value.reborrow_thread_safe()
    }
}

struct MutRefView<'a>(&'a (dyn ErasedMut + Send + Sync));
impl ErasedRef for MutRefView<'_> {
    fn type_id(&self) -> TypeId {
        ErasedMut::type_id(self.0)
    }
    fn holder_ptr(&self) -> *const () {
        ErasedMut::holder_ptr(self.0)
    }
    fn is_mutable(&self) -> bool {
        true
    }
}

pub(super) fn thread_safe_ref_into_local(value: Arc<dyn ErasedRef + Send + Sync + '_>) -> Rc<dyn ErasedRef + '_> {
    Rc::new(ArcRefView(value))
}

struct ArcRefView<'a>(Arc<dyn ErasedRef + Send + Sync + 'a>);
impl ErasedRef for ArcRefView<'_> {
    fn type_id(&self) -> TypeId {
        ErasedRef::type_id(self.0.as_ref())
    }
    fn holder_ptr(&self) -> *const () {
        self.0.holder_ptr()
    }
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
    fn local_mode_dispatches_type_identity_and_borrow_views() {
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

        let erased = Local::owned_into_any(owned);
        assert_eq!(*erased.downcast::<i32>().expect("owned type is preserved"), 41);
    }

    #[test]
    fn modes_expose_erased_any_views() {
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
    fn thread_safe_mode_dispatches_and_converts_shared_views() {
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

        let erased = ThreadSafe::owned_into_any(owned);
        assert_eq!(*erased.downcast::<i32>().expect("owned type is preserved"), 73);
    }
}
