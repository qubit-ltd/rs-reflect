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

type DescriptorFactory = for<'ctx> fn(&'ctx ReflectionContext) -> TypeDescriptor<'ctx>;

fn descriptor_for<'ctx, T: Reflect>(context: &'ctx ReflectionContext) -> TypeDescriptor<'ctx> {
    TypeDescriptor::of_in::<T>(context)
}

struct OwnedValue<T: Reflect + 'static>(T);

impl<T: Reflect + 'static> ErasedOwned for OwnedValue<T> {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
    fn as_any(&self) -> &dyn Any {
        &self.0
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        &mut self.0
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        let OwnedValue(value) = *self;
        Box::new(value)
    }
    fn shared_local(&self) -> std::rc::Rc<dyn super::mode::ErasedRef + '_> {
        erase_ref_local(&self.0)
    }
    fn mutable_local(&mut self) -> Box<dyn super::mode::ErasedMut + '_> {
        erase_mut_local(&mut self.0)
    }
}

impl<T: Reflect + Send + Sync + 'static> ErasedOwnedThreadSafe for OwnedValue<T> {
    fn shared_thread_safe(&self) -> std::sync::Arc<dyn super::mode::ErasedRef + Send + Sync + '_> {
        erase_ref_thread_safe(&self.0)
    }
    fn mutable_thread_safe(&mut self) -> Box<dyn super::mode::ErasedMutThreadSafe + '_> {
        erase_mut_thread_safe(&mut self.0)
    }
}

#[allow(private_bounds)]
/// A concrete value supported by a dynamic value mode.
///
/// Implementations are provided for reflected types. The [`ThreadSafe`] mode
/// additionally requires the type to be `Send + Sync`.
pub trait ValueFor<M: ValueMode>: ValueForInternal<M> + Reflect + Sized + 'static {}

#[allow(private_bounds)]
pub(crate) trait ValueForInternal<M: ValueMode>: Reflect + Sized + 'static {
    fn erase(value: Self) -> M::Owned;
    fn recover(value: M::Owned) -> Result<Self, M::Owned>;
    fn as_ref(value: &M::Owned) -> Option<&Self>;
    fn as_mut(value: &mut M::Owned) -> Option<&mut Self>;
    fn identity() -> TypeIdentity;
}

impl<T: Reflect + 'static> ValueForInternal<Local> for T {
    fn erase(value: Self) -> LocalOwned {
        LocalOwned(Box::new(OwnedValue(value)))
    }
    fn recover(value: LocalOwned) -> Result<Self, LocalOwned> {
        match value.0.as_any().is::<Self>() {
            true => m_local_recover(value),
            false => Err(value),
        }
    }
    fn as_ref(value: &LocalOwned) -> Option<&Self> {
        value.0.as_any().downcast_ref()
    }
    fn as_mut(value: &mut LocalOwned) -> Option<&mut Self> {
        value.0.as_any_mut().downcast_mut()
    }
    fn identity() -> TypeIdentity {
        identity::<Self>()
    }
}

impl<T: Reflect + Send + Sync + 'static> ValueForInternal<ThreadSafe> for T {
    fn erase(value: Self) -> ThreadSafeOwned {
        ThreadSafeOwned(Box::new(OwnedValue(value)))
    }
    fn recover(value: ThreadSafeOwned) -> Result<Self, ThreadSafeOwned> {
        if !value.0.as_any().is::<Self>() {
            return Err(value);
        }
        match ThreadSafe::owned_into_any(value).downcast::<Self>() {
            Ok(value) => Ok(*value),
            Err(_) => panic!("erased value changed type after its identity check"),
        }
    }
    fn as_ref(value: &ThreadSafeOwned) -> Option<&Self> {
        value.0.as_any().downcast_ref()
    }
    fn as_mut(value: &mut ThreadSafeOwned) -> Option<&mut Self> {
        value.0.as_any_mut().downcast_mut()
    }
    fn identity() -> TypeIdentity {
        identity::<Self>()
    }
}

impl<T: Reflect + 'static> ValueFor<Local> for T {}
impl<T: Reflect + Send + Sync + 'static> ValueFor<ThreadSafe> for T {}

fn m_local_recover<T: Reflect + 'static>(value: LocalOwned) -> Result<T, LocalOwned> {
    match Local::owned_into_any(value).downcast::<T>() {
        Ok(value) => Ok(*value),
        Err(_) => panic!("erased value changed type after its identity check"),
    }
}

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
pub struct DynamicOwned<M: ValueMode = Local> {
    value: M::Owned,
    identity: TypeIdentity,
    descriptor_factory: DescriptorFactory,
    _mode: PhantomData<M>,
}

impl<M: ValueMode> DynamicOwned<M> {
    /// Stores `value` behind the type-erased representation for `M`.
    ///
    /// `T` must implement [`Reflect`] and be `'static`; the [`ThreadSafe`]
    /// mode also requires `Send + Sync`.
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

    /// Describes the stored concrete type in the global reflection context.
    pub fn ty(&self) -> TypeDescriptor<'static> {
        self.ty_in(ReflectionContext::global())
    }

    /// Describes the stored concrete type in `context`.
    ///
    /// The returned descriptor is tied to the lifetime of `context`.
    pub fn ty_in<'ctx>(&self, context: &'ctx ReflectionContext) -> TypeDescriptor<'ctx> {
        (self.descriptor_factory)(context)
    }

    /// Returns whether the stored value has the same concrete type as `T`.
    pub fn is<T: ValueFor<M>>(&self) -> bool {
        self.type_id() == T::identity().type_id
    }

    /// Borrows the value as `T` when its concrete type matches.
    ///
    /// Returns `None` when the stored value has a different type.
    pub fn downcast_ref<T: ValueFor<M>>(&self) -> Option<&T> {
        T::as_ref(&self.value)
    }

    /// Mutably borrows the value as `T` when its concrete type matches.
    ///
    /// Returns `None` when the stored value has a different type.
    pub fn downcast_mut<T: ValueFor<M>>(&mut self) -> Option<&mut T> {
        T::as_mut(&mut self.value)
    }

    /// Attempts to recover the stored value as `T`.
    ///
    /// On a type mismatch, the error retains ownership of this dynamic value
    /// and reports the expected and actual type IDs.
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
