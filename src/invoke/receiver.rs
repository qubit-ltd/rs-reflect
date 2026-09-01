// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Invocation receiver values and receiver expectations.

use std::any::TypeId;

use crate::capability::CapabilityKey;
use crate::invoke::InvocationInputMode;
use crate::invoke::InvocationMode;
use crate::value::DynamicMut;
use crate::value::DynamicOwned;
use crate::value::DynamicRef;

/// A receiver supplied to a reflected method invocation.
///
/// Owned receiver containers such as `Box<T>` and `Pin<Box<T>>` retain their
/// exact container type in [`Self::Owned`]. `Pin<&T>` and `Pin<&mut T>` must
/// not be lowered to the ordinary borrowed variants because doing so would
/// discard their pin proof. Generated invocation support therefore uses the
/// typed pinned invocation contracts for supported pinned borrows; arbitrary
/// receiver conversions still require a separately registered safe adapter.
pub enum InvocationReceiver<'call, M: InvocationMode> {
    /// A receiver consumed as `self` or an owned receiver container.
    Owned(DynamicOwned<M>),
    /// A receiver supplied through a shared borrow.
    Ref(DynamicRef<'call, M>),
    /// A receiver supplied through an exclusive mutable borrow.
    Mut(DynamicMut<'call, M>),
}

impl<M: InvocationMode> InvocationReceiver<'_, M> {
    /// Returns the receiver's ownership or borrowing mode.
    #[must_use]
    #[inline(always)]
    pub const fn mode(&self) -> InvocationInputMode {
        match self {
            Self::Owned(_) => InvocationInputMode::Owned,
            Self::Ref(_) => InvocationInputMode::Ref,
            Self::Mut(_) => InvocationInputMode::Mut,
        }
    }

    /// Returns the exact process-local Rust type identity of the receiver.
    #[must_use]
    #[inline(always)]
    pub fn type_id(&self) -> TypeId {
        match self {
            Self::Owned(value) => M::owned_type_id(value),
            Self::Ref(value) => M::ref_type_id(value),
            Self::Mut(value) => M::mut_type_id(value),
        }
    }
}

/// A typed, safe conversion from one caller-supplied receiver container into
/// an explicit method receiver type.
///
/// The adapter must return the original receiver unchanged when conversion is
/// not possible. This lets generated invocation code preserve the complete
/// input in [`crate::invoke::InvocationRecovery`] rather than losing an owned
/// container during a failed conversion.
pub type ReceiverAdapter<R, M> = for<'call> fn(InvocationReceiver<'call, M>) -> Result<R, InvocationReceiver<'call, M>>;

/// Returns the reserved typed capability key for an explicit receiver type.
///
/// Register this key against the reflected target type with
/// [`crate::register_type_capabilities`] to opt an arbitrary self type into
/// safe dynamic invocation.
pub fn receiver_adapter_key<R: 'static, M: InvocationMode>() -> CapabilityKey<ReceiverAdapter<R, M>> {
    CapabilityKey::new_core("qubit.reflect.receiver_adapter")
}

/// The receiver shape and exact type required by a method adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiverExpectation {
    /// The method is an associated function and accepts no receiver.
    None,
    /// The method consumes an exact receiver value or container.
    Owned {
        /// Expected process-local Rust type identity.
        type_id: TypeId,
        /// Expected Rust type name used for diagnostics.
        type_name: &'static str,
    },
    /// The method reads an exact receiver through a shared borrow.
    Ref {
        /// Expected process-local Rust type identity.
        type_id: TypeId,
        /// Expected Rust type name used for diagnostics.
        type_name: &'static str,
    },
    /// The method mutates an exact receiver through an exclusive borrow.
    Mut {
        /// Expected process-local Rust type identity.
        type_id: TypeId,
        /// Expected Rust type name used for diagnostics.
        type_name: &'static str,
    },
}

impl ReceiverExpectation {
    /// Creates an expectation for an associated function without a receiver.
    pub const fn none() -> Self {
        Self::None
    }

    /// Creates an expectation for an owned receiver of exact type `T`.
    pub fn owned<T: ?Sized + 'static>() -> Self {
        Self::Owned {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
        }
    }

    /// Creates an expectation for a shared receiver of exact type `T`.
    pub fn borrowed<T: ?Sized + 'static>() -> Self {
        Self::Ref {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
        }
    }

    /// Creates an expectation for a mutable receiver of exact type `T`.
    pub fn borrowed_mut<T: ?Sized + 'static>() -> Self {
        Self::Mut {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
        }
    }

    /// Returns the required receiver mode, or `None` for an associated
    /// function.
    pub const fn mode(self) -> Option<InvocationInputMode> {
        match self {
            Self::None => None,
            Self::Owned { .. } => Some(InvocationInputMode::Owned),
            Self::Ref { .. } => Some(InvocationInputMode::Ref),
            Self::Mut { .. } => Some(InvocationInputMode::Mut),
        }
    }

    /// Returns the expected receiver type identity when a receiver is required.
    pub const fn type_id(self) -> Option<TypeId> {
        match self {
            Self::None => None,
            Self::Owned { type_id, .. } | Self::Ref { type_id, .. } | Self::Mut { type_id, .. } => Some(type_id),
        }
    }

    /// Returns the expected receiver type name when a receiver is required.
    pub const fn type_name(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Owned { type_name, .. } | Self::Ref { type_name, .. } | Self::Mut { type_name, .. } => {
                Some(type_name)
            }
        }
    }
}
