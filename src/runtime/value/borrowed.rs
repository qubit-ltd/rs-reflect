// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use super::failure::DowncastFailure;
use super::mode::ErasedMut;
use super::mode::ErasedMutThreadSafe;
use super::mode::ErasedRef;
use super::mode::Local;
use super::mode::ThreadSafe;
use super::mode::ValueMode;
use super::mode::ValueModeInternal;
use super::mode::thread_safe_ref_into_local;
use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::Reflect;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::identity::TypeIdentity;

/// Function pointer that builds a reflected type descriptor for a context.
type DescriptorFactory = for<'ctx> fn(&'ctx ReflectionContext) -> TypeDescriptor<'ctx>;

/// Creates the type descriptor for `T` using the caller's reflection context.
///
/// # Type Parameters
///
/// - `T`: Reflected type represented by the erased borrow.
///
/// # Parameters
///
/// - `context`: Context in which the descriptor is resolved.
///
/// # Returns
///
/// Returns a descriptor tied to the supplied context.
fn descriptor_for<'ctx, T: Reflect + ?Sized>(context: &'ctx ReflectionContext) -> TypeDescriptor<'ctx> {
    TypeDescriptor::of_in::<T>(context)
}

/// Concrete holder behind a type-erased shared reference.
/// # Type Parameters
///
/// - `'a`: Lifetime of the borrowed input or view.
/// - `T`: Concrete value type represented by the operation.
struct RefHolder<'a, T: Reflect + ?Sized>(
    /// Original shared borrow retained for the holder's lifetime.
    &'a T,
);
impl<T: Reflect + ?Sized> ErasedRef for RefHolder<'_, T> {
    /// Returns the concrete type represented by the held borrow.
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
    /// Exposes the holder address for checked downcast implementations.
    fn holder_ptr(&self) -> *const () {
        self as *const Self as *const ()
    }
    /// Reports that the holder contains only shared access.
    fn is_mutable(&self) -> bool {
        false
    }
}

/// Erases a shared borrow into a local-mode reference holder.
///
/// # Type Parameters
///
/// - `T`: Reflected type of the borrowed value.
///
/// # Parameters
///
/// - `value`: Shared borrow retained by the returned holder.
///
/// # Returns
///
/// Returns an erased holder that cannot outlive `value`.
pub(super) fn erase_ref_local<'a, T: Reflect + ?Sized>(value: &'a T) -> std::rc::Rc<dyn ErasedRef + 'a> {
    std::rc::Rc::new(RefHolder(value))
}

/// Erases a shared borrow into a thread-safe reference holder.
///
/// # Type Parameters
///
/// - `T`: Reflected, `Sync` type of the borrowed value.
///
/// # Parameters
///
/// - `value`: Shared borrow retained by the returned holder.
///
/// # Returns
///
/// Returns an erased holder that is `Send + Sync` and cannot outlive `value`.
pub(super) fn erase_ref_thread_safe<'a, T: Reflect + Sync + ?Sized>(
    value: &'a T,
) -> std::sync::Arc<dyn ErasedRef + Send + Sync + 'a> {
    std::sync::Arc::new(RefHolder(value))
}

/// Concrete owner of an exclusive borrow during type erasure.
/// # Type Parameters
///
/// - `'a`: Lifetime of the borrowed input or view.
/// - `T`: Concrete value type represented by the operation.
struct MutHolder<'a, T: Reflect + ?Sized>(
    /// Original mutable borrow, taken only when it is recovered.
    Option<&'a mut T>,
);

impl<'a, T: Reflect + ?Sized> ErasedMut for MutHolder<'a, T> {
    /// Returns the concrete type represented by the mutable borrow.
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
    /// Returns the holder address used by checked erased downcasts.
    fn holder_ptr(&self) -> *const () {
        self as *const Self as *const ()
    }
    /// Returns the holder address while this wrapper has exclusive access.
    fn holder_ptr_mut(&mut self) -> *mut () {
        self as *mut Self as *mut ()
    }
    /// Creates a shared local-mode view without releasing the mutable holder.
    fn shared_view_local(&self) -> std::rc::Rc<dyn ErasedRef + '_> {
        std::rc::Rc::new(MutRefViewLocal(self))
    }
    /// Creates a shorter exclusive view while keeping the original borrow
    /// active.
    fn reborrow<'b>(&'b mut self) -> Box<dyn ErasedMut + 'b> {
        Box::new(BorrowedMutViewLocal(self))
    }
}

impl<T: Reflect + Send + Sync + ?Sized> ErasedMutThreadSafe for MutHolder<'_, T> {
    /// Creates a shared thread-safe view of the active mutable borrow.
    fn shared_view_thread_safe(&self) -> Arc<dyn ErasedRef + Send + Sync + '_> {
        Arc::new(MutRefViewThreadSafe(self))
    }
    /// Creates a shorter thread-safe exclusive view of the same holder.
    fn reborrow_thread_safe<'b>(&'b mut self) -> Box<dyn ErasedMutThreadSafe + 'b> {
        Box::new(BorrowedMutViewThreadSafe(self))
    }
}

/// Erases an exclusive borrow into a local-mode mutable holder.
///
/// # Type Parameters
///
/// - `T`: Reflected type of the borrowed value.
///
/// # Parameters
///
/// - `value`: Exclusive borrow retained by the returned holder.
///
/// # Returns
///
/// Returns a mutable erased holder bounded by the borrow lifetime.
pub(super) fn erase_mut_local<'a, T: Reflect + ?Sized>(value: &'a mut T) -> Box<dyn ErasedMut + 'a> {
    Box::new(MutHolder(Some(value)))
}

/// Erases an exclusive borrow into a thread-safe mutable holder.
///
/// # Type Parameters
///
/// - `T`: Reflected `Send + Sync` type of the borrowed value.
///
/// # Parameters
///
/// - `value`: Exclusive borrow retained by the returned holder.
///
/// # Returns
///
/// Returns a thread-safe erased holder bounded by the borrow lifetime.
pub(super) fn erase_mut_thread_safe<'a, T: Reflect + Send + Sync + ?Sized>(
    value: &'a mut T,
) -> Box<dyn ErasedMutThreadSafe + 'a> {
    Box::new(MutHolder(Some(value)))
}

impl<T: Reflect + ?Sized> ErasedRef for MutHolder<'_, T> {
    /// Returns the concrete type represented by the mutable holder.
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
    /// Returns the stable address of this holder for checked type erasure
    /// recovery.
    fn holder_ptr(&self) -> *const () {
        self as *const Self as *const ()
    }
    /// Reports whether the represented holder owns an exclusive borrow.
    fn is_mutable(&self) -> bool {
        true
    }
}

/// Shared view that forwards a local mutable holder without owning it.
struct MutRefViewLocal<'a>(
    /// Borrow of the erased mutable holder.
    &'a dyn ErasedMut,
);
impl ErasedRef for MutRefViewLocal<'_> {
    /// Forwards the concrete type ID from the mutable holder.
    fn type_id(&self) -> TypeId {
        ErasedMut::type_id(self.0)
    }
    /// Returns the stable address of this holder for checked type erasure
    /// recovery.
    fn holder_ptr(&self) -> *const () {
        ErasedMut::holder_ptr(self.0)
    }
    /// Reports whether the represented holder owns an exclusive borrow.
    fn is_mutable(&self) -> bool {
        true
    }
}

/// Shared thread-safe view that forwards a mutable holder without owning it.
struct MutRefViewThreadSafe<'a>(
    /// Borrow of the `Send + Sync` erased mutable holder.
    &'a (dyn ErasedMut + Send + Sync),
);
impl ErasedRef for MutRefViewThreadSafe<'_> {
    /// Forwards the concrete type ID from the mutable holder.
    fn type_id(&self) -> TypeId {
        ErasedMut::type_id(self.0)
    }
    /// Returns the stable address of this holder for checked type erasure
    /// recovery.
    fn holder_ptr(&self) -> *const () {
        ErasedMut::holder_ptr(self.0)
    }
    /// Reports whether the represented holder owns an exclusive borrow.
    fn is_mutable(&self) -> bool {
        true
    }
}

/// Local forwarding view that bounds a mutable holder to an active reborrow.
struct BorrowedMutViewLocal<'a>(
    /// Active mutable holder borrowed for this forwarding view's lifetime.
    &'a mut dyn ErasedMut,
);
impl ErasedMut for BorrowedMutViewLocal<'_> {
    /// Returns the concrete type identity recorded by the underlying holder.
    fn type_id(&self) -> TypeId {
        ErasedMut::type_id(self.0)
    }
    /// Returns the stable address of this holder for checked type erasure
    /// recovery.
    fn holder_ptr(&self) -> *const () {
        ErasedMut::holder_ptr(self.0)
    }
    /// Returns the holder address while this wrapper is exclusively borrowed.
    fn holder_ptr_mut(&mut self) -> *mut () {
        ErasedMut::holder_ptr_mut(self.0)
    }
    /// Creates a local shared view that still reflects the active exclusive
    /// borrow.
    fn shared_view_local(&self) -> Rc<dyn ErasedRef + '_> {
        self.0.shared_view_local()
    }
    /// Creates a shorter exclusive view while keeping the original borrow
    /// active.
    fn reborrow<'b>(&'b mut self) -> Box<dyn ErasedMut + 'b> {
        Box::new(BorrowedMutViewLocal(&mut *self.0))
    }
}

/// Thread-safe forwarding view that bounds a mutable holder to an active
/// reborrow.
struct BorrowedMutViewThreadSafe<'a>(
    /// Active thread-safe holder borrowed for this forwarding view's lifetime.
    &'a mut (dyn ErasedMut + Send + Sync),
);
impl ErasedMut for BorrowedMutViewThreadSafe<'_> {
    /// Returns the concrete type identity recorded by the underlying holder.
    fn type_id(&self) -> TypeId {
        ErasedMut::type_id(self.0)
    }
    /// Returns the stable address of this holder for checked type erasure
    /// recovery.
    fn holder_ptr(&self) -> *const () {
        ErasedMut::holder_ptr(self.0)
    }
    /// Returns the holder address while this wrapper is exclusively borrowed.
    fn holder_ptr_mut(&mut self) -> *mut () {
        ErasedMut::holder_ptr_mut(self.0)
    }
    /// Creates a local shared view that still reflects the active exclusive
    /// borrow.
    fn shared_view_local(&self) -> Rc<dyn ErasedRef + '_> {
        self.0.shared_view_local()
    }
    /// Creates a shorter exclusive view while keeping the original borrow
    /// active.
    fn reborrow<'b>(&'b mut self) -> Box<dyn ErasedMut + 'b> {
        Box::new(BorrowedMutViewLocal(&mut *self.0))
    }
}

impl ErasedMutThreadSafe for BorrowedMutViewThreadSafe<'_> {
    /// Creates a shared thread-safe view without ending the exclusive reborrow
    /// chain.
    fn shared_view_thread_safe(&self) -> Arc<dyn ErasedRef + Send + Sync + '_> {
        Arc::new(MutRefViewThreadSafe(&*self.0))
    }
    /// Creates a shorter thread-safe exclusive view of the same holder.
    fn reborrow_thread_safe<'b>(&'b mut self) -> Box<dyn ErasedMutThreadSafe + 'b> {
        Box::new(BorrowedMutViewThreadSafe(&mut *self.0))
    }
}

/// A reflected type that can be erased as a shared borrow in mode `M`.
///
/// Implementations are available for reflected types; thread-safe mode also
/// requires `Sync`.
/// # Type Parameters
///
/// - `M`: Value mode that governs storage and thread-safety bounds.
#[allow(private_bounds)]
pub trait ValueForRef<M: ValueMode>: ValueForRefInternal<M> + Reflect {
    // empty
}

/// Mode-specific contract for erasing and recovering reflected shared borrows.
/// # Type Parameters
///
/// - `M`: Value mode that governs storage and thread-safety bounds.
pub(crate) trait ValueForRefInternal<M: ValueMode>: Reflect {
    /// Stores a shared borrow behind the mode-specific erased reference type.
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    fn erase_ref<'a>(value: &'a Self) -> M::Shared<'a>;
    /// Checks the concrete type identity and returns a borrow when it matches.
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    fn downcast_ref<'a, 'b>(value: &'b M::Shared<'a>) -> Option<&'b Self>;
    /// Checks the concrete type identity before exposing shared access through
    /// a mutable holder.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    fn downcast_mut_ref<'a, 'b>(value: &'b M::Mutable<'a>) -> Option<&'b Self>;
    /// Consumes the erased shared holder and recovers the borrow, returning the
    /// holder on a type mismatch.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    fn recover_ref<'a>(value: M::Shared<'a>) -> Result<&'a Self, M::Shared<'a>>
    where
        Self: 'a;
    /// Returns the type identity used to validate erased borrow recovery.
    fn identity() -> TypeIdentity;
}

impl<T: Reflect + ?Sized> ValueForRefInternal<Local> for T {
    /// Stores a shared borrow behind the mode-specific erased reference type.
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    fn erase_ref<'a>(value: &'a Self) -> Rc<dyn ErasedRef + 'a> {
        Rc::new(RefHolder(value))
    }
    /// Checks the concrete type identity and returns a borrow when it matches.
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    fn downcast_ref<'a, 'b>(value: &'b Rc<dyn ErasedRef + 'a>) -> Option<&'b Self> {
        if value.type_id() != TypeId::of::<Self>() {
            return None;
        }
        // SAFETY: `type_id` is checked first. `holder_ptr` is produced by
        // `RefHolder<T>` or forwarded by a mutable view, and `is_mutable`
        // selects the matching layout.
        unsafe {
            if value.is_mutable() {
                (*(value.holder_ptr() as *const MutHolder<'a, Self>)).0.as_deref()
            } else {
                Some((*(value.holder_ptr() as *const RefHolder<'a, Self>)).0)
            }
        }
    }
    /// Checks the concrete type identity before exposing shared access through
    /// a mutable holder.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    fn downcast_mut_ref<'a, 'b>(value: &'b Box<dyn ErasedMut + 'a>) -> Option<&'b Self> {
        if value.type_id() != TypeId::of::<Self>() {
            return None;
        }
        // SAFETY: only `MutHolder<T>` and its forwarding views report this type ID;
        // the pointer is forwarded unchanged and therefore has the matching layout.
        unsafe { (*(value.holder_ptr() as *const MutHolder<'a, Self>)).0.as_deref() }
    }
    /// Consumes the erased shared holder and recovers the borrow, returning the
    /// holder on a type mismatch.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    fn recover_ref<'a>(value: Rc<dyn ErasedRef + 'a>) -> Result<&'a Self, Rc<dyn ErasedRef + 'a>>
    where
        Self: 'a,
    {
        if value.type_id() != TypeId::of::<Self>() {
            return Err(value);
        }
        // SAFETY: as above, the type ID and mutable-kind discriminator establish the
        // concrete holder layout. The erased object lifetime is exactly `'a`.
        Ok(unsafe {
            if value.is_mutable() {
                (*(value.holder_ptr() as *const MutHolder<'a, Self>))
                    .0
                    .as_deref()
                    .expect("mutable holder always contains its original borrow")
            } else {
                (*(value.holder_ptr() as *const RefHolder<'a, Self>)).0
            }
        })
    }
    /// Returns the type identity used to validate erased borrow recovery.
    fn identity() -> TypeIdentity {
        type_identity::<Self>()
    }
}

impl<T: Reflect + Sync + ?Sized> ValueForRefInternal<ThreadSafe> for T {
    /// Stores a shared borrow behind the mode-specific erased reference type.
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    fn erase_ref<'a>(value: &'a Self) -> Arc<dyn ErasedRef + Send + Sync + 'a> {
        Arc::new(RefHolder(value))
    }
    /// Checks the concrete type identity and returns a borrow when it matches.
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    fn downcast_ref<'a, 'b>(value: &'b Arc<dyn ErasedRef + Send + Sync + 'a>) -> Option<&'b Self> {
        if value.type_id() != TypeId::of::<Self>() {
            return None;
        }
        // SAFETY: `type_id` is checked first. `holder_ptr` is produced by
        // `RefHolder<T>` or forwarded by a mutable view, and `is_mutable`
        // selects the matching layout.
        unsafe {
            if value.is_mutable() {
                (*(value.holder_ptr() as *const MutHolder<'a, Self>)).0.as_deref()
            } else {
                Some((*(value.holder_ptr() as *const RefHolder<'a, Self>)).0)
            }
        }
    }
    /// Checks the concrete type identity before exposing shared access through
    /// a mutable holder.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    fn downcast_mut_ref<'a, 'b>(value: &'b Box<dyn ErasedMutThreadSafe + 'a>) -> Option<&'b Self> {
        if value.type_id() != TypeId::of::<Self>() {
            return None;
        }
        // SAFETY: only `MutHolder<T>` and its forwarding views report this type ID;
        // the pointer is forwarded unchanged and therefore has the matching layout.
        unsafe { (*(value.holder_ptr() as *const MutHolder<'a, Self>)).0.as_deref() }
    }
    /// Consumes the erased shared holder and recovers the borrow, returning the
    /// holder on a type mismatch.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    fn recover_ref<'a>(
        value: Arc<dyn ErasedRef + Send + Sync + 'a>,
    ) -> Result<&'a Self, Arc<dyn ErasedRef + Send + Sync + 'a>>
    where
        Self: 'a,
    {
        if value.type_id() != TypeId::of::<Self>() {
            return Err(value);
        }
        // SAFETY: as above, the type ID and mutable-kind discriminator establish the
        // concrete holder layout. The erased object lifetime is exactly `'a`.
        Ok(unsafe {
            if value.is_mutable() {
                (*(value.holder_ptr() as *const MutHolder<'a, Self>))
                    .0
                    .as_deref()
                    .expect("mutable holder always contains its original borrow")
            } else {
                (*(value.holder_ptr() as *const RefHolder<'a, Self>)).0
            }
        })
    }
    /// Returns the type identity used to validate erased borrow recovery.
    fn identity() -> TypeIdentity {
        type_identity::<Self>()
    }
}

/// A reflected type that can be erased as an exclusive borrow in mode `M`.
///
/// Implementations are available for reflected types; thread-safe mode also
/// requires `Send + Sync`.
/// # Type Parameters
///
/// - `M`: Value mode that governs storage and thread-safety bounds.
#[allow(private_bounds)]
pub trait ValueForMut<M: ValueMode>: ValueForMutInternal<M> + Reflect {
    // empty
}

/// Mode-specific contract for erasing and recovering reflected exclusive
/// borrows.
///
/// # Type Parameters
///
/// - `M`: Value mode that governs storage and thread-safety bounds.
pub(crate) trait ValueForMutInternal<M: ValueMode>: Reflect {
    /// Stores the exclusive borrow behind the mode-specific erased mutable
    /// type.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    fn erase_mut<'a>(value: &'a mut Self) -> M::Mutable<'a>;
    /// Checks the concrete type identity and returns exclusive access when it
    /// matches.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    fn downcast_mut<'b, 'a>(value: &'b mut M::Mutable<'a>) -> Option<&'b mut Self>;
    /// Consumes the erased mutable holder and recovers its borrow, returning
    /// the holder on a type mismatch.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    fn recover_mut<'a>(value: M::Mutable<'a>) -> Result<&'a mut Self, M::Mutable<'a>>
    where
        Self: 'a;
    /// Returns the type identity used to validate erased borrow recovery.
    fn identity() -> TypeIdentity;
}

impl<T: Reflect + ?Sized> ValueForMutInternal<Local> for T {
    /// Stores the exclusive borrow behind the mode-specific erased mutable
    /// type.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    fn erase_mut<'a>(value: &'a mut Self) -> Box<dyn ErasedMut + 'a> {
        Box::new(MutHolder(Some(value)))
    }
    /// Checks the concrete type identity and returns exclusive access when it
    /// matches.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    fn downcast_mut<'b, 'a>(value: &'b mut Box<dyn ErasedMut + 'a>) -> Option<&'b mut Self> {
        if value.type_id() != TypeId::of::<Self>() {
            return None;
        }
        // SAFETY: only `MutHolder<T>` and its forwarding views report this type ID;
        // the pointer is forwarded unchanged and therefore has the matching layout.
        unsafe { (*(value.holder_ptr_mut() as *mut MutHolder<'a, Self>)).0.as_deref_mut() }
    }
    /// Consumes the erased mutable holder and recovers its borrow, returning
    /// the holder on a type mismatch.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    fn recover_mut<'a>(mut value: Box<dyn ErasedMut + 'a>) -> Result<&'a mut Self, Box<dyn ErasedMut + 'a>>
    where
        Self: 'a,
    {
        if value.type_id() != TypeId::of::<Self>() {
            return Err(value);
        }
        // SAFETY: the type ID is checked and `holder_ptr_mut` forwards the concrete
        // `MutHolder<T>` address; exclusive access to `value` preserves aliasing rules.
        // SAFETY: the private `ErasedMut` implementations report a type ID only
        // for a matching `MutHolder<T>` and forward this pointer only through an
        // active exclusive reborrow chain. The erased object's lifetime `'a` is
        // bounded by the borrow stored in that holder.
        let value_ptr = unsafe {
            let holder = &mut *(value.holder_ptr_mut() as *mut MutHolder<'a, Self>);
            holder.0.as_deref_mut().map(|value| value as *mut Self)
        };
        let Some(value_ptr) = value_ptr else {
            return Err(value);
        };
        // SAFETY: the pointer came from the unique mutable borrow stored in the
        // matching holder. `value` owns that holder or carries an active exclusive
        // reborrow of it; consuming the erased view ends this link in the reborrow
        // chain. The erased lifetime `'a` is bounded by the original borrow, so
        // recreating the reference for `'a` transfers the same exclusive borrow.
        drop(value);
        Ok(unsafe { &mut *value_ptr })
    }
    /// Returns the type identity used to validate erased borrow recovery.
    fn identity() -> TypeIdentity {
        type_identity::<Self>()
    }
}

impl<T: Reflect + Send + Sync + ?Sized> ValueForMutInternal<ThreadSafe> for T {
    /// Stores the exclusive borrow behind the mode-specific erased mutable
    /// type.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    fn erase_mut<'a>(value: &'a mut Self) -> Box<dyn ErasedMutThreadSafe + 'a> {
        Box::new(MutHolder(Some(value)))
    }
    /// Checks the concrete type identity and returns exclusive access when it
    /// matches.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    fn downcast_mut<'b, 'a>(value: &'b mut Box<dyn ErasedMutThreadSafe + 'a>) -> Option<&'b mut Self> {
        if value.type_id() != TypeId::of::<Self>() {
            return None;
        }
        // SAFETY: only `MutHolder<T>` and its forwarding views report this type ID;
        // the pointer is forwarded unchanged and therefore has the matching layout.
        unsafe { (*(value.holder_ptr_mut() as *mut MutHolder<'a, Self>)).0.as_deref_mut() }
    }
    /// Consumes the erased mutable holder and recovers its borrow, returning
    /// the holder on a type mismatch.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    fn recover_mut<'a>(
        mut value: Box<dyn ErasedMutThreadSafe + 'a>,
    ) -> Result<&'a mut Self, Box<dyn ErasedMutThreadSafe + 'a>>
    where
        Self: 'a,
    {
        if value.type_id() != TypeId::of::<Self>() {
            return Err(value);
        }
        // SAFETY: the type ID is checked and `holder_ptr_mut` forwards the concrete
        // `MutHolder<T>` address; exclusive access to `value` preserves aliasing rules.
        // SAFETY: the private `ErasedMut` implementations report a type ID only
        // for a matching `MutHolder<T>` and forward this pointer only through an
        // active exclusive reborrow chain. The erased object's lifetime `'a` is
        // bounded by the borrow stored in that holder.
        let value_ptr = unsafe {
            let holder = &mut *(value.holder_ptr_mut() as *mut MutHolder<'a, Self>);
            holder.0.as_deref_mut().map(|value| value as *mut Self)
        };
        let Some(value_ptr) = value_ptr else {
            return Err(value);
        };
        // SAFETY: the pointer came from the unique mutable borrow stored in the
        // matching holder. `value` owns that holder or carries an active exclusive
        // reborrow of it; consuming the erased view ends this link in the reborrow
        // chain. The erased lifetime `'a` is bounded by the original borrow, so
        // recreating the reference for `'a` transfers the same exclusive borrow.
        drop(value);
        Ok(unsafe { &mut *value_ptr })
    }
    /// Returns the type identity used to validate erased borrow recovery.
    fn identity() -> TypeIdentity {
        type_identity::<Self>()
    }
}

/// Builds the runtime identity used for checked erased downcasts.
/// # Type Parameters
///
/// - `T`: Concrete value type represented by the operation.
fn type_identity<T: Reflect + ?Sized>() -> TypeIdentity {
    TypeIdentity {
        type_id: TypeId::of::<T>(),
        rust_name: std::any::type_name::<T>(),
    }
}

/// A shared borrow with its concrete reflected type erased.
///
/// The view cannot outlive the borrowed value. Cloning it creates another
/// shared view of the same value.
/// # Type Parameters
///
/// - `'a`: Lifetime of the borrowed input or view.
/// - `M`: Value mode that governs storage and thread-safety bounds.
pub struct DynamicRef<'a, M: ValueMode = Local> {
    /// Mode-specific erased holder retaining the original borrow.
    value: M::Shared<'a>,
    /// Optional borrow of the concrete value for callers requesting `Any`
    /// access.
    any_value: Option<&'a M::AnyObject>,
    /// Returns the type identity used to validate erased borrow recovery.
    identity: TypeIdentity,
    /// Function that resolves this value’s type descriptor in a reflection
    /// context.
    descriptor_factory: DescriptorFactory,
    /// Carries the value mode in the type without storing an additional runtime
    /// value.
    _mode: PhantomData<M>,
}

impl<'a, M: ValueMode> DynamicRef<'a, M> {
    /// Erases a shared borrow while preserving its concrete type identity.
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete value type represented by the operation.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub fn new<T: ValueForRef<M> + ?Sized>(value: &'a T) -> Self {
        Self::new_with_any(value, None)
    }
    /// Creates an erased shared view and records an optional `Any` reference to
    /// the actual value. # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    /// - `any_value`: Optional `Any` view of the underlying value, borrowed for
    ///   the input lifetime.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete value type represented by the operation.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    /// - `any_value`: Optional `Any` view of the underlying value, borrowed for
    ///   the input lifetime.
    fn new_with_any<T: ValueForRef<M> + ?Sized>(value: &'a T, any_value: Option<&'a M::AnyObject>) -> Self {
        Self {
            value: T::erase_ref(value),
            any_value,
            identity: T::identity(),
            descriptor_factory: descriptor_for::<T>,
            _mode: PhantomData,
        }
    }
    /// Returns the actual borrowed value as `Any` when this view retained one.
    ///
    /// Views created by [`Self::new`] return `None`. The returned reference is
    /// tied to this view and never points to the internal erasure holder.
    pub fn as_any(&self) -> Option<&dyn std::any::Any> {
        self.any_value.map(M::any_object_as_any)
    }
    /// Returns the concrete Rust `TypeId` represented by this view.
    pub fn type_id(self) -> TypeId {
        self.identity.type_id
    }
    /// Describes the represented type using the global reflection context.
    pub fn ty(self) -> TypeDescriptor<'static> {
        self.ty_in(ReflectionContext::global())
    }
    /// Describes the represented type using `context`.
    ///
    /// The descriptor is tied to the context's lifetime.
    /// # Parameters
    ///
    /// - `context`: Reflection context used to resolve this descriptor’s source
    ///   expressions.
    pub fn ty_in<'ctx>(self, context: &'ctx ReflectionContext) -> TypeDescriptor<'ctx> {
        (self.descriptor_factory)(context)
    }
    /// Returns whether the represented concrete type is `T`.
    /// # Type Parameters
    ///
    /// - `T`: Concrete value type represented by the operation.
    pub fn is<T: ValueForRef<M> + ?Sized>(self) -> bool {
        self.type_id() == T::identity().type_id
    }
    /// Borrows the value as `T`, or returns `None` when the type differs.
    /// # Type Parameters
    ///
    /// - `T`: Concrete value type represented by the operation.
    pub fn downcast_ref<T: ValueForRef<M> + ?Sized>(&self) -> Option<&T> {
        T::downcast_ref(&self.value)
    }
    /// Creates a shorter-lived shared view of the same value.
    pub fn reborrow<'b>(&'b self) -> DynamicRef<'b, M>
    where
        'a: 'b,
    {
        DynamicRef::from_erased(
            M::shared_reborrow(&self.value),
            self.any_value,
            self.identity,
            self.descriptor_factory,
        )
    }
    /// Recovers the original borrow as `T`.
    ///
    /// On a type mismatch, the error retains this dynamic view and identifies
    /// both the expected and actual Rust types.
    /// # Type Parameters
    ///
    /// - `T`: Concrete value type represented by the operation.
    pub fn downcast<T: ValueForRef<M> + ?Sized>(self) -> Result<&'a T, DowncastFailure<Self>> {
        let expected = T::identity();
        if expected.type_id != self.identity.type_id {
            return Err(DowncastFailure::new(expected, self.identity, self));
        }
        match T::recover_ref(self.value) {
            Ok(value) => Ok(value),
            Err(value) => Err(DowncastFailure::new(
                expected,
                self.identity,
                Self {
                    value,
                    any_value: self.any_value,
                    identity: self.identity,
                    descriptor_factory: self.descriptor_factory,
                    _mode: PhantomData,
                },
            )),
        }
    }
    /// Constructs a typed view from an already validated erased holder and its
    /// metadata.
    ///
    /// # Parameters
    ///
    /// - `any_value`: Optional `Any` view of the underlying value, borrowed for
    ///   the input lifetime.
    /// - `identity`: Concrete type identity checked before downcast or
    ///   recovery.
    /// - `descriptor_factory`: Function that resolves the represented type in a
    ///   supplied reflection context.
    pub(super) fn from_erased(
        value: M::Shared<'a>,
        any_value: Option<&'a M::AnyObject>,
        identity: TypeIdentity,
        descriptor_factory: DescriptorFactory,
    ) -> Self {
        Self {
            value,
            any_value,
            identity,
            descriptor_factory,
            _mode: PhantomData,
        }
    }
}

impl<'a, M: ValueMode> Clone for DynamicRef<'a, M>
where
    M::Shared<'a>: Clone,
{
    /// Clones the shared holder while preserving type metadata and the original
    /// borrow lifetime.
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            any_value: self.any_value,
            identity: self.identity,
            descriptor_factory: self.descriptor_factory,
            _mode: PhantomData,
        }
    }
}

impl<'a> DynamicRef<'a, ThreadSafe> {
    /// Erases a shared borrow and retains a thread-safe `Any` view.
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete value type represented by the operation.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub fn new_any<T: ValueForRef<ThreadSafe> + std::any::Any + Sync>(value: &'a T) -> Self {
        Self::new_with_any(value, Some(value))
    }

    /// Converts the thread-safe erased holder into a local-mode holder without
    /// changing the borrow.
    pub(crate) fn into_local(self) -> DynamicRef<'a, Local> {
        let value = thread_safe_ref_into_local(self.value);
        DynamicRef {
            value,
            any_value: self.any_value.map(ThreadSafe::any_object_as_any),
            identity: self.identity,
            descriptor_factory: self.descriptor_factory,
            _mode: PhantomData,
        }
    }
}
impl<'a> DynamicRef<'a, Local> {
    /// Erases a shared borrow and retains a local `Any` view of its actual
    /// value.
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete value type represented by the operation.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub fn new_any<T: ValueForRef<Local> + std::any::Any>(value: &'a T) -> Self {
        Self::new_with_any(value, Some(value))
    }

    /// Converts the thread-safe erased holder into a local-mode holder without
    /// changing the borrow.
    pub(crate) fn into_local(self) -> Self {
        self
    }
}

/// An exclusive borrow with its concrete reflected type erased.
///
/// The view cannot outlive the borrowed value and can be reborrowed for a
/// shorter lifetime.
/// # Type Parameters
///
/// - `'a`: Lifetime of the borrowed input or view.
/// - `M`: Value mode that governs storage and thread-safety bounds.
pub struct DynamicMut<'a, M: ValueMode = Local> {
    /// Mode-specific erased holder retaining the original borrow.
    value: M::Mutable<'a>,
    /// Returns the type identity used to validate erased borrow recovery.
    identity: TypeIdentity,
    /// Function that resolves this value’s type descriptor in a reflection
    /// context.
    descriptor_factory: DescriptorFactory,
    /// Carries the value mode in the type without storing an additional runtime
    /// value.
    _mode: PhantomData<M::Marker>,
}

impl<'a, M: ValueMode> DynamicMut<'a, M> {
    /// Erases an exclusive borrow while preserving its concrete type identity.
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete value type represented by the operation.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub fn new<T: ValueForMut<M> + ?Sized>(value: &'a mut T) -> Self {
        Self {
            value: T::erase_mut(value),
            identity: T::identity(),
            descriptor_factory: descriptor_for::<T>,
            _mode: PhantomData,
        }
    }
    /// Returns the concrete Rust `TypeId` represented by this view.
    pub fn type_id(&self) -> TypeId {
        self.identity.type_id
    }
    /// Describes the represented type using the global reflection context.
    pub fn ty(&self) -> TypeDescriptor<'static> {
        self.ty_in(ReflectionContext::global())
    }
    /// Describes the represented type using `context`.
    ///
    /// The descriptor is tied to the context's lifetime.
    /// # Parameters
    ///
    /// - `context`: Reflection context used to resolve this descriptor’s source
    ///   expressions.
    pub fn ty_in<'ctx>(&self, context: &'ctx ReflectionContext) -> TypeDescriptor<'ctx> {
        (self.descriptor_factory)(context)
    }
    /// Returns whether the represented concrete type is `T`.
    /// # Type Parameters
    ///
    /// - `T`: Concrete value type represented by the operation.
    pub fn is<T: ValueForMut<M> + ?Sized>(&self) -> bool {
        self.type_id() == T::identity().type_id
    }
    /// Borrows the value as `T`, or returns `None` when the type differs.
    /// # Type Parameters
    ///
    /// - `T`: Concrete value type represented by the operation.
    pub fn downcast_ref<T: ValueForRef<M> + ?Sized>(&self) -> Option<&T> {
        T::downcast_mut_ref(&self.value)
    }
    /// Mutably borrows the value as `T`, or returns `None` when the type
    /// differs.
    /// # Type Parameters
    ///
    /// - `T`: Concrete value type represented by the operation.
    pub fn downcast_mut<T: ValueForMut<M> + ?Sized>(&mut self) -> Option<&mut T> {
        T::downcast_mut(&mut self.value)
    }
    /// Recovers the exclusive borrow as `T`.
    ///
    /// On a type mismatch, the error retains this dynamic view and identifies
    /// both the expected and actual Rust types.
    /// # Type Parameters
    ///
    /// - `T`: Concrete value type represented by the operation.
    pub fn downcast<T: ValueForMut<M> + ?Sized>(self) -> Result<&'a mut T, DowncastFailure<Self>> {
        let expected = T::identity();
        if expected.type_id != self.identity.type_id {
            return Err(DowncastFailure::new(expected, self.identity, self));
        }
        match T::recover_mut(self.value) {
            Ok(value) => Ok(value),
            Err(value) => Err(DowncastFailure::new(
                expected,
                self.identity,
                Self {
                    value,
                    identity: self.identity,
                    descriptor_factory: self.descriptor_factory,
                    _mode: PhantomData,
                },
            )),
        }
    }
    /// Creates a shared view of this borrow, tied to the current borrow of
    /// `self`.
    pub fn as_ref(&self) -> DynamicRef<'_, M> {
        DynamicRef::from_erased(
            M::shared_from_mut(&self.value),
            None,
            self.identity,
            self.descriptor_factory,
        )
    }
    /// Creates a shorter-lived exclusive view of the same value.
    pub fn reborrow(&mut self) -> DynamicMut<'_, M> {
        DynamicMut {
            value: M::reborrow_mut(&mut self.value),
            identity: self.identity,
            descriptor_factory: self.descriptor_factory,
            _mode: PhantomData,
        }
    }
    /// Constructs a typed view from an already validated erased holder and its
    /// metadata.
    ///
    /// # Parameters
    ///
    /// - `identity`: Concrete type identity checked before downcast or
    ///   recovery.
    /// - `descriptor_factory`: Function that resolves the represented type in a
    ///   supplied reflection context.
    pub(super) fn from_erased(
        value: M::Mutable<'a>,
        identity: TypeIdentity,
        descriptor_factory: DescriptorFactory,
    ) -> Self {
        Self {
            value,
            identity,
            descriptor_factory,
            _mode: PhantomData,
        }
    }
}

impl<'a> DynamicMut<'a, ThreadSafe> {
    /// Converts the thread-safe erased holder into a local-mode holder without
    /// changing the borrow.
    pub(crate) fn into_local(self) -> DynamicMut<'a, Local> {
        let value: Box<dyn ErasedMut + 'a> = self.value;
        DynamicMut {
            value,
            identity: self.identity,
            descriptor_factory: self.descriptor_factory,
            _mode: PhantomData,
        }
    }
}
impl<'a> DynamicMut<'a, Local> {
    /// Converts the thread-safe erased holder into a local-mode holder without
    /// changing the borrow.
    pub(crate) fn into_local(self) -> Self {
        self
    }
}

impl<T: Reflect + ?Sized> ValueForRef<Local> for T {}
impl<T: Reflect + Sync + ?Sized> ValueForRef<ThreadSafe> for T {}
impl<T: Reflect + ?Sized> ValueForMut<Local> for T {}
impl<T: Reflect + Send + Sync + ?Sized> ValueForMut<ThreadSafe> for T {}

#[cfg(test)]
mod tests {
    use super::DynamicMut;
    use super::DynamicRef;
    use crate::runtime::value::Local;
    use crate::runtime::value::ThreadSafe;

    #[test]
    fn test_local_shared_views_clone_reborrow_and_recover_after_failed_downcast() {
        let text = String::from("borrowed");
        let dynamic = DynamicRef::<Local>::new(&text);
        let clone = dynamic.clone();
        assert!(clone.clone().is::<String>());
        assert_eq!(clone.downcast_ref::<String>().map(String::as_str), Some("borrowed"));

        {
            let reborrowed = dynamic.reborrow();
            assert_eq!(
                reborrowed.downcast_ref::<String>().map(String::as_str),
                Some("borrowed")
            );
            assert!(reborrowed.downcast_ref::<u64>().is_none());
        }

        let failure = dynamic.downcast::<u64>().expect_err("a shared String is not a u64");
        assert_eq!(failure.expected(), std::any::TypeId::of::<u64>());
        assert_eq!(failure.actual(), std::any::TypeId::of::<String>());
        assert_eq!(
            failure
                .into_value()
                .downcast::<String>()
                .expect("failure retains the original borrow"),
            "borrowed"
        );
    }

    #[test]
    fn test_local_mutable_views_expose_shared_access_and_recover_after_failed_downcast() {
        let mut text = String::from("before");
        let dynamic = DynamicMut::<Local>::new(&mut text);
        assert_eq!(dynamic.downcast_ref::<String>().map(String::as_str), Some("before"));
        assert!(dynamic.downcast_ref::<u64>().is_none());

        {
            let shared = dynamic.as_ref();
            let clone = shared.clone();
            assert_eq!(clone.downcast::<String>().expect("shared view matches"), "before");
        }

        let failure = dynamic.downcast::<u64>().expect_err("a mutable String is not a u64");
        assert_eq!(failure.expected(), std::any::TypeId::of::<u64>());
        assert_eq!(failure.actual(), std::any::TypeId::of::<String>());
        failure
            .into_value()
            .downcast::<String>()
            .expect("failure retains the original exclusive borrow")
            .push_str(" after recovery");
        assert_eq!(text, "before after recovery");
    }

    #[test]
    fn test_local_mutable_reborrows_can_be_nested_and_recovered() {
        let mut values = [2_u32, 3, 5];
        let mut outer = DynamicMut::<Local>::new(&mut values[..]);
        {
            let mut middle = outer.reborrow();
            {
                let mut inner = middle.reborrow();
                inner
                    .downcast_mut::<[u32]>()
                    .expect("reborrow preserves the slice type")[1] = 7;
            }
            assert_eq!(
                &*middle.downcast::<[u32]>().expect("middle reborrow recovers"),
                &[2, 7, 5]
            );
        }
        assert_eq!(&*outer.downcast::<[u32]>().expect("outer borrow recovers"), &[2, 7, 5]);
    }

    #[test]
    fn test_local_reborrow_downcast_failure_recovers_and_keeps_the_parent_usable() {
        let mut value = String::from("local");
        let mut parent = DynamicMut::<Local>::new(&mut value);
        {
            let child = parent.reborrow();
            let failure = child.downcast::<u64>().expect_err("the reborrowed String is not a u64");
            assert_eq!(failure.expected(), std::any::TypeId::of::<u64>());
            assert_eq!(failure.actual(), std::any::TypeId::of::<String>());
            failure
                .into_value()
                .downcast::<String>()
                .expect("the failed child view retains the mutable borrow")
                .push_str(" child");
        }
        parent
            .downcast_mut::<String>()
            .expect("the parent view remains usable after child recovery")
            .push_str(" parent");
        assert_eq!(
            parent
                .downcast::<String>()
                .expect("the parent view recovers after the child ends"),
            "local child parent"
        );
    }

    #[test]
    fn test_thread_safe_borrowed_views_clone_reborrow_and_downgrade_to_local() {
        let text = String::from("thread safe");
        let shared = DynamicRef::<ThreadSafe>::new(&text);
        let clone = shared.clone();
        assert_eq!(
            clone.downcast::<String>().expect("clone preserves its type"),
            "thread safe"
        );
        assert_eq!(
            shared.reborrow().downcast_ref::<String>().map(String::as_str),
            Some("thread safe")
        );
        assert_eq!(
            shared
                .into_local()
                .downcast::<String>()
                .expect("downgraded view remains valid"),
            "thread safe"
        );

        let mut text = String::from("mutable");
        let mut mutable = DynamicMut::<ThreadSafe>::new(&mut text);
        {
            let reborrowed = mutable.reborrow();
            reborrowed
                .downcast::<String>()
                .expect("thread-safe reborrow preserves its type")
                .push_str(" view");
        }
        mutable
            .into_local()
            .downcast::<String>()
            .expect("downgraded mutable view recovers")
            .push('!');
        assert_eq!(text, "mutable view!");
    }

    #[test]
    fn test_thread_safe_reborrow_downcast_failure_recovers_and_keeps_the_parent_usable() {
        let mut value = String::from("thread safe");
        let mut parent = DynamicMut::<ThreadSafe>::new(&mut value);
        {
            let child = parent.reborrow();
            let failure = child.downcast::<u64>().expect_err("the reborrowed String is not a u64");
            assert_eq!(failure.expected(), std::any::TypeId::of::<u64>());
            assert_eq!(failure.actual(), std::any::TypeId::of::<String>());
            failure
                .into_value()
                .downcast::<String>()
                .expect("the failed child view retains the mutable borrow")
                .push_str(" child");
        }
        parent
            .downcast_mut::<String>()
            .expect("the parent view remains usable after child recovery")
            .push_str(" parent");
        assert_eq!(
            parent
                .downcast::<String>()
                .expect("the parent view recovers after the child ends"),
            "thread safe child parent"
        );
    }

    #[test]
    fn test_thread_safe_mutable_reborrows_can_be_nested_and_recovered() {
        let mut values = [11_u32, 13, 17];
        let mut outer = DynamicMut::<ThreadSafe>::new(&mut values[..]);
        {
            let mut middle = outer.reborrow();
            {
                let inner = middle.reborrow();
                inner
                    .downcast::<[u32]>()
                    .expect("the innermost view recovers the slice")[1] = 19;
            }
            middle
                .downcast::<[u32]>()
                .expect("the middle view remains usable after inner recovery")[2] = 23;
        }
        assert_eq!(
            &*outer
                .downcast::<[u32]>()
                .expect("the outer view remains usable after nested recovery"),
            &[11, 19, 23]
        );
    }

    #[test]
    fn test_local_mode_keeps_non_thread_safe_shared_and_mutable_borrows() {
        use std::rc::Rc;

        let value = Rc::new(String::from("local only"));
        let shared = DynamicRef::<Local>::new(&value);
        assert_eq!(shared.downcast::<Rc<String>>().expect("Local supports Rc"), &value);

        let mut value = Rc::new(String::from("mutable local"));
        let mut dynamic = DynamicMut::<Local>::new(&mut value);
        *dynamic
            .downcast_mut::<Rc<String>>()
            .expect("Local mutable view supports Rc") = Rc::new(String::from("mutable local value"));
        assert_eq!(
            dynamic
                .downcast::<Rc<String>>()
                .expect("Local mutable borrow recovers")
                .as_str(),
            "mutable local value"
        );
    }
}
