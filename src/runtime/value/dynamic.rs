// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;

use super::borrowed::DynamicMut;
use super::borrowed::DynamicRef;
use super::mode::ValueMode;
use super::owned::DynamicOwned;
use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::TypeDescriptor;

/// Describes the ownership form held by a [`DynamicValue`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DynamicValueKind {
    /// The value owns its reflected value.
    Owned,
    /// The value holds a shared borrow.
    Ref,
    /// The value holds an exclusive borrow.
    Mut,
}

/// A tagged dynamic value that retains whether it is owned or borrowed.
pub enum DynamicValue<'a, M: ValueMode = super::mode::Local> {
    /// An owned reflected value.
    Owned(DynamicOwned<M>),
    /// A shared borrow of a reflected value.
    Ref(DynamicRef<'a, M>),
    /// An exclusive borrow of a reflected value.
    Mut(DynamicMut<'a, M>),
}

impl<'a, M: ValueMode> DynamicValue<'a, M> {
    /// Returns whether this value is owned, shared, or exclusively borrowed.
    pub fn kind(&self) -> DynamicValueKind {
        match self {
            Self::Owned(_) => DynamicValueKind::Owned,
            Self::Ref(_) => DynamicValueKind::Ref,
            Self::Mut(_) => DynamicValueKind::Mut,
        }
    }

    /// Returns the exact Rust type identity represented by this value.
    pub fn type_id(&self) -> TypeId {
        match self {
            Self::Owned(value) => value.type_id(),
            Self::Ref(value) => value.clone().type_id(),
            Self::Mut(value) => value.type_id(),
        }
    }

    /// Describes the represented type using the global reflection context.
    pub fn ty(&self) -> TypeDescriptor<'static> {
        self.ty_in(ReflectionContext::global())
    }

    /// Describes the represented type using `context`.
    pub fn ty_in<'ctx>(&self, context: &'ctx ReflectionContext) -> TypeDescriptor<'ctx> {
        match self {
            Self::Owned(value) => value.ty_in(context),
            Self::Ref(value) => value.clone().ty_in(context),
            Self::Mut(value) => value.ty_in(context),
        }
    }

    /// Creates a shared view whose lifetime is limited by this borrow.
    pub fn as_ref(&self) -> DynamicRef<'_, M> {
        match self {
            Self::Owned(value) => value.as_ref(),
            Self::Ref(value) => value.reborrow(),
            Self::Mut(value) => value.as_ref(),
        }
    }

    /// Recovers an owned value of type `T`.
    ///
    /// Returns the original tagged value when it is borrowed or its concrete
    /// type differs from `T`.
    pub fn into_owned<T: super::owned::ValueFor<M>>(self) -> Result<T, Self> {
        match self {
            Self::Owned(value) => match value.downcast::<T>() {
                Ok(value) => Ok(value),
                Err(failure) => Err(Self::Owned(failure.into_value())),
            },
            value => Err(value),
        }
    }

    /// Recovers a shared borrow of type `T`.
    ///
    /// Returns the original tagged value when it is not shared or its
    /// concrete type differs from `T`.
    pub fn into_ref<T: super::borrowed::ValueForRef<M> + ?Sized>(self) -> Result<&'a T, Self> {
        match self {
            Self::Ref(value) => match value.downcast::<T>() {
                Ok(value) => Ok(value),
                Err(failure) => Err(Self::Ref(failure.into_value())),
            },
            value => Err(value),
        }
    }

    /// Recovers an exclusive borrow of type `T`.
    ///
    /// Returns the original tagged value when it is not mutable or its
    /// concrete type differs from `T`.
    pub fn into_mut<T: super::borrowed::ValueForMut<M> + ?Sized>(self) -> Result<&'a mut T, Self> {
        match self {
            Self::Mut(value) => match value.downcast::<T>() {
                Ok(value) => Ok(value),
                Err(failure) => Err(Self::Mut(failure.into_value())),
            },
            value => Err(value),
        }
    }
}
