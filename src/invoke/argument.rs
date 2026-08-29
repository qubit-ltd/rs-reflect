//! Positional argument values and their validation expectations.

use std::any::TypeId;

use crate::invoke::InvocationMode;
use crate::value::DynamicMut;
use crate::value::DynamicOwned;
use crate::value::DynamicRef;

/// The ownership or borrowing mode of an invocation input.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InvocationInputMode {
    /// The input is consumed by the invoked method.
    Owned,
    /// The method receives a shared borrow.
    Ref,
    /// The method receives an exclusive mutable borrow.
    Mut,
}

/// One non-receiver positional invocation argument.
pub enum InvocationArg<'call, M: InvocationMode> {
    /// An owned argument that may be consumed only after validation succeeds.
    Owned(DynamicOwned<M>),
    /// A shared borrowed argument.
    Ref(DynamicRef<'call, M>),
    /// A mutable borrowed argument.
    Mut(DynamicMut<'call, M>),
}

impl<M: InvocationMode> InvocationArg<'_, M> {
    /// Returns the input's ownership or borrowing mode.
    pub const fn mode(&self) -> InvocationInputMode {
        match self {
            Self::Owned(_) => InvocationInputMode::Owned,
            Self::Ref(_) => InvocationInputMode::Ref,
            Self::Mut(_) => InvocationInputMode::Mut,
        }
    }

    /// Returns the exact process-local Rust type identity of the input.
    pub fn type_id(&self) -> TypeId {
        match self {
            Self::Owned(value) => M::owned_type_id(value),
            Self::Ref(value) => M::ref_type_id(value),
            Self::Mut(value) => M::mut_type_id(value),
        }
    }
}

/// The exact type and passing mode expected for one positional argument.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArgumentExpectation {
    mode: InvocationInputMode,
    type_id: TypeId,
    type_name: &'static str,
}

impl ArgumentExpectation {
    /// Creates an expectation for an owned `T` argument.
    pub fn owned<T: ?Sized + 'static>() -> Self {
        Self::new::<T>(InvocationInputMode::Owned)
    }

    /// Creates an expectation for a shared `T` argument.
    pub fn borrowed<T: ?Sized + 'static>() -> Self {
        Self::new::<T>(InvocationInputMode::Ref)
    }

    /// Creates an expectation for a mutable `T` argument.
    pub fn borrowed_mut<T: ?Sized + 'static>() -> Self {
        Self::new::<T>(InvocationInputMode::Mut)
    }

    /// Returns the required argument mode.
    pub const fn mode(self) -> InvocationInputMode {
        self.mode
    }

    /// Returns the exact expected process-local Rust type identity.
    pub const fn type_id(self) -> TypeId {
        self.type_id
    }

    /// Returns the expected Rust type name for diagnostics.
    pub const fn type_name(self) -> &'static str {
        self.type_name
    }

    /// Creates one exact argument expectation for `T`.
    fn new<T: ?Sized + 'static>(mode: InvocationInputMode) -> Self {
        Self {
            mode,
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
        }
    }
}
