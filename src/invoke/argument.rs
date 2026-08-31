// qubit-style: allow public-type-layout
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

/// One caller-ordered positional or named invocation argument binding.
///
/// Named and positional bindings may be interleaved. During descriptor-aware
/// binding, a positional input selects the first declaration-order parameter
/// that has not already been occupied by an earlier binding. A named input
/// selects the unique identifier parameter with that name. Binding validation
/// never extracts an owned dynamic value.
pub struct InvocationBinding<'call, M: InvocationMode> {
    name: Option<Box<str>>,
    argument: InvocationArg<'call, M>,
}

impl<'call, M: InvocationMode> InvocationBinding<'call, M> {
    /// Creates a positional binding.
    ///
    /// The binding selects the next unoccupied declaration-order parameter
    /// when a method descriptor validates the invocation.
    pub fn positional(argument: InvocationArg<'call, M>) -> Self {
        Self { name: None, argument }
    }

    /// Creates a named binding without validating the supplied name.
    ///
    /// Validation accepts the name only when exactly one simple identifier
    /// parameter has that name. Unknown, ambiguous, unavailable, duplicate,
    /// and missing bindings produce a structured pre-execution error.
    pub fn named<N>(name: N, argument: InvocationArg<'call, M>) -> Self
    where
        N: Into<Box<str>>,
    {
        Self {
            name: Some(name.into()),
            argument,
        }
    }

    /// Returns the caller-supplied name, or `None` for a positional binding.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the bound dynamic argument without consuming it.
    pub const fn argument(&self) -> &InvocationArg<'call, M> {
        &self.argument
    }

    /// Splits the binding into its optional name and untouched argument.
    pub(crate) fn into_parts(self) -> (Option<Box<str>>, InvocationArg<'call, M>) {
        (self.name, self.argument)
    }
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
