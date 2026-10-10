// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Positional and named invocation argument bindings.

use std::any::TypeId;

use crate::invoke::InvocationArg;
use crate::invoke::InvocationInputMode;
use crate::invoke::InvocationMode;

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
