// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! AssociatedConstBindingDescriptor metadata and behavior.

use crate::descriptor::AssociatedConstDescriptor;
use crate::descriptor::AssociatedConstImplementationSource;
use crate::descriptor::AssociatedConstReadUnavailableReason;
use crate::descriptor::AssociatedConstReader;
use crate::value::ReflectedOwned;

/// One associated constant binding contributed by a concrete impl.
///
/// This descriptor is constructed by generated registration code.
///
/// # Examples
///
/// ```
/// use std::sync::LazyLock;
/// use qubit_reflect::descriptor::{AssociatedConstBindingDescriptor, AssociatedConstDescriptor, AssociatedConstImplementationSource};
/// use qubit_reflect::expression::{ConcreteTypeExpression, TypeExpression};
///
/// static DECLARATION: LazyLock<AssociatedConstDescriptor> = LazyLock::new(|| {
///     AssociatedConstDescriptor::new(
///         0,
///         "LIMIT",
///         "limit",
///         TypeExpression::Concrete(ConcreteTypeExpression::new(["usize"], []).expect("non-empty path")),
///         false,
///     )
/// });
/// let binding = AssociatedConstBindingDescriptor::new(
///     &DECLARATION,
///     AssociatedConstImplementationSource::Overridden,
///     None,
/// );
/// assert!(!binding.is_readable());
/// ```
#[must_use]
#[derive(Clone, Debug)]
pub struct AssociatedConstBindingDescriptor {
    /// Associated constant declaration being implemented.
    declaration: &'static AssociatedConstDescriptor,
    /// Whether the value comes from the default or an override.
    implementation_source: AssociatedConstImplementationSource,
    /// Safe owned-value reader, when available.
    reader: Option<&'static AssociatedConstReader>,
    /// Reason no reader can be provided.
    read_unavailable_reason: Option<AssociatedConstReadUnavailableReason>,
}

impl AssociatedConstBindingDescriptor {
    /// Creates associated constant binding facts.
    ///
    /// # Parameters
    ///
    /// - `declaration`: Associated constant declaration being bound.
    /// - `implementation_source`: Whether the implementation is defaulted or
    ///   overridden.
    /// - `reader`: Safe owned-value reader, when the value type permits one.
    ///
    /// # Returns
    ///
    /// Returns the associated constant binding facts.
    #[doc(hidden)]
    #[must_use = "the associated constant binding facts are required by generated registration"]
    pub const fn new(
        declaration: &'static AssociatedConstDescriptor,
        implementation_source: AssociatedConstImplementationSource,
        reader: Option<&'static AssociatedConstReader>,
    ) -> Self {
        let read_unavailable_reason = match reader {
            Some(_) => None,
            None => Some(AssociatedConstReadUnavailableReason::UnprovenOwnedValue),
        };
        Self {
            declaration,
            implementation_source,
            reader,
            read_unavailable_reason,
        }
    }

    /// Returns the trait declaration being implemented.
    ///
    /// # Returns
    ///
    /// Returns the associated constant declaration.
    #[must_use]
    #[inline]
    pub const fn declaration(&self) -> &'static AssociatedConstDescriptor {
        self.declaration
    }

    /// Returns whether the value is defaulted or explicitly overridden.
    ///
    /// # Returns
    ///
    /// Returns the implementation source.
    #[must_use]
    #[inline]
    pub const fn implementation_source(&self) -> AssociatedConstImplementationSource {
        self.implementation_source
    }

    /// Returns whether a safe owned-value reader is available.
    ///
    /// # Returns
    ///
    /// Returns `true` when [`Self::read`] can return a value.
    #[must_use]
    #[inline]
    pub const fn is_readable(&self) -> bool {
        self.reader.is_some()
    }

    /// Returns the structured reason why no safe reader is available.
    ///
    /// `None` means [`Self::read`] can produce a fresh owned value.
    ///
    /// # Returns
    ///
    /// Returns the reason reading is unavailable, or `None` when it is
    /// available.
    #[must_use]
    #[inline]
    pub const fn read_unavailable_reason(&self) -> Option<AssociatedConstReadUnavailableReason> {
        self.read_unavailable_reason
    }

    /// Reads the associated constant through its safe adapter.
    ///
    /// `None` means the declared type cannot cross the owned dynamic boundary.
    ///
    /// # Returns
    ///
    /// Returns a fresh owned reflected value, or `None` when no safe reader
    /// exists.
    #[must_use]
    pub fn read(&self) -> Option<ReflectedOwned> {
        self.reader.map(AssociatedConstReader::read)
    }
}
