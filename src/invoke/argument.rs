// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Positional argument values and their validation expectations.

use std::any::TypeId;

use crate::invoke::InvocationMode;
use crate::value::DynamicMut;
use crate::value::DynamicOwned;
use crate::value::DynamicRef;

/// The ownership or borrowing mode of an invocation input.
///
/// # Examples
///
/// ```
/// use qubit_reflect::invoke::InvocationInputMode;
///
/// assert_eq!(InvocationInputMode::Owned, InvocationInputMode::Owned);
/// ```
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
///
/// # Examples
///
/// ```
/// use qubit_reflect::invoke::{InvocationArg, InvocationInputMode};
/// use qubit_reflect::value::{DynamicOwned, Local};
///
/// let argument = InvocationArg::<Local>::Owned(DynamicOwned::<Local>::new(7_u8));
/// assert_eq!(argument.mode(), InvocationInputMode::Owned);
/// ```
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
///
/// # Examples
///
/// ```
/// use qubit_reflect::invoke::{InvocationArg, InvocationBinding};
/// use qubit_reflect::value::{DynamicOwned, Local};
///
/// let binding = InvocationBinding::named(
///     "count",
///     InvocationArg::Owned(DynamicOwned::<Local>::new(3_u32)),
/// );
/// assert_eq!(binding.name(), Some("count"));
/// ```
pub struct InvocationBinding<'call, M: InvocationMode> {
    /// Caller-supplied name, absent for a positional binding.
    name: Option<Box<str>>,
    /// Dynamic value and its ownership or borrowing mode.
    argument: InvocationArg<'call, M>,
}

impl<'call, M: InvocationMode> InvocationBinding<'call, M> {
    /// Creates a positional binding.
    ///
    /// The binding selects the next unoccupied declaration-order parameter
    /// when a method descriptor validates the invocation.
    ///
    /// # Parameters
    ///
    /// - `argument`: Dynamic argument to bind positionally.
    ///
    /// # Returns
    ///
    /// Returns an unnamed positional binding.
    #[must_use]
    pub fn positional(argument: InvocationArg<'call, M>) -> Self {
        Self { name: None, argument }
    }

    /// Creates a named binding without validating the supplied name.
    ///
    /// Validation accepts the name only when exactly one simple identifier
    /// parameter has that name. Unknown, ambiguous, unavailable, duplicate,
    /// and missing bindings produce a structured pre-execution error.
    ///
    /// # Type Parameters
    ///
    /// - `N`: Name value convertible to owned text.
    ///
    /// # Parameters
    ///
    /// - `name`: Caller-supplied parameter name.
    /// - `argument`: Dynamic argument to bind by name.
    ///
    /// # Returns
    ///
    /// Returns a named binding; descriptor-aware validation checks the name.
    #[must_use]
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
    ///
    /// # Returns
    ///
    /// Returns the name, or `None` for a positional binding.
    #[must_use]
    #[inline]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the bound dynamic argument without consuming it.
    ///
    /// # Returns
    ///
    /// Returns a shared reference to the bound argument.
    #[must_use]
    #[inline]
    pub const fn argument(&self) -> &InvocationArg<'call, M> {
        &self.argument
    }

    /// Splits the binding into its optional name and untouched argument.
    ///
    /// # Returns
    ///
    /// Returns the name and original dynamic argument.
    #[must_use]
    pub(crate) fn into_parts(self) -> (Option<Box<str>>, InvocationArg<'call, M>) {
        (self.name, self.argument)
    }
}

impl<M: InvocationMode> InvocationArg<'_, M> {
    /// Returns the input's ownership or borrowing mode.
    ///
    /// # Returns
    ///
    /// Returns `Owned`, `Ref`, or `Mut` for this input.
    #[must_use]
    #[inline]
    pub const fn mode(&self) -> InvocationInputMode {
        match self {
            Self::Owned(_) => InvocationInputMode::Owned,
            Self::Ref(_) => InvocationInputMode::Ref,
            Self::Mut(_) => InvocationInputMode::Mut,
        }
    }

    /// Returns the exact process-local Rust type identity of the input.
    ///
    /// # Returns
    ///
    /// Returns the input's exact `TypeId`.
    #[must_use]
    #[inline]
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
    /// Required ownership or borrowing mode.
    mode: InvocationInputMode,
    /// Exact process-local type identity required by the adapter.
    type_id: TypeId,
    /// Rust type name retained for diagnostics.
    type_name: &'static str,
}

impl ArgumentExpectation {
    /// Creates an expectation for an owned `T` argument.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Exact owned argument type.
    ///
    /// # Returns
    ///
    /// Returns an owned argument expectation for `T`.
    #[must_use]
    pub fn owned<T: ?Sized + 'static>() -> Self {
        Self::new::<T>(InvocationInputMode::Owned)
    }

    /// Creates an expectation for a shared `T` argument.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Exact shared-borrow argument type.
    ///
    /// # Returns
    ///
    /// Returns a shared-borrow expectation for `T`.
    #[must_use]
    pub fn borrowed<T: ?Sized + 'static>() -> Self {
        Self::new::<T>(InvocationInputMode::Ref)
    }

    /// Creates an expectation for a mutable `T` argument.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Exact mutable-borrow argument type.
    ///
    /// # Returns
    ///
    /// Returns a mutable-borrow expectation for `T`.
    #[must_use]
    pub fn borrowed_mut<T: ?Sized + 'static>() -> Self {
        Self::new::<T>(InvocationInputMode::Mut)
    }

    /// Returns the required argument mode.
    ///
    /// # Returns
    ///
    /// Returns the required ownership or borrowing mode.
    #[must_use]
    #[inline]
    pub const fn mode(self) -> InvocationInputMode {
        self.mode
    }

    /// Returns the exact expected process-local Rust type identity.
    ///
    /// # Returns
    ///
    /// Returns the exact expected `TypeId`.
    #[must_use]
    #[inline]
    pub const fn type_id(self) -> TypeId {
        self.type_id
    }

    /// Returns the expected Rust type name for diagnostics.
    ///
    /// # Returns
    ///
    /// Returns the compiler-provided type name of the expected value.
    #[must_use]
    #[inline]
    pub const fn type_name(self) -> &'static str {
        self.type_name
    }

    /// Creates one exact argument expectation for `T`.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Exact argument type.
    ///
    /// # Parameters
    ///
    /// - `mode`: Required ownership or borrowing mode.
    ///
    /// # Returns
    ///
    /// Returns an expectation for `T` with the supplied mode.
    fn new<T: ?Sized + 'static>(mode: InvocationInputMode) -> Self {
        Self {
            mode,
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
        }
    }
}
