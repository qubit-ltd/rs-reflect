// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::Any;
use std::any::TypeId;
use std::marker::PhantomData;

use super::borrowed::DynamicMut;
use super::borrowed::DynamicRef;
use super::borrowed::erase_mut_local;
use super::borrowed::erase_mut_thread_safe;
use super::borrowed::erase_ref_local;
use super::borrowed::erase_ref_thread_safe;
use super::failure::DowncastFailure;
use super::mode::ErasedOwned;
use super::mode::ErasedOwnedThreadSafe;
use super::mode::Local;
use super::mode::LocalOwned;
use super::mode::ThreadSafe;
use super::mode::ThreadSafeOwned;
use super::mode::ValueMode;
use super::mode::ValueModeInternal;
use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::Reflect;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::identity::TypeIdentity;

/// Factory that resolves an erased type into a descriptor for a supplied
/// context.
type DescriptorFactory = for<'ctx> fn(&'ctx ReflectionContext) -> TypeDescriptor<'ctx>;

/// Creates a descriptor for a concrete reflected type in `context`.
///
/// # Parameters
///
/// * `context` — context that owns the returned descriptor.
///
/// # Type Parameters
///
/// * `T` — reflected concrete type to describe.
fn descriptor_for<'ctx, T: Reflect>(context: &'ctx ReflectionContext) -> TypeDescriptor<'ctx> {
    TypeDescriptor::of_in::<T>(context)
}

/// Concrete holder used to preserve the original reflected value after erasure.
///
/// # Type Parameters
///
/// * `T` — concrete reflected value type held by this wrapper.
pub(super) struct OwnedValue<T: Reflect + 'static>(
    /// Concrete value retained behind the erased owner.
    T,
);

impl<T: Reflect + 'static> ErasedOwned for OwnedValue<T> {
    /// Returns the concrete type identifier without inspecting the value.
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
    /// Borrows the concrete value for type checks and shared downcasts.
    fn as_any(&self) -> &dyn Any {
        &self.0
    }
    /// Mutably borrows the concrete value for exclusive downcasts.
    fn as_any_mut(&mut self) -> &mut dyn Any {
        &mut self.0
    }
    /// Erases the holder type while preserving ownership of its concrete value.
    fn into_holder_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
    /// Creates a non-thread-safe shared borrow handle to the contained value.
    fn shared_local(&self) -> std::rc::Rc<dyn super::mode::ErasedRef + '_> {
        erase_ref_local(&self.0)
    }
    /// Creates a non-thread-safe exclusive borrow handle to the contained
    /// value.
    fn mutable_local(&mut self) -> Box<dyn super::mode::ErasedMut + '_> {
        erase_mut_local(&mut self.0)
    }
}

impl<T: Reflect + Send + Sync + 'static> ErasedOwnedThreadSafe for OwnedValue<T> {
    /// Creates a shared borrow handle that can cross threads with this value.
    fn shared_thread_safe(&self) -> std::sync::Arc<dyn super::mode::ErasedRef + Send + Sync + '_> {
        erase_ref_thread_safe(&self.0)
    }
    /// Creates an exclusive borrow handle that satisfies the thread-safe mode.
    fn mutable_thread_safe(&mut self) -> Box<dyn super::mode::ErasedMutThreadSafe + '_> {
        erase_mut_thread_safe(&mut self.0)
    }
}

/// A concrete value supported by a dynamic value mode.
///
/// Implementations are provided for reflected types. The [`ThreadSafe`] mode
/// additionally requires the type to be `Send + Sync`.
///
/// # Type Parameters
///
/// * `M` — value ownership mode that controls thread-safety constraints.
#[allow(private_bounds)]
pub trait ValueFor<M: ValueMode>: ValueForInternal<M> + Reflect + Sized + 'static {
    // empty
}

/// Internal conversion operations used by the public `ValueFor` contract.
///
/// # Type Parameters
///
/// * `M` — runtime ownership mode used for erased storage.
#[allow(private_bounds)]
pub(crate) trait ValueForInternal<M: ValueMode>: Reflect + Sized + 'static {
    /// Moves the concrete value into the selected erased owner.
    ///
    /// # Parameters
    ///
    /// * `value` — reflected value to erase.
    fn erase(value: Self) -> M::Owned;
    /// Recovers ownership when the erased holder contains this exact type.
    ///
    /// # Parameters
    ///
    /// * `value` — erased owner to inspect and possibly consume.
    fn recover(value: M::Owned) -> Result<Self, M::Owned>;
    /// Returns a shared typed view when the holder contains this type.
    ///
    /// # Parameters
    ///
    /// * `value` — erased owner to inspect without consuming it.
    fn as_ref(value: &M::Owned) -> Option<&Self>;
    /// Returns an exclusive typed view when the holder contains this type.
    ///
    /// # Parameters
    ///
    /// * `value` — erased owner to inspect and mutably borrow.
    fn as_mut(value: &mut M::Owned) -> Option<&mut Self>;
    /// Returns the stable runtime identity recorded for this reflected type.
    fn identity() -> TypeIdentity;
}

impl<T: Reflect + 'static> ValueForInternal<Local> for T {
    /// Stores the value in the local-mode erased holder.
    ///
    /// # Parameters
    ///
    /// * `value` — reflected value to erase.
    fn erase(value: Self) -> LocalOwned {
        LocalOwned(Box::new(OwnedValue(value)))
    }
    /// Recovers this type from a local holder while returning mismatches
    /// intact.
    ///
    /// # Parameters
    ///
    /// * `value` — local erased owner to inspect and possibly consume.
    fn recover(value: LocalOwned) -> Result<Self, LocalOwned> {
        match value.0.as_any().is::<Self>() {
            true => m_local_recover(value),
            false => Err(value),
        }
    }
    /// Downcasts a local holder to a shared reference of this type.
    ///
    /// # Parameters
    ///
    /// * `value` — local erased owner to inspect.
    fn as_ref(value: &LocalOwned) -> Option<&Self> {
        value.0.as_any().downcast_ref()
    }
    /// Downcasts a local holder to a mutable reference of this type.
    ///
    /// # Parameters
    ///
    /// * `value` — local erased owner to inspect mutably.
    fn as_mut(value: &mut LocalOwned) -> Option<&mut Self> {
        value.0.as_any_mut().downcast_mut()
    }
    /// Returns this reflected type's stable identity.
    fn identity() -> TypeIdentity {
        identity::<Self>()
    }
}

impl<T: Reflect + Send + Sync + 'static> ValueForInternal<ThreadSafe> for T {
    /// Stores the thread-safe value in its erased holder.
    ///
    /// # Parameters
    ///
    /// * `value` — reflected value to erase.
    fn erase(value: Self) -> ThreadSafeOwned {
        ThreadSafeOwned(Box::new(OwnedValue(value)))
    }
    /// Recovers this type from a thread-safe holder without losing mismatches.
    ///
    /// # Parameters
    ///
    /// * `value` — thread-safe erased owner to inspect and possibly consume.
    fn recover(value: ThreadSafeOwned) -> Result<Self, ThreadSafeOwned> {
        if !value.0.as_any().is::<Self>() {
            return Err(value);
        }
        match ThreadSafe::owned_into_holder_any(value).downcast::<OwnedValue<Self>>() {
            Ok(value) => {
                let OwnedValue(inner) = *value;
                Ok(inner)
            }
            Err(_) => panic!("erased value changed type after its identity check"),
        }
    }
    /// Downcasts a thread-safe holder to a shared reference of this type.
    ///
    /// # Parameters
    ///
    /// * `value` — thread-safe erased owner to inspect.
    fn as_ref(value: &ThreadSafeOwned) -> Option<&Self> {
        value.0.as_any().downcast_ref()
    }
    /// Downcasts a thread-safe holder to a mutable reference of this type.
    ///
    /// # Parameters
    ///
    /// * `value` — thread-safe erased owner to inspect mutably.
    fn as_mut(value: &mut ThreadSafeOwned) -> Option<&mut Self> {
        value.0.as_any_mut().downcast_mut()
    }
    /// Returns this reflected type's stable identity.
    fn identity() -> TypeIdentity {
        identity::<Self>()
    }
}

impl<T: Reflect + 'static> ValueFor<Local> for T {}
impl<T: Reflect + Send + Sync + 'static> ValueFor<ThreadSafe> for T {}

/// Reconstitutes the concrete value after its local-mode type ID has matched.
///
/// # Type Parameters
///
/// * `T` — reflected value type stored in the local erased holder.
///
/// # Parameters
///
/// * `value` — local erased holder whose concrete value was already
///   type-checked.
fn m_local_recover<T: Reflect + 'static>(value: LocalOwned) -> Result<T, LocalOwned> {
    match Local::owned_into_holder_any(value).downcast::<OwnedValue<T>>() {
        Ok(value) => {
            let OwnedValue(inner) = *value;
            Ok(inner)
        }
        Err(_) => panic!("erased value changed type after its identity check"),
    }
}

/// Captures the runtime type identity shared by all wrappers for `T`.
///
/// # Type Parameters
///
/// * `T` — reflected concrete type whose identity is recorded.
fn identity<T: Reflect + 'static>() -> TypeIdentity {
    TypeIdentity {
        type_id: TypeId::of::<T>(),
        rust_name: std::any::type_name::<T>(),
    }
}

/// An owned reflected value whose concrete type is erased at compile time.
///
/// The default [`Local`] mode permits values that are not `Send` or `Sync`.
/// Use [`ThreadSafe`] when the value must be shareable across threads; its
/// concrete type must implement `Send + Sync`.
///
/// # Type Parameters
///
/// * `M` — local or thread-safe ownership mode; defaults to [`Local`].
pub struct DynamicOwned<M: ValueMode = Local> {
    /// Erased allocation holding the concrete value.
    value: M::Owned,
    /// Type identity captured when the value was erased.
    identity: TypeIdentity,
    /// Context-aware descriptor constructor for the concrete type.
    descriptor_factory: DescriptorFactory,
    /// Associates this owner with its mode without storing a mode value.
    _mode: PhantomData<M>,
}

impl<M: ValueMode> DynamicOwned<M> {
    /// Stores `value` behind the type-erased representation for `M`.
    ///
    /// `T` must implement [`Reflect`] and be `'static`; the [`ThreadSafe`]
    /// mode also requires `Send + Sync`.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected concrete value type to store.
    ///
    /// # Parameters
    ///
    /// * `value` — concrete value transferred into this dynamic owner.
    pub fn new<T: ValueFor<M>>(value: T) -> Self {
        let descriptor_factory = descriptor_for::<T>;
        Self {
            value: T::erase(value),
            identity: T::identity(),
            descriptor_factory,
            _mode: PhantomData,
        }
    }

    /// Returns the [`TypeId`] of the stored concrete type.
    pub fn type_id(&self) -> TypeId {
        self.identity.type_id
    }

    /// Returns the stored concrete value as `Any` for read-only downcasting.
    pub fn as_any(&self) -> &dyn Any {
        M::owned_as_any(&self.value)
    }

    /// Describes the stored concrete type in the global reflection context.
    pub fn ty(&self) -> TypeDescriptor<'static> {
        self.ty_in(ReflectionContext::global())
    }

    /// Describes the stored concrete type in `context`.
    ///
    /// The returned descriptor is tied to the lifetime of `context`.
    ///
    /// # Parameters
    ///
    /// * `context` — context used to resolve the concrete type metadata.
    pub fn ty_in<'ctx>(&self, context: &'ctx ReflectionContext) -> TypeDescriptor<'ctx> {
        (self.descriptor_factory)(context)
    }

    /// Returns whether the stored value has the same concrete type as `T`.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected type to compare with the stored value.
    pub fn is<T: ValueFor<M>>(&self) -> bool {
        self.type_id() == T::identity().type_id
    }

    /// Borrows the value as `T` when its concrete type matches.
    ///
    /// Returns `None` when the stored value has a different type.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected concrete type to borrow.
    pub fn downcast_ref<T: ValueFor<M>>(&self) -> Option<&T> {
        T::as_ref(&self.value)
    }

    /// Mutably borrows the value as `T` when its concrete type matches.
    ///
    /// Returns `None` when the stored value has a different type.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected concrete type to borrow mutably.
    pub fn downcast_mut<T: ValueFor<M>>(&mut self) -> Option<&mut T> {
        T::as_mut(&mut self.value)
    }

    /// Attempts to recover the stored value as `T`.
    ///
    /// On a type mismatch, the error retains ownership of this dynamic value
    /// and reports the expected and actual type IDs.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected concrete type to recover.
    pub fn downcast<T: ValueFor<M>>(self) -> Result<T, DowncastFailure<Self>> {
        let expected = T::identity();
        if self.identity.type_id != expected.type_id {
            return Err(DowncastFailure::new(expected, self.identity, self));
        }
        match T::recover(self.value) {
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

    /// Creates a shared dynamically typed view of this value.
    ///
    /// The view borrows this owner and cannot outlive it.
    pub fn as_ref(&self) -> DynamicRef<'_, M> {
        DynamicRef::from_erased(
            M::shared_from_owned(&self.value),
            None,
            self.identity,
            self.descriptor_factory,
        )
    }

    /// Creates a mutable dynamically typed view of this value.
    ///
    /// The view exclusively borrows this owner and cannot outlive it.
    pub fn as_mut(&mut self) -> DynamicMut<'_, M> {
        DynamicMut::from_erased(
            M::mutable_from_owned(&mut self.value),
            self.identity,
            self.descriptor_factory,
        )
    }
}

impl DynamicOwned<ThreadSafe> {
    /// Converts the owner to local mode without cloning its erased allocation.
    pub(crate) fn into_local(self) -> DynamicOwned<Local> {
        let ThreadSafeOwned(value) = self.value;
        let value: Box<dyn ErasedOwned> = value;
        DynamicOwned {
            value: LocalOwned(value),
            identity: self.identity,
            descriptor_factory: self.descriptor_factory,
            _mode: PhantomData,
        }
    }
}
